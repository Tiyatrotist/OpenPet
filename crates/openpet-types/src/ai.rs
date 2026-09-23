use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

/// Available generative AI & LLM provider backends.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    OpenAi,
    Anthropic,
    Gemini,
    Ollama,
    ComfyUi,
    Custom,
}

impl ProviderKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::OpenAi => "openai",
            Self::Anthropic => "anthropic",
            Self::Gemini => "gemini",
            Self::Ollama => "ollama",
            Self::ComfyUi => "comfyui",
            Self::Custom => "custom",
        }
    }
}

/// Provider operational feature matrix.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderCapabilities {
    pub streaming: bool,
    pub tool_calling: bool,
    pub structured_output: bool,
    pub vision_input: bool,
    pub image_generation: bool,
    pub model_discovery: bool,
}

impl Default for ProviderCapabilities {
    fn default() -> Self {
        Self {
            streaming: true,
            tool_calling: false,
            structured_output: false,
            vision_input: false,
            image_generation: false,
            model_discovery: false,
        }
    }
}

/// Normalized provider configuration.
///
/// NOTE: Passwords and API tokens MUST NOT be stored in this struct;
/// they are isolated inside the Windows Credential Manager / secrets vault.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub kind: ProviderKind,
    pub base_url: Option<String>,
    pub model_name: String,
    pub is_enabled: bool,
    pub has_credential: bool,
    pub timeout_seconds: u32,
}

impl ProviderConfig {
    pub fn new_ollama(model: impl Into<String>) -> Self {
        Self {
            kind: ProviderKind::Ollama,
            base_url: Some("http://localhost:11434".to_string()),
            model_name: model.into(),
            is_enabled: true,
            has_credential: true, // Local Ollama does not require secret token
            timeout_seconds: 60,
        }
    }
}

/// Role in a chat conversation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChatRole {
    System,
    User,
    Assistant,
    Tool,
}

/// Single conversational chat message.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: Uuid,
    pub conversation_id: Uuid,
    pub role: ChatRole,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub tool_call_id: Option<String>,
}

impl ChatMessage {
    pub fn user(conversation_id: Uuid, content: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            conversation_id,
            role: ChatRole::User,
            content: content.into(),
            timestamp: Utc::now(),
            tool_call_id: None,
        }
    }

    pub fn assistant(conversation_id: Uuid, content: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            conversation_id,
            role: ChatRole::Assistant,
            content: content.into(),
            timestamp: Utc::now(),
            tool_call_id: None,
        }
    }
}

/// Structured tool invocation proposed by an LLM.
///
/// CRITICAL: LLM tools CANNOT execute mutations without explicit user consent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolCallProposal {
    pub id: String,
    pub name: String,
    pub arguments_json: String,
    pub requires_user_confirmation: bool,
    pub user_confirmed: bool,
}

/// Standardized provider error classifications across all backends.
#[derive(Error, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProviderError {
    #[error("Authentication failed: invalid or missing API key")]
    AuthenticationFailed,

    #[error("Rate limit reached: provider requested backoff")]
    RateLimited { retry_after_secs: Option<u64> },

    #[error("Network connection error: {0}")]
    Network(String),

    #[error("Request timed out after {0} seconds")]
    Timeout(u32),

    #[error("Invalid request parameters: {0}")]
    InvalidRequest(String),

    #[error("Unsupported capability for this provider")]
    UnsupportedCapability,

    #[error("Malformed response from provider: {0}")]
    MalformedResponse(String),

    #[error("Operation was cancelled by user")]
    Cancelled,

    #[error("Provider is currently unavailable")]
    ProviderUnavailable,

    #[error("Other provider error: {0}")]
    Other(String),
}
