//! # OpenPet Platform Windows
//!
//! Windows-specific APIs: Mutex single-instance enforcement, Per-Monitor DPI awareness V2,
//! toast notifications, and tray integration.

pub mod dpi;
pub mod notification;
pub mod singleton;
pub mod tray;

pub use dpi::*;
pub use notification::*;
pub use singleton::*;
pub use tray::*;
