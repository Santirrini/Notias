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
    pub recovery_required: bool,
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
    let recovery_required = matches!(report, db::IntegrityReport::Mismatch);
    if !recovery_required {
        let version: i64 = conn.query_row("SELECT COALESCE(MAX(version), 0) FROM schema_version", [], |r| r.get(0)).unwrap();
        let _ = db::write_meta(&paths.meta_file, &db::migrations::db_hash(&conn).unwrap(), version);
    } else {
        tracing::error!("db integrity mismatch; recovery screen will offer rebuild");
    }

    let state = AppState { paths, db: Mutex::new(conn), recovery_required };
    tracing::info!("notias starting; data_dir={:?}", state.paths.data_dir);

    tauri::Builder::default()
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            commands::ping,
            commands::recovery_required,
            notes::list_notes,
            notes::get_note,
            notes::create_note,
            notes::update_note,
            notes::delete_note,
            notes::search_notes,
            notes::rebuild_index,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
