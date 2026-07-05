pub mod integrity;
pub mod migrations;
pub mod paths;

pub use paths::AppPaths;
pub use integrity::{write_meta, verify, IntegrityReport};

use rusqlite::Connection;
use rusqlite::params;
use crate::error::AppResult;

pub fn open(paths: &paths::AppPaths) -> AppResult<Connection> {
    std::fs::create_dir_all(&paths.data_dir)?;
    let conn = Connection::open(&paths.db_file)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    migrations::run(&conn)?;
    // ponytail: best-effort sqlite-vec load. If the runtime is missing (dev box without
    // sqlite-vec), the embed worker degrades to provider-error at runtime, never panics.
    let _ = conn.enable_load_extension(true);
    let _ = conn.load_extension("vec0");
    let _ = conn.enable_load_extension(false);
    Ok(conn)
}

pub fn get_provider_settings(conn: &Connection) -> AppResult<Vec<(String, bool, String)>> {
    let mut stmt = conn.prepare("SELECT name, enabled, config_json FROM provider_settings ORDER BY name")?;
    let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get::<_, i64>(1)? != 0, r.get(2)?)))?
        .filter_map(Result::ok).collect();
    Ok(rows)
}

pub fn set_provider_setting(conn: &Connection, name: &str, enabled: bool, config_json: &str) -> AppResult<()> {
    conn.execute(
        "INSERT INTO provider_settings(name, enabled, config_json) VALUES(?1, ?2, ?3)
         ON CONFLICT(name) DO UPDATE SET enabled=excluded.enabled, config_json=excluded.config_json",
        params![name, enabled as i64, config_json],
    )?;
    Ok(())
}