use serde::{Deserialize, Serialize};

/// Granular capability permissions requested by a sandboxed Wasm plugin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginCapability {
    PetReadState,
    PetCommand,
    Notifications,
    RemindersRead,
    RemindersWrite,
    MemoryRead,
}

impl PluginCapability {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::PetReadState => "pet.read_state",
            Self::PetCommand => "pet.command",
            Self::Notifications => "notifications",
            Self::RemindersRead => "reminders.read",
            Self::RemindersWrite => "reminders.write",
            Self::MemoryRead => "memory.read",
        }
    }
}

/// Metadata identifying a Wasm plugin.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginMetadata {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub requested_capabilities: Vec<PluginCapability>,
}
