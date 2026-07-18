use crate::ai::provider::Provider;
use crate::error::{AppError, AppResult};
use std::time::Duration;
use tauri::{Emitter, Manager};

pub async fn run(note_id: String, app: tauri::AppHandle) -> AppResult<()> {
    let state = app.state::<crate::AppState>();
    let provider = state.router.local_only().await
        .ok_or_else(|| AppError::Config("ollama not configured".into()))?;
    let embed_model = "nomic-embed-text".to_string();

    let text: String = {
        let conn = state.db_conn()?;
        conn.query_row(
            "SELECT body FROM notes WHERE id = ?1 AND deleted_at IS NULL",
            rusqlite::params![&note_id],
            |r| r.get(0),
        ).map_err(|e| AppError::Config(format!("note read: {e}")))?
    };

    let mut attempt = 0u32;
    let vectors = loop {
        attempt += 1;
        match provider.embed(&[text.clone()], &embed_model).await {
            Ok(v) => break v,
            Err(e) if attempt < 3 => {
                let delay = Duration::from_secs(2u64.pow(attempt));
                tracing::warn!("embed attempt {attempt} failed: {e}, retrying in {delay:?}");
                tokio::time::sleep(delay).await;
            }
            Err(e) => {
                mark_status(&app, &note_id, "failed")?;
                return Err(e);
            }
        }
    };

    {
        let conn = state.db_conn()?;
        write_vec(&conn, &note_id, &vectors[0])?;
        conn.execute(
            "UPDATE notes SET embedding_status = 'ready' WHERE id = ?1",
            rusqlite::params![&note_id],
        )?;
    }

    let _ = app.emit("embedding_updated", serde_json::json!({"note_id": note_id, "status": "ready"}));
    Ok(())
}

fn write_vec(conn: &rusqlite::Connection, note_id: &str, v: &[f32]) -> AppResult<()> {
    let rowid: i64 = conn.query_row(
        "SELECT rowid FROM notes WHERE id = ?1",
        rusqlite::params![note_id],
        |r| r.get(0),
    ).map_err(|e| AppError::Config(format!("note rowid: {e}")))?;
    let json = serde_json::to_string(v).map_err(|e| AppError::Config(format!("vec encode: {e}")))?;
    conn.execute(
        "INSERT INTO note_vec(rowid, embedding) VALUES(?1, vec_f32(?2))
         ON CONFLICT(rowid) DO UPDATE SET embedding=vec_f32(?2)",
        rusqlite::params![rowid, json],
    ).map_err(|e| AppError::Config(format!("vec write: {e}")))?;
    Ok(())
}

fn mark_status(app: &tauri::AppHandle, note_id: &str, status: &str) -> AppResult<()> {
    let state = app.state::<crate::AppState>();
    let conn = state.db_conn()?;
    conn.execute(
        "UPDATE notes SET embedding_status = ?2 WHERE id = ?1",
        rusqlite::params![note_id, status],
    )?;
    let _ = app.emit("embedding_updated", serde_json::json!({"note_id": note_id, "status": status}));
    Ok(())
}