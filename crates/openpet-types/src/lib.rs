//! # OpenPet Types
//!
//! Canonical domain models, shared contracts, IPC structures, and event definitions
//! for the OpenPet ecosystem. Designed to be strictly deterministic, serialization-safe,
//! and free of heavy runtime dependencies.

pub mod ai;
pub mod behavior;
pub mod generation;
pub mod ipc;
pub mod memory;
pub mod pet;
pub mod plugin;
pub mod reminder;
pub mod screen;
pub mod settings;

pub use ai::*;
pub use behavior::*;
pub use generation::*;
pub use ipc::*;
pub use memory::*;
pub use pet::*;
pub use plugin::*;
pub use reminder::*;
pub use screen::*;
pub use settings::*;
