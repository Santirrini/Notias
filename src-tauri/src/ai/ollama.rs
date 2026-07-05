use super::provider::{Provider, ProviderStatus, CompleteRequest, Completion, ChatRequest, ChatChunk};
use crate::error::{AppError, AppResult};
use reqwest::Client;
use std::path::Path;

pub struct OllamaProvider {
    pub base_url: String,
    pub client: Client,
}

impl OllamaProvider {
    pub fn new(base_url: impl Into<String>, client: Client) -> Self {
        Self { base_url: base_url.into(), client }
    }
}

impl Provider for OllamaProvider {
    fn name(&self) -> &'static str { "ollama" }
    async fn health(&self) -> AppResult<ProviderStatus> { unimplemented!() }
    async fn complete(&self, _req: CompleteRequest) -> AppResult<Completion> { unimplemented!() }
    async fn chat_stream(&self, _req: ChatRequest, _on_chunk: Box<dyn Fn(ChatChunk) + Send + Sync>) -> AppResult<Completion> { unimplemented!() }
    async fn embed(&self, _inputs: &[String], _model: &str) -> AppResult<Vec<Vec<f32>>> { unimplemented!() }
    async fn transcribe(&self, _audio_path: &Path) -> AppResult<String> {
        Err(AppError::Config("transcribe not supported by this provider".into()))
    }
    async fn summarize(&self, _text: &str, _style: &str) -> AppResult<String> { unimplemented!() }
}