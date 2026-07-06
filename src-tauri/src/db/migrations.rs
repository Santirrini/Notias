use crate::error::{AppError, AppResult};
use rusqlite::Connection;
use sha2::{Digest, Sha256};

// ponytail: compile-time enumeration so production bundles (no migrations/ on disk) still apply.
// When adding a new migration: append its number here, add an `include_str!` arm in `run()`,
// and add the SQL file under migrations/.
const LATEST_VERSION: i64 = 4;

pub fn run(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER PRIMARY KEY,
            applied_at TEXT NOT NULL
        )",
    )?;
    let current: i64 = conn
        .query_row("SELECT COALESCE(MAX(version), 0) FROM schema_version", [], |r| r.get(0))?;

    for version in (current + 1)..=LATEST_VERSION {
        let sql = match version {
            1 => include_str!("../../migrations/0001_initial.sql"),
            2 => include_str!("../../migrations/0002_notes_initial.sql"),
            3 => include_str!("../../migrations/0003_ai_provider_settings.sql"),
            4 => include_str!("../../migrations/0004_provider_priority.sql"),
            _ => return Err(AppError::Config(format!("unknown migration {version}"))),
        };
        let tx = conn.unchecked_transaction()?;
        tx.execute_batch(sql)?;
        tx.execute(
            "INSERT INTO schema_version (version, applied_at) VALUES (?1, datetime('now'))",
            [version],
        )?;
        tx.commit()?;
    }
    Ok(())
}

pub fn db_hash(conn: &Connection) -> AppResult<String> {
    // ponytail: hash of page count + schema version. Stronger integrity is a later concern;
    // see Phase 0 review notes — known gap is that unrelated page bloat changes hash and
    // schema-stable edits do not. Upgrade to PRAGMA quick_check + WAL hash if mismatches
    // surface in production telemetry.
    let page_count: i64 = conn.query_row("PRAGMA page_count", [], |r| r.get(0))?;
    let version: i64 = conn.query_row("SELECT MAX(version) FROM schema_version", [], |r| r.get(0))?;
    let mut hasher = Sha256::new();
    hasher.update(page_count.to_be_bytes());
    hasher.update(version.to_be_bytes());
    Ok(hex::encode(hasher.finalize()))
}

pub fn read_schema_version(conn: &Connection) -> AppResult<i64> {
    let v: i64 = conn.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_version",
        [],
        |r| r.get(0),
    )?;
    Ok(v)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn migrations_apply_once() {
        let conn = Connection::open_in_memory().unwrap();
        run(&conn).unwrap();
        let v: i64 = conn.query_row("SELECT MAX(version) FROM schema_version", [], |r| r.get(0)).unwrap();
        assert_eq!(v, 4);

        // Re-running is a no-op.
        run(&conn).unwrap();
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM schema_version", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn migration_0004_seed_rows() {
        let conn = Connection::open_in_memory().unwrap();
        run(&conn).unwrap();
        let mut stmt = conn.prepare("SELECT name, chat_priority, transcribe_provider FROM provider_settings ORDER BY name").unwrap();
        let rows: Vec<(String, i64, String)> = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))).unwrap()
            .filter_map(Result::ok).collect();
        assert!(rows.iter().any(|(n, _, _)| n == "ollama"));
        assert!(rows.iter().any(|(n, _, _)| n == "openai"));
        assert!(rows.iter().any(|(n, _, _)| n == "groq"));
        let ollama = rows.iter().find(|(n, _, _)| n == "ollama").unwrap();
        assert_eq!(ollama.1, 0, "ollama priority must be 0");
    }

    #[test]
    fn db_hash_changes_with_schema() {
        let conn = Connection::open_in_memory().unwrap();
        run(&conn).unwrap();
        let h1 = db_hash(&conn).unwrap();
        conn.execute_batch("CREATE TABLE extra (x INTEGER)").unwrap();
        let h2 = db_hash(&conn).unwrap();
        assert_ne!(h1, h2);
    }
}