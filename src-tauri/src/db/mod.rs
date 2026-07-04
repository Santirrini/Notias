pub mod migrations;
pub mod paths;

pub use paths::AppPaths;

use rusqlite::Connection;
use crate::error::AppResult;

pub fn open(paths: &paths::AppPaths) -> AppResult<Connection> {
    std::fs::create_dir_all(&paths.data_dir)?;
    let conn = Connection::open(&paths.db_file)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    migrations::run(&conn)?;
    Ok(conn)
}