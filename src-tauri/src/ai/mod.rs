pub mod provider;
pub mod ollama;
pub mod router;
pub mod embed;
pub mod prompts;

pub use provider::{Provider, ProviderStatus, CompleteRequest, Completion, ChatMessage, ChatRequest, ChatChunk};
pub use router::Router;