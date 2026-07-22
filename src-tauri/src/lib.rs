pub mod ai;
pub mod calendar;
pub mod commands;
pub mod db;
pub mod diagnostics;
pub mod error;
pub mod notes;
pub mod oauth;
pub mod srs;
pub mod secrets;
pub mod study;
pub mod tasks;
pub mod time_util;

use crate::diagnostics::types::StartupError;
use crate::diagnostics::{log_setup, panic_hook};
use db::AppPaths;
use std::sync::{Arc, Mutex};
use tauri::Emitter;

pub struct AppState {
    pub paths: AppPaths,
    /// `None` only when bootstrap failed before the DB could be opened.
    /// Commands that need the DB check this and return a clear error
    /// instead of panicking, so the UI can render the startup error
    /// screen instead of a Tauri rejection.
    pub db: Option<Mutex<rusqlite::Connection>>,
    pub recovery_required: bool,
    pub http: reqwest::Client,
    pub router: Arc<crate::ai::Router>,
    pub embed_jobs: Mutex<std::collections::HashMap<String, tauri::async_runtime::JoinHandle<()>>>,
    /// First fatal error during bootstrap, if any. Surfaced to the
    /// frontend via the `startup_error` IPC command and the
    /// `notias:startup-error` event.
    pub startup_error: Option<StartupError>,
}

impl AppState {
    /// Acquire the DB connection guard. Returns a clear error if the DB
    /// isn't available (bootstrap failed) or the mutex is poisoned. This
    /// is the *only* correct way for commands to touch `state.db`.
    pub fn db_conn(&self) -> Result<std::sync::MutexGuard<'_, rusqlite::Connection>, error::AppError> {
        let mutex = self
            .db
            .as_ref()
            .ok_or_else(|| error::AppError::Config("database unavailable (startup failed)".into()))?;
        mutex.lock().map_err(|_| error::AppError::Config("db lock poisoned".into()))
    }
}

/// Outcome of the bootstrap phase. The window always opens; the
/// `startup_error` field tells the frontend to render the error screen
/// when paths/db couldn't be brought up.
struct AppBootstrap {
    state: AppState,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 1. Best-effort: get paths so we can install the file logger and
    // panic hook. If even `AppPaths::new()` fails (no project dirs on
    // some exotic OS), fall back to stdout-only logging and keep going.
    let paths_for_logs = AppPaths::new().ok();

    if let Some(ref p) = paths_for_logs {
        let _ = log_setup::init(p);
    }
    if let Some(ref p) = paths_for_logs {
        panic_hook::install(&p.logs_dir);
    }

    // Stamp the process start time so the snapshot has something to show.
    let started = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    let _ = diagnostics::commands::STARTED_AT_MS.set(started);

    // 2. Graceful bootstrap — every step wraps its error in a
    // `StartupError` instead of `.expect()`-ing. The window will still
    // open; the frontend decides what to render.
    let boot = bootstrap(paths_for_logs);

    let state = boot.state;

