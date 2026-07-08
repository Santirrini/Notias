use serde::{Deserialize, Serialize};
use std::path::Path;
use crate::error::{AppError, AppResult};
use async_trait::async_trait;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderStatus {
    pub healthy: bool,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompleteRequest {
    pub prompt: String,
    pub model: String,
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Completion {
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub messages: Vec<ChatMessage>,
    pub model: String,
    #[serde(default = "default_provider")]
    pub provider: String,
}

fn default_provider() -> String { "auto".into() }

#[derive(Debug, Clone, Serialize)]
pub struct ChatChunk {
    pub text: String,
}

#[async_trait]
pub trait Provider: Send + Sync {
    fn name(&self) -> &'static str;
    async fn health(&self) -> AppResult<ProviderStatus>;
    async fn complete(&self, req: CompleteRequest) -> AppResult<Completion>;
    async fn chat_stream(
        &self,
        req: ChatRequest,
        on_chunk: Box<dyn Fn(ChatChunk) + Send + Sync>,
    ) -> AppResult<Completion>;
    async fn embed(&self, inputs: &[String], model: &str) -> AppResult<Vec<Vec<f32>>>;
    async fn transcribe(&self, _audio_path: &Path) -> AppResult<String> {
        Err(AppError::Config("transcribe not supported by this provider".into()))
    }
    async fn summarize(&self, text: &str, style: &str) -> AppResult<String>;
}