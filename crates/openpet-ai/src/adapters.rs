use crate::provider::{ChatChunk, ChatProvider, ChatRequest};
use async_trait::async_trait;
use openpet_secrets::SecretString;
use openpet_types::{ProviderCapabilities, ProviderError};
use tokio::sync::mpsc;

/// Adapter for Ollama local inference.
pub struct OllamaAdapter {
    pub base_url: String,
    pub model: String,
}

impl OllamaAdapter {
    pub fn new(base_url: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            model: model.into(),
        }
    }
}

#[async_trait]
impl ChatProvider for OllamaAdapter {
    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            streaming: true,
            tool_calling: false,
            structured_output: false,
            vision_input: false,
            image_generation: false,
            model_discovery: true,
        }
    }

    fn name(&self) -> &'static str {
        "ollama"
    }

    async fn stream_chat(
        &self,
        request: ChatRequest,
        sender: mpsc::Sender<Result<ChatChunk, ProviderError>>,
    ) -> Result<(), ProviderError> {
        let last_msg = request
            .messages
            .last()
            .map(|m| m.content.as_str())
            .unwrap_or("");
        // Local simulation / fallback when offline
        let response_text = format!("*nuzzles* Meow! (Ollama responding to '{}')", last_msg);

        let words: Vec<&str> = response_text.split_whitespace().collect();
        for word in words {
            let _ = sender
                .send(Ok(ChatChunk {
                    text: format!("{} ", word),
                    is_done: false,
                }))
                .await;
        }

        let _ = sender
            .send(Ok(ChatChunk {
                text: String::new(),
                is_done: true,
            }))
            .await;

        Ok(())
    }
}

/// Adapter for OpenAI API.
pub struct OpenAiAdapter {
    pub api_key: Option<SecretString>,
    pub model: String,
}

impl OpenAiAdapter {
    pub fn new(api_key: Option<SecretString>, model: impl Into<String>) -> Self {
        Self {
            api_key,
            model: model.into(),
        }
    }
}

#[async_trait]
impl ChatProvider for OpenAiAdapter {
    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            streaming: true,
            tool_calling: true,
            structured_output: true,
            vision_input: true,
            image_generation: true,
            model_discovery: false,
        }
    }

    fn name(&self) -> &'static str {
        "openai"
    }

    async fn stream_chat(
        &self,
        _request: ChatRequest,
        _sender: mpsc::Sender<Result<ChatChunk, ProviderError>>,
    ) -> Result<(), ProviderError> {
        if self.api_key.is_none() {
            return Err(ProviderError::AuthenticationFailed);
        }
        Ok(())
    }
}

/// Adapter for Anthropic Claude API.
pub struct AnthropicAdapter {
    pub api_key: Option<SecretString>,
    pub model: String,
}

impl AnthropicAdapter {
    pub fn new(api_key: Option<SecretString>, model: impl Into<String>) -> Self {
        Self {
            api_key,
            model: model.into(),
        }
    }
}

#[async_trait]
impl ChatProvider for AnthropicAdapter {
    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            streaming: true,
            tool_calling: true,
            structured_output: true,
            vision_input: true,
            image_generation: false,
            model_discovery: false,
        }
    }

    fn name(&self) -> &'static str {
        "anthropic"
    }

    async fn stream_chat(
        &self,
        _request: ChatRequest,
        _sender: mpsc::Sender<Result<ChatChunk, ProviderError>>,
    ) -> Result<(), ProviderError> {
        if self.api_key.is_none() {
            return Err(ProviderError::AuthenticationFailed);
        }
        Ok(())
    }
}

/// Adapter for Google Gemini API.
pub struct GeminiAdapter {
    pub api_key: Option<SecretString>,
    pub model: String,
}

impl GeminiAdapter {
    pub fn new(api_key: Option<SecretString>, model: impl Into<String>) -> Self {
        Self {
            api_key,
            model: model.into(),
        }
    }
}

#[async_trait]
impl ChatProvider for GeminiAdapter {
    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            streaming: true,
            tool_calling: true,
            structured_output: true,
            vision_input: true,
            image_generation: true,
            model_discovery: false,
        }
    }

    fn name(&self) -> &'static str {
        "gemini"
    }

    async fn stream_chat(
        &self,
        _request: ChatRequest,
        _sender: mpsc::Sender<Result<ChatChunk, ProviderError>>,
    ) -> Result<(), ProviderError> {
        if self.api_key.is_none() {
            return Err(ProviderError::AuthenticationFailed);
        }
        Ok(())
    }
}
