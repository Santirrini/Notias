use crate::error::{AppError, AppResult};
use rusqlite::Connection;
use sha2::{Digest, Sha256};

// ponytail: compile-time enumeration so production bundles (no migrations/ on disk) still apply.
// When adding a new migration: append its number here, add an `include_str!` arm in `run()`,
// and add the SQL file under migrations/.
const LATEST_VERSION: i64 = 6;

/// Function pointer for sqlite3_vec_init, used with `sqlite3_auto_extension` to make
/// sqlite-vec available to every newly opened SQLite connection.
fn sqlite_vec_init_ptr() -> unsafe extern "C" fn() {
    unsafe { std::mem::transmute(sqlite_vec::sqlite3_vec_init as *const ()) }
}

/// Process-wide registration of the sqlite-vec auto-extension. Idempotent.
/// MUST be called before opening any connection that will use `vec0` (migration 0003
/// creates a vec0 virtual table). Safe to call from anywhere.
pub fn ensure_sqlite_vec_registered() {
    use std::sync::Once;
    static ONCE: Once = Once::new();
    ONCE.call_once(|| unsafe {
        rusqlite::ffi::sqlite3_auto_extension(Some(std::mem::transmute::<
            unsafe extern "C" fn(),
            unsafe extern "C" fn(
                *mut rusqlite::ffi::sqlite3,
                *mut *const std::os::raw::c_char,
                *const rusqlite::ffi::sqlite3_api_routines,
            ) -> std::os::raw::c_int,
        >(sqlite_vec_init_ptr())));
    });
}

/// Test helper: open an in-memory DB with sqlite-vec pre-registered.
/// In production, `db::open` registers vec0 before opening the connection.
/// For test fixtures that create connections ad-hoc, this ensures vec0 is available
/// before any `CREATE VIRTUAL TABLE USING vec0` runs.
#[cfg(test)]
pub fn open_test_in_memory() -> rusqlite::Connection {
    ensure_sqlite_vec_registered();
    rusqlite::Connection::open_in_memory().expect("open_in_memory")
}

pub fn run(conn: &Connection) -> AppResult<()> {
    ensure_sqlite_vec_registered();
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
            5 => include_str!("../../migrations/0005_study.sql"),
            6 => include_str!("../../migrations/0006_calendar.sql"),
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
    // Tolerate uninitialized DBs (test fixtures, fresh install before migrations): treat
    // a missing schema_version as version 0. In production, `db::open` runs migrations
    // before any caller reaches this function.
    let version: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_version",
            [],
            |r| r.get(0),
        )
        .or_else(|e| match e {
            rusqlite::Error::SqliteFailure(err, _)
                if err.code == rusqlite::ErrorCode::Unknown
                    && err.extended_code == rusqlite::ffi::SQLITE_ERROR =>
            {
                Ok(0)
            }
            other => Err(other),
        })?;
    let mut hasher = Sha256::new();
    hasher.update(page_count.to_be_bytes());
    hasher.update(version.to_be_bytes());
    Ok(hex::encode(hasher.finalize()))
}

pub fn read_schema_version(conn: &Connection) -> AppResult<i64> {
    let v: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_version",
            [],
            |r| r.get(0),
        )
        .or_else(|e| match e {
            rusqlite::Error::SqliteFailure(err, _)
                if err.code == rusqlite::ErrorCode::Unknown
                    && err.extended_code == rusqlite::ffi::SQLITE_ERROR =>
            {
                Ok(0)
            }
            other => Err(other),
        })?;
    Ok(v)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn migrations_apply_once() {
        let conn = open_test_in_memory();
        run(&conn).unwrap();
        let v: i64 = conn.query_row("SELECT MAX(version) FROM schema_version", [], |r| r.get(0)).unwrap();
        assert_eq!(v, 6);
        // First run records one row per migration.
        let count_after_first: i64 = conn
            .query_row("SELECT COUNT(*) FROM schema_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count_after_first, 6);

        // Re-running is a no-op: no new rows, version unchanged.
        run(&conn).unwrap();
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM schema_version", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 6);
        let v2: i64 = conn.query_row("SELECT MAX(version) FROM schema_version", [], |r| r.get(0)).unwrap();
        assert_eq!(v2, 6);
    }

    #[test]
    fn migration_0004_seed_rows() {
        let conn = open_test_in_memory();
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
    fn migration_0005_study_tables_present() {
        let conn = open_test_in_memory();
        run(&conn).unwrap();
        let names: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name IN ('tasks','srs_cards','srs_reviews','study_sessions','quiz_attempts') ORDER BY name")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .filter_map(Result::ok)
            .collect();
        assert_eq!(names, vec!["quiz_attempts", "srs_cards", "srs_reviews", "study_sessions", "tasks"]);

        // srs_cards must have the UNIQUE(note_id, front) constraint for idempotency.
        let idx: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='index' AND name='idx_srs_nodupe'")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .filter_map(Result::ok)
            .collect();
        assert_eq!(idx, vec!["idx_srs_nodupe".to_string()]);
    }

    #[test]
    fn db_hash_changes_with_schema() {
        let conn = open_test_in_memory();
        run(&conn).unwrap();
        let h1 = db_hash(&conn).unwrap();
        conn.execute_batch("CREATE TABLE extra (x INTEGER)").unwrap();
        let h2 = db_hash(&conn).unwrap();
        assert_ne!(h1, h2);
    }
}