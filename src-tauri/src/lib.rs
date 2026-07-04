pub mod ai;
pub mod commands;
pub mod db;
pub mod error;
pub mod notes;
pub mod scheduler;
pub mod secrets;

use db::AppPaths;
use std::sync::Mutex;

pub struct AppState {
    pub paths: AppPaths,
    pub db: Mutex<rusqlite::Connection>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "notias_lib=info".into()))
        .with_target(false)
        .try_init()
        .ok();
    let paths = AppPaths::new().expect("paths");
    let conn = db::open(&paths).expect("db open");
    let report = db::verify(&paths.meta_file, &conn).expect("verify");
    if matches!(report, db::IntegrityReport::Ok | db::IntegrityReport::Fresh) {
        let version: i64 = conn.query_row("SELECT COALESCE(MAX(version), 0) FROM schema_version", [], |r| r.get(0)).unwrap();
        let _ = db::write_meta(&paths.meta_file, &db::migrations::db_hash(&conn).unwrap(), version);
    }
    // ponytail: Mismatch path is wired in Phase 1 (UI recovery screen). For now log and continue.

    let state = AppState { paths, db: Mutex::new(conn) };
    tracing::info!("notias starting; data_dir={:?}", state.paths.data_dir);

    tauri::Builder::default()
        .manage(state)
        .invoke_handler(tauri::generate_handler![commands::ping])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
