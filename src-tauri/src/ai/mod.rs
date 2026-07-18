pub mod provider;
pub mod ollama;
pub mod openai;
pub mod groq;
pub mod router;
pub mod embed;
pub mod prompts;
pub mod json_helpers;

pub use provider::{Provider, ProviderStatus, CompleteRequest, Completion, ChatMessage, ChatRequest, ChatChunk};
pub use openai::OpenAiProvider;
pub use groq::GroqProvider;
pub use router::Router;
use ollama::OllamaProvider;

use std::sync::Arc;

use crate::error::{AppError, AppResult};
use crate::AppState;
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use ulid::Ulid;

#[derive(Serialize)]
pub struct ProviderInfo {
    pub name: String,
    pub enabled: bool,
    pub healthy: bool,
    pub detail: Option<String>,
}

#[derive(Serialize)]
pub struct StreamHandle {
    pub stream_id: String,
}

#[tauri::command]
pub fn list_providers(state: State<'_, AppState>) -> AppResult<Vec<ProviderInfo>> {
    let conn = state.db_conn()?;
    let rows = crate::db::get_provider_settings(&conn)?;
    drop(conn);
    let router = state.router.clone();
    let snap = tauri::async_runtime::block_on(router.snapshot());
    let mut out = vec![];
    for (name, enabled, _cfg) in rows {
        let (healthy, detail) = match snap.get(&name) {
            Some(p) => {
                let s = tauri::async_runtime::block_on(p.health())
                    .unwrap_or(ProviderStatus { healthy: false, detail: Some("health check failed".into()) });
                (s.healthy, s.detail)
            }
            None => match name.as_str() {
                "ollama" => (false, Some("disabled or unreachable".into())),
                "openai" | "groq" => (false, Some("no api key set".into())),
                _ => (false, Some("not configured".into())),
            },
        };
        out.push(ProviderInfo { name, enabled, healthy, detail });
    }
    Ok(out)
}

#[tauri::command]
pub fn enable_provider(
    name: String,
    enabled: bool,
    config_json: Option<String>,
    state: State<'_, AppState>,
) -> AppResult<()> {
    let conn = state.db_conn()?;
    let cfg = config_json.unwrap_or_else(|| "{}".into());
    crate::db::set_provider_setting(&conn, &name, enabled, &cfg)?;
    drop(conn);
    let conn = state.db_conn()?;
    let _ = tauri::async_runtime::block_on(state.router.try_reload(&conn));
    Ok(())
}

#[tauri::command]
pub fn test_provider(
    name: String,
    state: State<'_, AppState>,
) -> AppResult<ProviderStatus> {
    let router = state.router.clone();
    let snap = tauri::async_runtime::block_on(router.snapshot());
    let p = match snap.get(&name) {
        Some(p) => p,
        None => match name.as_str() {
            "ollama" => transient_ollama(&state)? as Arc<dyn Provider>,
            _ => return Err(AppError::Config(format!("unknown provider: {name}"))),
        },
    };
    tauri::async_runtime::block_on(p.health())
}

/// Probe a transient OllamaProvider from the DB row's `base_url`, regardless of whether
/// the user has toggled the provider on. Lets the Settings UI test connectivity before
/// committing to enabling it. Falls back to the canonical localhost default.
fn transient_ollama(state: &State<'_, AppState>) -> AppResult<Arc<OllamaProvider>> {
    let conn = state.db_conn()?;
    let cfg = crate::db::get_provider_settings(&conn)?
        .into_iter()
        .find(|(n, _, _)| n == "ollama")
        .map(|(_, _, c)| c);
    drop(conn);
    let url = cfg.as_deref()
        .and_then(|c| serde_json::from_str::<serde_json::Value>(c).ok())
        .and_then(|v| v.get("base_url").and_then(|u| u.as_str()).map(str::to_string))
        .unwrap_or_else(|| "http://127.0.0.1:11434".into());
    Ok(Arc::new(OllamaProvider::new(url, state.http.clone())))
}

#[tauri::command]
pub async fn ai_chat(
    req: ChatRequest,
    app: AppHandle,
    state: State<'_, AppState>,
) -> AppResult<StreamHandle> {
    let stream_id = Ulid::new().to_string();
    let provider: Arc<dyn Provider> = state.router.pick_chat().await?;
    let id = stream_id.clone();
    let app2 = app.clone();
    tauri::async_runtime::spawn(async move {
        let channel = format!("provider://stream/{id}");
        let channel_for_done = channel.clone();
        let cb = Box::new(move |chunk: ChatChunk| {
            let _ = app2.emit(&channel, serde_json::json!({"kind":"chunk","text":chunk.text}));
        });
        match provider.chat_stream(req, cb).await {
            Ok(_) => { let _ = app.emit(&channel_for_done, serde_json::json!({"kind":"done"})); }
            Err(e) => { let _ = app.emit(&channel_for_done, serde_json::json!({"kind":"error","message":e.to_string()})); }
        }
    });
    Ok(StreamHandle { stream_id })
}

#[tauri::command]
pub async fn ai_complete(
    prompt: String,
    model: Option<String>,
    state: State<'_, AppState>,
) -> AppResult<String> {
    let provider = state.router.pick_chat().await?;
    let model = model.unwrap_or_else(|| "llama3.2".into());
    Ok(provider.complete(CompleteRequest { prompt, model, max_tokens: None }).await?.text)
}

#[tauri::command]
pub async fn ai_summarize(
    text: String,
    style: String,
    state: State<'_, AppState>,
) -> AppResult<String> {
    let provider = state.router.pick_chat().await?;
    provider.summarize(&text, &style).await
}

#[tauri::command]
pub async fn ai_transcribe(
    audio_path: String,
    state: State<'_, AppState>,
) -> AppResult<String> {
    let provider = state.router.pick_transcribe().await?;
    provider.transcribe(std::path::Path::new(&audio_path)).await
}

#[derive(Serialize)]
pub struct RagHit {
    pub note_id: String,
    pub title: String,
    pub distance: f64,
}

#[tauri::command]
pub fn rag_search(query: String, state: State<'_, AppState>) -> AppResult<Vec<RagHit>> {
    let router = state.router.clone();
    let provider = tauri::async_runtime::block_on(async {
        router.local_only().await
    }).ok_or_else(|| AppError::Config("ollama not configured (embeddings require ollama)".into()))?;

    let q_vec = tauri::async_runtime::block_on(async {
        provider.embed(&[query.clone()], "nomic-embed-text").await
    }).map_err(|e| AppError::Config(format!("embed query: {e}")))?;
    let json = serde_json::to_string(&q_vec[0]).map_err(|e| AppError::Config(format!("vec encode: {e}")))?;

    let conn = state.db_conn()?;
    let mut stmt = conn.prepare(
        "SELECT n.id, n.title, vec_distance_cosine(v.embedding, vec_f32(?1)) AS dist
         FROM note_vec v JOIN notes n ON n.rowid = v.rowid
         WHERE n.deleted_at IS NULL
         ORDER BY dist ASC LIMIT 8"
    )?;
    let rows = stmt.query_map(rusqlite::params![json], |r| Ok(RagHit {
        note_id: r.get(0)?,
        title: r.get(1)?,
        distance: r.get(2)?,
    }))?.filter_map(Result::ok).collect();
    Ok(rows)
}