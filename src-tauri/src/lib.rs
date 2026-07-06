pub mod ai;
pub mod commands;
pub mod commands_secrets;
pub mod db;
pub mod error;
pub mod notes;
pub mod srs;
pub mod secrets;
pub mod study;
pub mod tasks;
pub mod time_util;

use db::AppPaths;
use std::sync::{Arc, Mutex};

pub struct AppState {
    pub paths: AppPaths,
    pub db: Mutex<rusqlite::Connection>,
    pub recovery_required: bool,
    pub http: reqwest::Client,
    pub router: Arc<crate::ai::Router>,
    pub embed_jobs: Mutex<std::collections::HashMap<String, tauri::async_runtime::JoinHandle<()>>>,
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

    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .expect("http client");

    let ollama_provider: Option<Arc<crate::ai::ollama::OllamaProvider>> = {
        let rows = db::get_provider_settings(&conn).expect("provider settings");
        rows.into_iter()
            .find(|(n, _, _)| n == "ollama")
            .and_then(|(_, enabled, cfg)| if enabled {
                serde_json::from_str::<serde_json::Value>(&cfg).ok()
                    .and_then(|v| v.get("base_url").and_then(|u| u.as_str()).map(|s| s.to_string()))
                    .map(|url| Arc::new(crate::ai::ollama::OllamaProvider::new(url, http.clone())))
            } else { None })
    };
    let router = crate::ai::Router::new(ollama_provider);
    let _ = tauri::async_runtime::block_on(router.reload(&conn));

    let state = AppState {
        paths,
        db: Mutex::new(conn),
        recovery_required,
        http,
        router,
        embed_jobs: Mutex::new(std::collections::HashMap::new()),
    };
    tracing::info!("notias starting; data_dir={:?}", state.paths.data_dir);

    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
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
            ai::list_providers,
            ai::enable_provider,
            ai::test_provider,
            ai::ai_chat,
            ai::ai_complete,
            ai::ai_summarize,
            ai::rag_search,
            ai::ai_transcribe,
            commands_secrets::set_provider_key,
            commands_secrets::delete_provider_key,
            commands_secrets::has_provider_key,
            commands_secrets::provider_key_status,
            tasks::list_tasks,
            tasks::create_task,
            tasks::update_task,
            tasks::delete_task,
            srs::generate_cards,
            srs::save_cards,
            srs::queue,
            srs::review,
            srs::suspend,
            study::generate_quiz,
            study::grade_quiz,
            study::generate_plan,
            study::save_plan,
            study::get_plan,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
