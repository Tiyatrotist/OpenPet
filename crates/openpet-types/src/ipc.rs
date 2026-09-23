use crate::ai::{ChatMessage, ProviderConfig, ToolCallProposal};
use crate::behavior::{AnimationCommand, InteractionType};
use crate::memory::MemoryFact;
use crate::pet::{PetId, PetMetadata, PetState};
use crate::plugin::PluginMetadata;
use crate::reminder::Reminder;
use crate::screen::ScreenContext;
use crate::settings::AppSettings;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// IPC Handshake initiated by client (Control Center).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientHello {
    pub protocol_version: u32,
    pub client_version: String,
    pub client_kind: String,
}

/// IPC Handshake response from OpenPet Host.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerHello {
    pub protocol_version: u32,
    pub host_version: String,
    pub capabilities: Vec<String>,
}

/// Commands and queries sent from Control Center to OpenPet Host.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum IpcRequest {
    Handshake(ClientHello),
    ListPets,
    GetActivePet,
    SetActivePet(PetId),
    GetPetState,
    InteractPet(InteractionType),
    ExecuteAnimationCommand(AnimationCommand),
    GetSettings,
    UpdateSettings(AppSettings),
    ListMemories,
    CreateMemory {
        subject: String,
        predicate: String,
        object: String,
        confidence: f32,
    },
    DeleteMemory(Uuid),
    SetMemoryLocked {
        id: Uuid,
        locked: bool,
    },
    ListReminders,
    CreateReminder {
        title: String,
        body: String,
        schedule_utc: chrono::DateTime<chrono::Utc>,
        recurrence: crate::reminder::RecurrenceRule,
    },
    DeleteReminder(Uuid),
    ConfirmToolCall {
        proposal_id: String,
        approved: bool,
    },
    SendChatMessage {
        conversation_id: Uuid,
        content: String,
    },
    ListProviders,
    SaveProviderConfig(ProviderConfig),
    SetPrivacyMode(bool),
    ListPlugins,
    Ping,
    Shutdown,
}

/// Responses sent from OpenPet Host to Control Center.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum IpcResponse {
    Handshake(ServerHello),
    Pets(Vec<PetMetadata>),
    ActivePet(Option<PetMetadata>),
    PetState(PetState),
    Settings(AppSettings),
    Memories(Vec<MemoryFact>),
    Reminders(Vec<Reminder>),
    ChatMessage(ChatMessage),
    ChatStreamChunk {
        conversation_id: Uuid,
        delta: String,
        is_final: bool,
    },
    ToolProposal(ToolCallProposal),
    Providers(Vec<ProviderConfig>),
    Plugins(Vec<PluginMetadata>),
    Pong,
    Ack,
    Error(String),
}

/// Asynchronous server push events sent from Host to Control Center.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum IpcEvent {
    PetStateChanged(PetState),
    ReminderDue(Reminder),
    ScreenContextUpdated(ScreenContext),
    Notification { title: String, message: String },
}