    // 3. If bootstrap produced an error, emit it so the frontend can
    // render the StartupErrorScreen. The frontend also calls the
    // `startup_error` IPC at mount time as a safety net.
    let startup_error_for_event = state.startup_error.clone();
    let bootstrap_state = state.db.is_none();

    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_os::init())
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            commands::ping,
            commands::recovery_required,
            commands::set_provider_key,
            commands::delete_provider_key,
            commands::has_provider_key,
            commands::provider_key_status,
            ai::ai_chat,
            ai::ai_complete,
            ai::ai_summarize,
            ai::ai_transcribe,
            ai::list_providers,
            ai::enable_provider,
            ai::test_provider,
            ai::rag_search,
            srs::save_cards,
            srs::generate_cards,
            notes::list_notes,
            notes::get_note,
            notes::create_note,
            notes::update_note,
            notes::delete_note,
            notes::search_notes,
            notes::rebuild_index,
            notes::sync_export_zip,
            notes::sync_import_zip,
            notes::sync_rebuild_now,
            notes::save_drawing,
            notes::read_drawing,
            notes::read_drawing_state,
            notes::delete_drawing,
            tasks::list_tasks,
            tasks::create_task,
            tasks::update_task,
            tasks::delete_task,
            srs::queue,
            srs::review,
            srs::suspend,
            study::generate_quiz,
            study::grade_quiz,
            study::generate_plan,
            study::save_plan,
            study::get_plan,
            calendar::calendar_auth_status,
            calendar::calendar_connect,
            calendar::calendar_disconnect,
            calendar::calendar_pull,
            calendar::calendar_create,
            calendar::calendar_list,
            diagnostics::commands::diagnostics_snapshot,
            diagnostics::commands::diagnostics_recent_logs,
            diagnostics::commands::diagnostics_log_path,
            diagnostics::commands::diagnostics_open_logs_folder,
            diagnostics::commands::diagnostics_export_report,
            diagnostics::commands::startup_error,
        ])
        .setup(move |app| {
            if bootstrap_state {
                if let Some(err) = &startup_error_for_event {
                    let _ = app.emit("notias:startup-error", err.clone());
                }
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Run the startup phases with full error capture. Each failure is
/// folded into `state.startup_error`; only `paths` is required for the
/// window to open.
fn bootstrap(paths_for_logs: Option<AppPaths>) -> AppBootstrap {
    // Phase: paths
    let paths = match paths_for_logs.or_else(|| AppPaths::new().ok()) {
        Some(p) => p,
        None => {
            return AppBootstrap {
                state: AppState {
                    paths: empty_paths(),
                    db: None,
                    recovery_required: false,
                    http: dummy_http_client(),
                    router: crate::ai::Router::new(None),
                    embed_jobs: Mutex::new(Default::default()),
                    startup_error: Some(StartupError {
                        code: "config".into(),
                        phase: "paths".into(),
                        message: "could not resolve OS data directory".into(),
                        detail: None,
                    }),
                },
            };
        }
    };

    // Phase: db open
    let conn = match db::open(&paths) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("db open failed: {e}");
            return AppBootstrap {
                state: AppState {
                    paths,
                    db: None,
                    recovery_required: false,
                    http: dummy_http_client(),
                    router: crate::ai::Router::new(None),
                    embed_jobs: Mutex::new(Default::default()),
                    startup_error: Some(StartupError {
                        code: error_code(&e),
                        phase: "db_open".into(),
                        message: e.to_string(),
                        detail: None,
                    }),
                },
            };
        }
    };

    // Phase: integrity verify
    let recovery_required = match db::verify(&paths.meta_file, &conn) {
        Ok(report) => matches!(report, db::IntegrityReport::Mismatch),
        Err(e) => {
            tracing::error!("db verify failed: {e}");
            return AppBootstrap {
                state: AppState {
                    paths,
                    db: Some(Mutex::new(conn)),
                    recovery_required: false,
                    http: dummy_http_client(),
                    router: crate::ai::Router::new(None),
                    embed_jobs: Mutex::new(Default::default()),
                    startup_error: Some(StartupError {
                        code: error_code(&e),
                        phase: "db_verify".into(),
                        message: e.to_string(),
                        detail: None,
                    }),
                },
            };
        }
    };
    if !recovery_required {
        if let Ok(version) = conn.query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_version",
            [],
            |r| r.get::<_, i64>(0),
        ) {
            let _ = db::write_meta(
                &paths.meta_file,
                &db::migrations::db_hash(&conn).unwrap_or_default(),
                version,
            );
        }
    } else {
        tracing::error!("db integrity mismatch; recovery screen will offer rebuild");
    }

    // Phase: http client
    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .unwrap_or_else(|e| {
            tracing::warn!("http client build failed ({e}); using default client");
            reqwest::Client::new()
        });

    // Phase: provider bootstrap
    let ollama_provider: Option<Arc<crate::ai::ollama::OllamaProvider>> = db::get_provider_settings(&conn)
        .ok()
        .and_then(|rows| {
            rows.into_iter()
                .find(|(n, _, _)| n == "ollama")
                .and_then(|(_, enabled, cfg)| {
                    if enabled {
                        serde_json::from_str::<serde_json::Value>(&cfg).ok()
                            .and_then(|v| v.get("base_url").and_then(|u| u.as_str()).map(|s| s.to_string()))
                            .map(|url| Arc::new(crate::ai::ollama::OllamaProvider::new(url, http.clone())))
                    } else {
                        None
                    }
                })
        });
    let router = crate::ai::Router::new(ollama_provider);
    let _ = tauri::async_runtime::block_on(router.reload(&conn));

    let state = AppState {
        paths,
        db: Some(Mutex::new(conn)),
        recovery_required,
        http,
        router,
        embed_jobs: Mutex::new(Default::default()),
        startup_error: None,
    };

    // Phase: startup rebuild (best-effort). Errors are logged but not
    // promoted to startup_error — the existing RecoveryBanner handles
    // them at the UI level.
    if notes::sync::rebuild_if_stale(&state.paths.notes_dir, &state.paths.db_file) {
        tracing::info!("notes/ has changes newer than notias.db; rebuilding at startup");
        if let Some(db) = &state.db {
            let conn = db.lock().expect("db lock");
            let _ = notes::index::rebuild_from_disk(&conn, &state.paths.notes_dir);
            let _ = db::write_meta(
                &state.paths.meta_file,
                &db::migrations::db_hash(&conn).unwrap_or_default(),
                db::migrations::read_schema_version(&conn).unwrap_or(0),
            );
        }
    }

    // Phase: file watcher (best-effort, never fatal).
    if let Ok(c) = notes::sync::open_watcher_connection(&state.paths.db_file) {
        let _watcher = notes::sync::spawn_watcher(
            state.paths.notes_dir.clone(),
            Arc::new(Mutex::new(c)),
            std::time::Duration::from_millis(500),
        );
    } else {
        tracing::warn!("watcher: open failed");
    }

    tracing::info!("notias starting; data_dir={:?}", state.paths.data_dir);

    AppBootstrap { state }
}

