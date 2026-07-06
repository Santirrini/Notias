use super::provider::*;
use super::prompts;
use crate::error::{AppError, AppResult};
use reqwest::Client;
use serde::Deserialize;
use std::path::Path;

pub struct GroqProvider {
    pub client: Client,
    pub api_key: String,
    pub base_url: String,
    pub chat_model: String,
    pub transcribe_model: String,
}

impl GroqProvider {
    pub fn new(api_key: String, client: Client) -> Self {
        Self {
            client,
            api_key,
            base_url: "https://api.groq.com/openai/v1".into(),
            chat_model: "llama-3.1-70b-versatile".into(),
            transcribe_model: "whisper-large-v3-turbo".into(),
        }
    }

    pub fn from_config(api_key: String, cfg: &serde_json::Value, client: Client) -> Self {
        let mut p = Self::new(api_key, client);
        if let Some(b) = cfg.get("base_url").and_then(|v| v.as_str()) { p.base_url = b.into(); }
        if let Some(m) = cfg.get("chat_model").and_then(|v| v.as_str()) { p.chat_model = m.into(); }
        if let Some(m) = cfg.get("transcribe_model").and_then(|v| v.as_str()) { p.transcribe_model = m.into(); }
        p
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
struct StreamChunk {
    choices: Vec<StreamChoice>,
}
#[derive(Deserialize)]
struct StreamChoice {
    delta: Delta,
}
#[derive(Deserialize)]
struct Delta {
    #[serde(default)]
    content: String,
}
#[derive(Deserialize)]
struct TranscribeResponse {
    #[serde(default)]
    text: String,
}

impl Provider for GroqProvider {
    fn name(&self) -> &'static str { "groq" }

    async fn health(&self) -> AppResult<ProviderStatus> {
        let r = self.client.get(format!("{}/models", self.base_url))
            .bearer_auth(&self.api_key)
            .send().await
            .map_err(|e| AppError::Provider(format!("groq: {e}")))?;
        if r.status() == 401 {
            return Ok(ProviderStatus { healthy: false, detail: Some("invalid api key".into()) });
        }
        Ok(ProviderStatus {
            healthy: r.status().is_success(),
            detail: if r.status().is_success() { None } else { Some(format!("HTTP {}", r.status())) },
        })
    }

    async fn complete(&self, req: CompleteRequest) -> AppResult<Completion> {
        let body = serde_json::json!({
            "model": req.model,
            "messages": [{ "role": "user", "content": req.prompt }],
            "stream": false,
        });
        let r = self.client.post(format!("{}/chat/completions", self.base_url))
            .bearer_auth(&self.api_key)
            .json(&body).send().await
            .map_err(|e| AppError::Provider(format!("groq: {e}")))?;
        if !r.status().is_success() {
            let s = r.status();
            let body = r.text().await.unwrap_or_default();
            return Err(AppError::Provider(format!("groq {s}: {body}")));
        }
        let parsed: ChatResponse = r.json().await
            .map_err(|e| AppError::Provider(format!("groq parse: {e}")))?;
        Ok(Completion { text: parsed.choices.into_iter().next().map(|c| c.message.content).unwrap_or_default() })
    }

    async fn chat_stream(&self, req: ChatRequest, on_chunk: Box<dyn Fn(ChatChunk) + Send + Sync>) -> AppResult<Completion> {
        let body = serde_json::json!({
            "model": req.model,
            "messages": req.messages,
            "stream": true,
        });
        let mut r = self.client.post(format!("{}/chat/completions", self.base_url))
            .bearer_auth(&self.api_key)
            .json(&body).send().await
            .map_err(|e| AppError::Provider(format!("groq: {e}")))?;
        if !r.status().is_success() {
            let s = r.status();
            let body = r.text().await.unwrap_or_default();
            return Err(AppError::Provider(format!("groq {s}: {body}")));
        }
        let mut full = String::new();
        while let Some(line) = r.chunk().await.map_err(|e| AppError::Provider(format!("groq stream: {e}")))? {
            let s = std::str::from_utf8(&line).unwrap_or("");
            for l in s.lines() {
                let payload = l.strip_prefix("data:").map(|x| x.trim()).unwrap_or("");
                if payload.is_empty() || payload == "[DONE]" { continue; }
                if let Ok(c) = serde_json::from_str::<StreamChunk>(payload) {
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

    async fn embed(&self, _inputs: &[String], _model: &str) -> AppResult<Vec<Vec<f32>>> {
        // ponytail: Groq has no public embedding API as of spec date — return Provider error.
        // Embeddings fallback to Ollama in router; users wanting both must enable Ollama + Groq.
        Err(AppError::Provider("groq does not support embeddings".into()))
    }

    async fn transcribe(&self, audio_path: &Path) -> AppResult<String> {
        let bytes = std::fs::read(audio_path)?;
        let filename = audio_path.file_name().and_then(|s| s.to_str()).unwrap_or("audio.webm").to_string();
        let mime = if filename.ends_with(".mp3") { "audio/mpeg" }
            else if filename.ends_with(".wav") { "audio/wav" }
            else if filename.ends_with(".m4a") { "audio/m4a" }
            else { "audio/webm" };
        let part = reqwest::multipart::Part::bytes(bytes)
            .file_name(filename)
            .mime_str(mime).map_err(|e| AppError::Provider(format!("groq transcribe mime: {e}")))?;
        let form = reqwest::multipart::Form::new()
            .text("model", self.transcribe_model.clone())
            .text("response_format", "json")
            .part("file", part);

        let r = self.client.post(format!("{}/audio/transcriptions", self.base_url))
            .bearer_auth(&self.api_key)
            .multipart(form)
            .send().await
            .map_err(|e| AppError::Provider(format!("groq transcribe: {e}")))?;
        if !r.status().is_success() {
            let s = r.status();
            let body = r.text().await.unwrap_or_default();
            return Err(AppError::Provider(format!("groq transcribe {s}: {body}")));
        }
        let parsed: TranscribeResponse = r.json().await
            .map_err(|e| AppError::Provider(format!("groq transcribe parse: {e}")))?;
        Ok(parsed.text)
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
    fn defaults_are_groq_appropriate() {
        let p = GroqProvider::new("gsk-test".into(), Client::new());
        assert!(p.base_url.contains("api.groq.com"));
        assert_eq!(p.chat_model, "llama-3.1-70b-versatile");
        assert_eq!(p.transcribe_model, "whisper-large-v3-turbo");
    }

    #[test]
    fn embed_returns_provider_error() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let p = GroqProvider::new("gsk-test".into(), Client::new());
            let r = p.embed(&["x".into()], "").await;
            assert!(r.is_err());
        });
    }
}
