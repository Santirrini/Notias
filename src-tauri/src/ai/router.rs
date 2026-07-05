use super::ollama::OllamaProvider;
use std::sync::Arc;

pub struct Router {
    pub ollama: Option<Arc<OllamaProvider>>,
}

impl Router {
    pub fn new(ollama: Option<Arc<OllamaProvider>>) -> Self { Self { ollama } }
    pub fn local_chat(&self) -> Option<Arc<OllamaProvider>> { self.ollama.clone() }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_no_ollama_returns_none() {
        let r = Router::new(None);
        assert!(r.local_chat().is_none());
    }
}