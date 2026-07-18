use super::provider::*;
use super::prompts;
use crate::error::{AppError, AppResult};
use reqwest::Client;
use reqwest::Response;
use serde::Deserialize;

pub struct OpenAiProvider {
    pub client: Client,
    pub api_key: String,
    pub base_url: String,
    pub chat_model: String,
    pub embed_model: String,
}

impl OpenAiProvider {
    pub fn new(api_key: String, client: Client) -> Self {
        Self {
            client,
            api_key,
            base_url: "https://api.openai.com/v1".into(),
            chat_model: "gpt-4o-mini".into(),
            embed_model: "text-embedding-3-small".into(),
        }
    }

    pub fn from_config(api_key: String, cfg: &serde_json::Value, client: Client) -> Self {
        let mut p = Self::new(api_key, client);
        if let Some(b) = cfg.get("base_url").and_then(|v| v.as_str()) { p.base_url = b.into(); }
        if let Some(m) = cfg.get("chat_model").and_then(|v| v.as_str()) { p.chat_model = m.into(); }
        if let Some(m) = cfg.get("embed_model").and_then(|v| v.as_str()) { p.embed_model = m.into(); }
        p
    }

    fn auth(&self, r: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        r.bearer_auth(&self.api_key)
    }
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}
#[derive(Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}
#[derive(Deserialize)]
struct ChatStreamChunk {
    choices: Vec<ChatStreamChoice>,
}
#[derive(Deserialize)]
struct ChatStreamChoice {
    delta: ChatDelta,
}
#[derive(Deserialize)]
struct ChatDelta {
    #[serde(default)]
    content: String,
}
#[derive(Deserialize)]
struct EmbedResponse {
    data: Vec<EmbedData>,
}
#[derive(Deserialize)]
struct EmbedData {
    embedding: Vec<f32>,
}

async fn status_detail(r: Response) -> ProviderStatus {
    let s = r.status();
    let body = r.text().await.unwrap_or_default();
    ProviderStatus {
        healthy: s.is_success(),
        detail: if s.is_success() { None } else { Some(format!("HTTP {}: {}", s.as_u16(), &body[..body.len().min(200)])) },
    }
}

#[async_trait::async_trait]
impl Provider for OpenAiProvider {
    fn name(&self) -> &'static str { "openai" }

    async fn health(&self) -> AppResult<ProviderStatus> {
        let r = self.auth(self.client.get(format!("{}/models", self.base_url)))
            .send().await
            .map_err(|e| AppError::Provider(format!("openai: {e}")))?;
        if r.status() == 401 {
            return Ok(ProviderStatus { healthy: false, detail: Some("invalid api key".into()) });
        }
        Ok(status_detail(r).await)
    }

    async fn complete(&self, req: CompleteRequest) -> AppResult<Completion> {
        let body = serde_json::json!({
            "model": req.model,
            "messages": [{ "role": "user", "content": req.prompt }],
            "stream": false,
        });
        let r = self.auth(self.client.post(format!("{}/chat/completions", self.base_url)))
            .json(&body).send().await
            .map_err(|e| AppError::Provider(format!("openai: {e}")))?;
        if !r.status().is_success() {
            let s = r.status();
            let body = r.text().await.unwrap_or_default();
            return Err(AppError::Provider(format!("openai {s}: {body}")));
        }
        let parsed: ChatResponse = r.json().await
            .map_err(|e| AppError::Provider(format!("openai parse: {e}")))?;
        Ok(Completion { text: parsed.choices.into_iter().next().map(|c| c.message.content).unwrap_or_default() })
    }

    async fn chat_stream(&self, req: ChatRequest, on_chunk: Box<dyn Fn(ChatChunk) + Send + Sync>) -> AppResult<Completion> {
        let body = serde_json::json!({
            "model": req.model,
            "messages": req.messages,
            "stream": true,
        });
        let mut r = self.auth(self.client.post(format!("{}/chat/completions", self.base_url)))
            .json(&body).send().await
            .map_err(|e| AppError::Provider(format!("openai: {e}")))?;
        if !r.status().is_success() {
            let s = r.status();
            let body = r.text().await.unwrap_or_default();
            return Err(AppError::Provider(format!("openai {s}: {body}")));
        }
        let mut full = String::new();
        while let Some(line) = r.chunk().await.map_err(|e| AppError::Provider(format!("openai stream: {e}")))? {
            let s = std::str::from_utf8(&line).unwrap_or("");
            for l in s.lines() {
                let payload = l.strip_prefix("data:").map(|x| x.trim()).unwrap_or("");
                if payload.is_empty() || payload == "[DONE]" { continue; }
                if let Ok(c) = serde_json::from_str::<ChatStreamChunk>(payload) {
                    for choice in c.choices {
                        if !choice.delta.content.is_empty() {
                            full.push_str(&choice.delta.content);
                            on_chunk(ChatChunk { text: choice.delta.content });
                        }
                    }
                }
            }
        }
        Ok(Completion { text: full })
    }

    async fn embed(&self, inputs: &[String], model: &str) -> AppResult<Vec<Vec<f32>>> {
        let body = serde_json::json!({ "input": inputs, "model": model });
        let r = self.auth(self.client.post(format!("{}/embeddings", self.base_url)))
            .json(&body).send().await
            .map_err(|e| AppError::Provider(format!("openai embed: {e}")))?;
        if !r.status().is_success() {
            let s = r.status();
            let body = r.text().await.unwrap_or_default();
            return Err(AppError::Provider(format!("openai embed {s}: {body}")));
        }
        let parsed: EmbedResponse = r.json().await
            .map_err(|e| AppError::Provider(format!("openai embed parse: {e}")))?;
        Ok(parsed.data.into_iter().map(|d| d.embedding).collect())
    }

    async fn summarize(&self, text: &str, style: &str) -> AppResult<String> {
        let prompt = prompts::summarize(text, style);
        self.complete(CompleteRequest { prompt, model: self.chat_model.clone(), max_tokens: None })
            .await.map(|c| c.text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn from_config_overrides_defaults() {
        let cfg = serde_json::json!({
            "base_url": "https://proxy.example/v1",
            "chat_model": "gpt-4o",
            "embed_model": "text-embedding-3-large",
        });
        let p = OpenAiProvider::from_config("sk-test".into(), &cfg, Client::new());
        assert_eq!(p.base_url, "https://proxy.example/v1");
        assert_eq!(p.chat_model, "gpt-4o");
        assert_eq!(p.embed_model, "text-embedding-3-large");
    }

    #[test]
    fn new_uses_defaults() {
        let p = OpenAiProvider::new("sk-test".into(), Client::new());
        assert_eq!(p.base_url, "https://api.openai.com/v1");
        assert_eq!(p.chat_model, "gpt-4o-mini");
        assert_eq!(p.embed_model, "text-embedding-3-small");
    }
}
