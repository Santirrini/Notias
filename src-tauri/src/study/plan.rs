use crate::error::{AppError, AppResult};
use crate::time_util::time_now;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use ulid::Ulid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PlanBlock {
    pub day: String,
    pub start: String,
    pub minutes: u32,
    pub kind: String,
    pub refs: Vec<String>,
    pub rationale: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Plan {
    pub week_start: String,
    pub daily_hours_cap: u8,
    pub blocks: Vec<PlanBlock>,
}

pub fn validate(p: &Plan) -> Result<(), String> {
    // ponytail: structural invariants; the model still produces *something* useful even if the LLM
    // violates a soft rule (e.g. an over-budget block). Caller decides whether to discard.
    if p.blocks.is_empty() {
        return Err("no blocks".into());
    }
    if p.daily_hours_cap == 0 || p.daily_hours_cap > 16 {
        return Err("daily_hours_cap out of range".into());
    }
    for b in &p.blocks {
        if b.minutes == 0 || b.minutes > 60 * p.daily_hours_cap as u32 {
            return Err(format!("bad minutes {}", b.minutes));
        }
        if !(b.day.starts_with("202") || b.day.starts_with("203")) {
            return Err(format!("bad day {}", b.day));
        }
        if b.start.len() != 5 || b.start.as_bytes()[2] != b':' {
            return Err(format!("bad start {}", b.start));
        }
    }
    Ok(())
}

pub fn save(conn: &rusqlite::Connection, p: &Plan) -> AppResult<String> {
    if let Err(e) = validate(p) {
        return Err(AppError::Invalid(format!("plan: {e}")));
    }
    let id = Ulid::new().to_string();
    let plan_json = serde_json::to_string(p)?;
    conn.execute(
        "INSERT INTO study_sessions (id, kind, started_at, plan_json) VALUES (?1, 'plan', ?2, ?3)",
        params![id, time_now(), plan_json],
    )?;
    Ok(id)
}

pub fn get(conn: &rusqlite::Connection, week_start: &str) -> AppResult<Option<Plan>> {
    let mut stmt = conn.prepare(
        "SELECT plan_json FROM study_sessions \
         WHERE kind='plan' AND json_extract(plan_json, '$.week_start') = ?1 \
         ORDER BY started_at DESC LIMIT 1",
    )?;
    let mut rows = stmt.query(params![week_start])?;
    if let Some(r) = rows.next()? {
        let s: String = r.get(0)?;
        let p: Plan = serde_json::from_str(&s)
            .map_err(|e| AppError::Invalid(format!("plan json: {e}")))?;
        return Ok(Some(p));
    }
    Ok(None)
}

pub fn build_context(
    conn: &rusqlite::Connection,
    week_start: &str,
    daily_hours: u8,
) -> AppResult<String> {
    let mut s = String::new();

    let mut stmt = conn.prepare(
        "SELECT title, due_at FROM tasks WHERE status IN ('todo','doing') \
         AND due_at IS NOT NULL AND substr(due_at, 1, 10) BETWEEN ?1 AND date(?1, '+6 days') \
         ORDER BY due_at ASC LIMIT 20",
    )?;
    let rows = stmt.query_map(params![week_start], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
    })?;
    s.push_str("Tasks due this week:\n");
    let mut any = false;
    for r in rows {
        let (t, d) = r?;
        s.push_str(&format!("- {} (due {})\n", t, d));
        any = true;
    }
    if !any {
        s.push_str("- (none)\n");
    }

    let mut stmt =
        conn.prepare("SELECT title FROM notes WHERE deleted_at IS NULL ORDER BY updated DESC LIMIT 10")?;
    let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
    s.push_str("\nRecent notes:\n");
    let mut any = false;
    for r in rows {
        s.push_str(&format!("- {}\n", r?));
        any = true;
    }
    if !any {
        s.push_str("- (none)\n");
    }

    let backlog: i64 = conn.query_row(
        "SELECT COUNT(*) FROM srs_cards WHERE suspended_at IS NULL",
        [],
        |r| r.get(0),
    )?;
    s.push_str(&format!("\nSRS backlog: {} cards\n", backlog));
    s.push_str(&format!("Daily study cap: {} hours\n", daily_hours));
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use tempfile::tempdir;

    fn bp() -> PlanBlock {
        PlanBlock {
            day: "2026-07-06".into(),
            start: "09:00".into(),
            minutes: 60,
            kind: "review".into(),
            refs: vec![],
            rationale: "x".into(),
        }
    }

    fn plan() -> Plan {
        Plan {
            week_start: "2026-07-06".into(),
            daily_hours_cap: 4,
            blocks: vec![bp()],
        }
    }

    #[test]
    fn validate_rejects_zero_blocks() {
        let p = Plan {
            week_start: "2026-07-06".into(),
            daily_hours_cap: 4,
            blocks: vec![],
        };
        assert!(validate(&p).is_err());
    }

    #[test]
    fn validate_accepts_valid() {
        assert!(validate(&plan()).is_ok());
    }

    #[test]
    fn validate_rejects_bad_time() {
        let mut p = plan();
        p.blocks[0].start = "9:00".into();
        assert!(validate(&p).is_err());
    }

    #[test]
    fn validate_rejects_zero_minutes() {
        let mut p = plan();
        p.blocks[0].minutes = 0;
        assert!(validate(&p).is_err());
    }

    #[test]
    fn round_trip_save_get() {
        let d = tempdir().unwrap();
        let paths = db::AppPaths::from_root(d.path().to_path_buf()).unwrap();
        std::fs::create_dir_all(&paths.data_dir).unwrap();
        let conn = db::open(&paths).unwrap();
        let _id = save(&conn, &plan()).unwrap();
        let got = get(&conn, "2026-07-06").unwrap();
        assert!(got.is_some());
        assert_eq!(got.unwrap().blocks.len(), 1);
    }

    #[test]
    fn build_context_handles_empty_db() {
        let d = tempdir().unwrap();
        let paths = db::AppPaths::from_root(d.path().to_path_buf()).unwrap();
        std::fs::create_dir_all(&paths.data_dir).unwrap();
        let conn = db::open(&paths).unwrap();
        let s = build_context(&conn, "2026-07-06", 4).unwrap();
        assert!(s.contains("Tasks due this week:"));
        assert!(s.contains("Recent notes:"));
        assert!(s.contains("SRS backlog: 0"));
    }
}