/// Map an `AppError` to the same wire-code vocabulary used by its
/// `Serialize` impl, so the startup-error view stays consistent with
/// runtime command errors.
fn error_code(e: &error::AppError) -> String {
    use crate::error::AppError as A;
    match e {
        A::Io(_) => "io".into(),
        A::Db(_) => "db".into(),
        A::Json(_) | A::Yaml(_) => "serialization".into(),
        A::Keyring(_) => "auth".into(),
        A::Zip(_) => "io".into(),
        A::Config(_) => "config".into(),
        A::NotFound(_) => "not_found".into(),
        A::Invalid(_) => "invalid".into(),
        A::Provider(_) => "provider".into(),
        A::Auth(_) => "auth".into(),
    }
}

fn dummy_http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .unwrap_or_default()
}

fn empty_paths() -> AppPaths {
    // Best-effort fallback so the window still opens even on the
    // weirdest failure mode. Uses CWD-relative paths so nothing breaks
    // downstream — every consumer treats the directory as opaque.
    AppPaths::from_root(std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")))
        .unwrap_or_else(|_| AppPaths {
            data_dir: std::path::PathBuf::from("."),
            notes_dir: std::path::PathBuf::from("./notes"),
            db_file: std::path::PathBuf::from("./notias.db"),
            meta_file: std::path::PathBuf::from("./notias.meta"),
            config_file: std::path::PathBuf::from("./config.json"),
            logs_dir: std::path::PathBuf::from("./logs"),
            reports_dir: std::path::PathBuf::from("./reports"),
        })
}