//! # OpenPet Render
//!
//! Frame scheduling, sprite atlases, dynamic frame rate scaling, and compact alpha hit-testing.

pub mod atlas;
pub mod hitmask;
pub mod mimi;
pub mod realistic;
pub mod scheduler;

pub use atlas::*;
pub use hitmask::*;
pub use mimi::*;
pub use realistic::*;
pub use scheduler::*;
