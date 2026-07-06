// Phase 6 — Google Calendar API v3 client (events only).
// Pure HTTP; takes a token, returns DTOs.

use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};

pub const CAL_BASE: &str = "https://www.googleapis.com/calendar/v3";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GEvent {
    pub id: String,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub start: GDateTime,
    pub end: GDateTime,
    pub updated: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GDateTime {
    #[serde(rename = "dateTime")]
    pub date_time: String,
    #[serde(rename = "timeZone", skip_serializing_if = "Option::is_none", default)]
    pub time_zone: Option<String>,
}

pub async fn list_events(
    http: &reqwest::Client,
    access_token: &str,
    time_min: &str,
    time_max: &str,
) -> AppResult<Vec<GEvent>> {
    let url = format!("{}/calendars/primary/events", CAL_BASE);
    let resp = http.get(&url)
        .bearer_auth(access_token)
        .query(&[
            ("timeMin", time_min),
            ("timeMax", time_max),
            ("singleEvents", "true"),
            ("orderBy", "startTime"),
        ])
        .send().await
        .map_err(|e| AppError::Provider(format!("list events: {e}")))?;
    let body: EventsResponse = resp.error_for_status()
        .map_err(|e| AppError::Provider(format!("list status: {e}")))?
        .json().await
        .map_err(|e| AppError::Provider(format!("list json: {e}")))?;
    Ok(body.items.unwrap_or_default())
}

pub async fn insert_event(
    http: &reqwest::Client,
    access_token: &str,
    summary: &str,
    description: &str,
    start_iso: &str,
    end_iso: &str,
) -> AppResult<GEvent> {
    let url = format!("{}/calendars/primary/events", CAL_BASE);
    let body = serde_json::json!({
        "summary": summary,
        "description": description,
        "start": { "dateTime": start_iso },
        "end":   { "dateTime": end_iso },
    });
    let resp = http.post(&url)
        .bearer_auth(access_token)
        .json(&body)
        .send().await
        .map_err(|e| AppError::Provider(format!("insert: {e}")))?;
    let ev: GEvent = resp.error_for_status()
        .map_err(|e| AppError::Provider(format!("insert status: {e}")))?
        .json().await
        .map_err(|e| AppError::Provider(format!("insert json: {e}")))?;
    Ok(ev)
}

#[derive(Deserialize)]
struct EventsResponse { items: Option<Vec<GEvent>> }

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn gevent_parses_minimal_payload() {
        let body = json!({
            "id": "abc123",
            "summary": "Lecture",
            "start": { "dateTime": "2026-07-06T09:00:00Z" },
            "end":   { "dateTime": "2026-07-06T10:00:00Z" },
            "updated": "2026-07-06T08:00:00Z"
        });
        let ev: GEvent = serde_json::from_value(body).unwrap();
        assert_eq!(ev.id, "abc123");
        assert_eq!(ev.summary.as_deref(), Some("Lecture"));
        assert_eq!(ev.start.date_time, "2026-07-06T09:00:00Z");
    }
}