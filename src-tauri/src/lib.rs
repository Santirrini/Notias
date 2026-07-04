pub mod ai;
pub mod commands;
pub mod db;
pub mod error;
pub mod scheduler;
pub mod secrets;
// `pub mod notes;` is added in Chunk 2 Task 20 when src-tauri/src/notes/mod.rs exists.

use db::AppPaths;
use std::sync::Mutex;

pub struct AppState {
    pub paths: AppPaths,
    pub db: Mutex<rusqlite::Connection>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let paths = AppPaths::new().expect("paths");
    let conn = db::open(&paths).expect("db open");
    let report = db::verify(&paths.db_file, &paths.meta_file, &conn).expect("verify");
    if matches!(report, db::IntegrityReport::Ok | db::IntegrityReport::Fresh) {
        let version: i64 = conn.query_row("SELECT COALESCE(MAX(version), 0) FROM schema_version", [], |r| r.get(0)).unwrap();
        let _ = db::write_meta(&paths.meta_file, &db::migrations::db_hash(&conn).unwrap(), version);
    }
    // ponytail: Mismatch path is wired in Phase 1 (UI recovery screen). For now log and continue.

    let state = AppState { paths, db: Mutex::new(conn) };

    tauri::Builder::default()
        .manage(state)
        .invoke_handler(tauri::generate_handler![commands::ping])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
