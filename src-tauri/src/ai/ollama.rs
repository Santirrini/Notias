use super::provider::*;
use super::prompts;
use crate::error::{AppError, AppResult};
use reqwest::Client;
use serde::Deserialize;

pub struct OllamaProvider {
    pub base_url: String,
    pub client: Client,
}

impl OllamaProvider {
    pub fn new(base_url: impl Into<String>, client: Client) -> Self {
        Self { base_url: base_url.into(), client }
    }

    fn default_chat_model(&self) -> String {
        "llama3.2".into()
    }
}

#[derive(Deserialize)]
struct OllamaChatChunk {
    response: String,
    done: bool,
}

#[async_trait::async_trait]
impl Provider for OllamaProvider {
    fn name(&self) -> &'static str { "ollama" }

    async fn health(&self) -> AppResult<ProviderStatus> {
        match self.client.get(format!("{}/api/tags", self.base_url)).send().await {
            Ok(r) if r.status().is_success() => Ok(ProviderStatus { healthy: true, detail: None }),
            Ok(r) => Ok(ProviderStatus { healthy: false, detail: Some(format!("HTTP {}", r.status())) }),
            Err(e) => Ok(ProviderStatus { healthy: false, detail: Some(e.to_string()) }),
        }
    }

    async fn complete(&self, req: CompleteRequest) -> AppResult<Completion> {
        let body = serde_json::json!({ "model": req.model, "prompt": req.prompt, "stream": false });
        let r = self.client.post(format!("{}/api/generate", self.base_url)).json(&body).send().await
            .map_err(|e| AppError::Config(format!("ollama: {e}")))?;
        #[derive(Deserialize)] struct R { response: String }
        let r: R = r.json().await.map_err(|e| AppError::Config(format!("ollama parse: {e}")))?;
        Ok(Completion { text: r.response })
    }

    async fn chat_stream(&self, req: ChatRequest, on_chunk: Box<dyn Fn(ChatChunk) + Send + Sync>) -> AppResult<Completion> {
        let body = serde_json::json!({ "model": req.model, "messages": req.messages, "stream": true });
        let mut r = self.client.post(format!("{}/api/chat", self.base_url)).json(&body).send().await
            .map_err(|e| AppError::Config(format!("ollama: {e}")))?;
        let mut full = String::new();
        while let Some(line) = r.chunk().await.map_err(|e| AppError::Config(format!("ollama stream: {e}")))? {
            let line_str = std::str::from_utf8(&line).unwrap_or("");
            for l in line_str.lines() {
                if l.is_empty() { continue; }
                if let Ok(c) = serde_json::from_str::<OllamaChatChunk>(l) {
                    if !c.response.is_empty() {
                        full.push_str(&c.response);
                        on_chunk(ChatChunk { text: c.response });
                    }
                    if c.done { return Ok(Completion { text: full }); }
                }
            }
        }
        Ok(Completion { text: full })
    }

    async fn embed(&self, inputs: &[String], model: &str) -> AppResult<Vec<Vec<f32>>> {
        let body = serde_json::json!({ "model": model, "input": inputs });
        let r = self.client.post(format!("{}/api/embed", self.base_url)).json(&body).send().await
            .map_err(|e| AppError::Config(format!("ollama embed: {e}")))?;
        #[derive(Deserialize)] struct R { embeddings: Vec<Vec<f32>> }
        let r: R = r.json().await.map_err(|e| AppError::Config(format!("ollama embed parse: {e}")))?;
        Ok(r.embeddings)
    }

    async fn summarize(&self, text: &str, style: &str) -> AppResult<String> {
        let prompt = prompts::summarize(text, style);
        self.complete(CompleteRequest { prompt, model: self.default_chat_model(), max_tokens: None })
            .await.map(|c| c.text)
    }
}