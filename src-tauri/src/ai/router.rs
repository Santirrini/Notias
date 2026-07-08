use super::groq::GroqProvider;
use super::ollama::OllamaProvider;
use super::openai::OpenAiProvider;
use super::provider::Provider;
use crate::error::{AppError, AppResult};
use rusqlite::Connection;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct Router {
    inner: RwLock<Inner>,
}

pub struct Inner {
    pub ollama: Option<Arc<OllamaProvider>>,
    pub openai: Option<Arc<OpenAiProvider>>,
    pub groq: Option<Arc<GroqProvider>>,
}

impl Router {
    pub fn new(ollama: Option<Arc<OllamaProvider>>) -> Arc<Self> {
        Arc::new(Self {
            inner: RwLock::new(Inner { ollama, openai: None, groq: None }),
        })
    }

    /// ponytail: best-effort reload. Failures logged but never propagated — a
    /// transient keyring miss should not break the running app.
    pub async fn try_reload(&self, conn: &Connection) -> AppResult<()> {
        match self.reload(conn).await {
            Ok(_) => Ok(()),
            Err(e) => {
                tracing::warn!("router reload failed (non-fatal): {e}");
                Ok(())
            }
        }
    }

    pub async fn reload(&self, conn: &Connection) -> AppResult<()> {
        let settings = crate::db::get_provider_settings(conn).unwrap_or_default();
        let has_key = |name: &str| crate::secrets::get(name).ok().flatten();
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .map_err(|e| AppError::Config(format!("http client: {e}")))?;

        let prior_ollama = self.inner.read().await.ollama.clone();
        let mut next = Inner { ollama: None, openai: None, groq: None };
        for (name, enabled, cfg) in settings {
            if !enabled { continue; }
            let cfg_v: serde_json::Value = serde_json::from_str(&cfg).unwrap_or_default();
            match name.as_str() {
                "ollama" => { next.ollama = prior_ollama.clone(); }
                "openai" => if let Some(k) = has_key("openai") {
                    next.openai = Some(Arc::new(OpenAiProvider::from_config(k, &cfg_v, client.clone())));
                }
                "groq" => if let Some(k) = has_key("groq") {
                    next.groq = Some(Arc::new(GroqProvider::from_config(k, &cfg_v, client.clone())));
                }
                _ => {}
            }
        }
        *self.inner.write().await = next;
        Ok(())
    }

    pub async fn pick_chat(&self) -> AppResult<Arc<dyn Provider>> {
        let g = self.inner.read().await;
        if let Some(o) = &g.ollama {
            if o.health().await.map(|s| s.healthy).unwrap_or(false) {
                return Ok(o.clone() as Arc<dyn Provider>);
            }
        }
        if let Some(p) = &g.openai { return Ok(p.clone() as Arc<dyn Provider>); }
        if let Some(p) = &g.groq   { return Ok(p.clone() as Arc<dyn Provider>); }
        Err(AppError::Config("no AI provider enabled".into()))
    }

    pub async fn pick_transcribe(&self) -> AppResult<Arc<dyn Provider>> {
        let g = self.inner.read().await;
        if let Some(p) = &g.groq { return Ok(p.clone() as Arc<dyn Provider>); }
        if let Some(p) = &g.openai { return Ok(p.clone() as Arc<dyn Provider>); }
        Err(AppError::Config("transcribe requires a cloud provider (groq/openai) — none configured".into()))
    }

    pub async fn local_only(&self) -> Option<Arc<OllamaProvider>> {
        self.inner.read().await.ollama.clone()
    }

    pub async fn snapshot(&self) -> InnerSnapshot {
        let g = self.inner.read().await;
        InnerSnapshot {
            ollama: g.ollama.clone(),
            openai: g.openai.clone(),
            groq: g.groq.clone(),
        }
    }
}

pub struct InnerSnapshot {
    pub ollama: Option<Arc<OllamaProvider>>,
    pub openai: Option<Arc<OpenAiProvider>>,
    pub groq: Option<Arc<GroqProvider>>,
}

impl InnerSnapshot {
    pub fn get(&self, name: &str) -> Option<Arc<dyn Provider>> {
        match name {
            "ollama" => self.ollama.clone().map(|a| a as Arc<dyn Provider>),
            "openai" => self.openai.clone().map(|a| a as Arc<dyn Provider>),
            "groq"   => self.groq.clone().map(|a| a as Arc<dyn Provider>),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn router_new_without_providers_errors_on_pick() {
        let r = Router::new(None);
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            assert!(r.pick_chat().await.is_err());
            assert!(r.pick_transcribe().await.is_err());
            assert!(r.local_only().await.is_none());
        });
    }

    #[test]
    fn router_holds_ollama_when_provided() {
        let o = Arc::new(OllamaProvider::new("http://localhost:11434", reqwest::Client::new()));
        let r = Router::new(Some(o));
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            assert!(r.local_only().await.is_some());
        });
    }

    #[test]
    fn reload_is_idempotent_when_no_cloud_settings() {
        let conn = crate::db::migrations::open_test_in_memory();
        crate::db::migrations::run(&conn).unwrap();
        let r = Router::new(None);
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            r.try_reload(&conn).await.unwrap();
            r.try_reload(&conn).await.unwrap();
        });
    }
}
