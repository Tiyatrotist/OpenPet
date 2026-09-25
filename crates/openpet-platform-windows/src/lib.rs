//! # OpenPet Platform Windows
//!
//! Windows-specific APIs: Mutex single-instance enforcement, Per-Monitor DPI awareness V2,
//! toast notifications, and tray integration.

pub mod chat_window;
pub mod dock_3d;
pub mod dpi;
pub mod notification;
pub mod pet_window;
pub mod singleton;
pub mod tray;

pub use chat_window::*;
pub use dock_3d::*;
pub use dpi::*;
pub use notification::*;
pub use pet_window::*;
pub use singleton::*;
pub use tray::*;
