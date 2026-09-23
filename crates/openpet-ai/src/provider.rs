use async_trait::async_trait;
use openpet_types::{ChatMessage, ProviderCapabilities, ProviderError};
use tokio::sync::mpsc;

/// Request payload for generating streaming conversational responses.
#[derive(Debug, Clone)]
pub struct ChatRequest {
    pub messages: Vec<ChatMessage>,
    pub system_prompt: Option<String>,
    pub temperature: f32,
    pub max_tokens: Option<u32>,
}

/// Incremental token or delta returned during streaming.
#[derive(Debug, Clone, PartialEq)]
pub struct ChatChunk {
    pub text: String,
    pub is_done: bool,
}

/// Canonical asynchronous interface for all supported AI conversational backends.
#[async_trait]
pub trait ChatProvider: Send + Sync {
    /// Capabilities exposed by this model and provider.
    fn capabilities(&self) -> ProviderCapabilities;

    /// Provider identifier kind.
    fn name(&self) -> &'static str;

    /// Generate an incremental stream of response chunks.
    async fn stream_chat(
        &self,
        request: ChatRequest,
        sender: mpsc::Sender<Result<ChatChunk, ProviderError>>,
    ) -> Result<(), ProviderError>;
}
