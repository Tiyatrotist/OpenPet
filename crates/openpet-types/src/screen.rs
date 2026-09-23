use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// High-level derived activity categorization inferred locally on the machine.
///
/// Raw frame data is immediately discarded after classification and NEVER exposed
/// to AI providers, IPC streams, disk, or logs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ScreenActivity {
    #[default]
    Idle,
    Writing,
    Meeting,
    Video,
    Unknown,
}

/// Abstract derived screen context signal presented to the pet behavior engine.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScreenContext {
    pub activity: ScreenActivity,
    pub confidence: f32,
    pub timestamp: DateTime<Utc>,
}

impl Default for ScreenContext {
    fn default() -> Self {
        Self {
            activity: ScreenActivity::Idle,
            confidence: 1.0,
            timestamp: Utc::now(),
        }
    }
}
