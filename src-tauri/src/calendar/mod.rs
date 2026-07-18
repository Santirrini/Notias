pub mod google;
use google::GEvent;

use crate::error::{AppError, AppResult};
use crate::oauth::google as oauth;
use crate::secrets;
use crate::AppState;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use tauri::State;

const KEYRING_USER: &str = "google_calendar";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CalendarEvent {
    pub gcal_id: String,
    pub summary: String,
    pub description: Option<String>,
    pub starts_at: String,
    pub ends_at: String,
    pub updated_at: String,
    pub source: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct AuthStatus {
    pub connected: bool,
    pub expires_at: Option<i64>,
}

fn load_token() -> AppResult<Option<oauth::StoredToken>> {
    match secrets::get(KEYRING_USER)? {
        Some(json) => Ok(Some(serde_json::from_str(&json).map_err(|e| AppError::Auth(format!("token json: {e}")))?)),
        None => Ok(None),
    }
}

fn save_token(tok: &oauth::StoredToken) -> AppResult<()> {
    let json = serde_json::to_string(tok).map_err(|e| AppError::Auth(format!("token ser: {e}")))?;
    secrets::set(KEYRING_USER, &json)
}

async fn access_token(http: &reqwest::Client) -> AppResult<String> {
    let tok = load_token()?.ok_or_else(|| AppError::Auth("not connected".into()))?;
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64).unwrap_or(0);
    if tok.expires_at > now + 30 {
        return Ok(tok.access_token);
    }
    if tok.refresh_token.is_empty() {
        return Err(AppError::Auth("token expired and no refresh_token".into()));
    }
    let refreshed = oauth::refresh(http, &tok.refresh_token).await?;
    save_token(&refreshed)?;
    Ok(refreshed.access_token)
}

#[tauri::command]
pub async fn calendar_auth_status() -> AppResult<AuthStatus> {
    Ok(match load_token()? {
        Some(t) => AuthStatus { connected: true, expires_at: Some(t.expires_at) },
        None => AuthStatus { connected: false, expires_at: None },
    })
}

#[tauri::command]
pub async fn calendar_connect(state: State<'_, AppState>) -> AppResult<()> {
    let (code, verifier, redirect_uri) = oauth::authorize(&state.http).await?;
    let tok = oauth::exchange_code(&state.http, &code, &verifier, &redirect_uri).await?;
    save_token(&tok)
}

#[tauri::command]
pub async fn calendar_disconnect() -> AppResult<()> {
    secrets::delete(KEYRING_USER)
}

#[tauri::command]
pub async fn calendar_pull(days: u32, state: State<'_, AppState>) -> AppResult<usize> {
    let token = access_token(&state.http).await?;
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64).unwrap_or(0);
    let time_min = format_iso(now);
    let time_max = format_iso(now + (days as i64) * 86_400);
    let events = google::list_events(&state.http, &token, &time_min, &time_max).await?;
    let conn = state.db_conn()?;
    merge_events(&conn, &events)?;
    let _ = conn.execute(
        "INSERT INTO calendar_sync_state(scope, last_pulled_at, last_error) VALUES('primary', ?1, NULL)
         ON CONFLICT(scope) DO UPDATE SET last_pulled_at=excluded.last_pulled_at, last_error=NULL",
        params![format_iso(now)],
    );
    Ok(events.len())
}

#[tauri::command]
pub async fn calendar_create(summary: String, description: String, start_iso: String, end_iso: String, state: State<'_, AppState>) -> AppResult<CalendarEvent> {
    let token = access_token(&state.http).await?;
    let ev = google::insert_event(&state.http, &token, &summary, &description, &start_iso, &end_iso).await?;
    let local = CalendarEvent {
        gcal_id: ev.id.clone(),
        summary: ev.summary.clone().unwrap_or_default(),
        description: ev.description.clone(),
        starts_at: ev.start.date_time.clone(),
        ends_at: ev.end.date_time.clone(),
        updated_at: ev.updated.clone(),
        source: "google".into(),
    };
    let conn = state.db_conn()?;
    upsert_local(&conn, &local)?;
    Ok(local)
}

#[tauri::command]
pub fn calendar_list(state: State<'_, AppState>) -> AppResult<Vec<CalendarEvent>> {
    let conn = state.db_conn()?;
    list_local(&conn)
}

fn merge_events(conn: &Connection, events: &[GEvent]) -> AppResult<()> {
    let tx = conn.unchecked_transaction()?;
    for ev in events {
        // ponytail: last-write-wins — Google's `updated` overwrites the local row.
        // 3-way merge would need version vectors; deferred until we support local-only edits.
        tx.execute(
            "INSERT INTO calendar_events(gcal_id, summary, description, starts_at, ends_at, updated_at, source)
             VALUES(?1, ?2, ?3, ?4, ?5, ?6, 'google')
             ON CONFLICT(gcal_id) DO UPDATE SET
               summary=excluded.summary,
               description=excluded.description,
               starts_at=excluded.starts_at,
               ends_at=excluded.ends_at,
               updated_at=excluded.updated_at,
               source='google'",
            params![ev.id, ev.summary.clone().unwrap_or_default(), ev.description.clone(),
                    ev.start.date_time, ev.end.date_time, ev.updated],
        )?;
    }
    tx.commit()?;
    Ok(())
}

fn upsert_local(conn: &Connection, ev: &CalendarEvent) -> AppResult<()> {
    conn.execute(
        "INSERT INTO calendar_events(gcal_id, summary, description, starts_at, ends_at, updated_at, source)
         VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT(gcal_id) DO UPDATE SET
           summary=excluded.summary, description=excluded.description,
           starts_at=excluded.starts_at, ends_at=excluded.ends_at,
           updated_at=excluded.updated_at, source=excluded.source",
        params![ev.gcal_id, ev.summary, ev.description, ev.starts_at, ev.ends_at, ev.updated_at, ev.source],
    )?;
    Ok(())
}

fn list_local(conn: &Connection) -> AppResult<Vec<CalendarEvent>> {
    let mut stmt = conn.prepare(
        "SELECT gcal_id, summary, description, starts_at, ends_at, updated_at, source
         FROM calendar_events ORDER BY starts_at ASC LIMIT 200"
    )?;
    let rows = stmt.query_map([], |r| Ok(CalendarEvent {
        gcal_id: r.get(0)?,
        summary: r.get(1)?,
        description: r.get(2)?,
        starts_at: r.get(3)?,
        ends_at: r.get(4)?,
        updated_at: r.get(5)?,
        source: r.get(6)?,
    }))?;
    Ok(rows.filter_map(Result::ok).collect())
}

fn format_iso(unix_secs: i64) -> String {
    use time::format_description::well_known::Rfc3339;
    use time::OffsetDateTime;
    OffsetDateTime::from_unix_timestamp(unix_secs)
        .unwrap_or(OffsetDateTime::UNIX_EPOCH)
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".into())
}