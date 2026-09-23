//! # OpenPet Plugin API & Stable ABI v1
//!
//! Semantic commands, capability requests, and snapshot contracts for Wasm plugins.

use openpet_types::{BehaviorType, PetEmote, PluginCapability};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const PLUGIN_ABI_VERSION: u32 = 1;

#[derive(Error, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PluginError {
    #[error("Capability not granted: {0}")]
    CapabilityDenied(String),
    #[error("Plugin exceeded memory fuel budget")]
    OutOfFuel,
    #[error("Plugin timed out")]
    Timeout,
    #[error("Execution error: {0}")]
    Execution(String),
}

/// Immutable snapshot of pet state provided to plugins with `pet.read_state` capability.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PetStateSnapshot {
    pub mood: f32,
    pub energy: f32,
    pub curiosity: f32,
    pub bond: f32,
    pub current_behavior: String,
}

/// Semantic command proposed by a plugin to influence pet desktop behavior.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum PetCommand {
    PlayAnimation { behavior: BehaviorType, loops: u32 },
    ShowEmote(PetEmote),
    RequestInteraction { name: String },
    TemporaryMoodModifier { delta: f32 },
    DispatchNotification { title: String, message: String },
}

/// Plugin declaration exported by sandboxed components.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub capabilities: Vec<PluginCapability>,
}
