use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Durable state machine progression for generating a pet from user photos.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GenerationJobState {
    Pending,
    Validating,
    Segmenting,
    Generating,
    Scoring,
    PreviewReady,
    Building,
    Complete,
    Failed,
    Cancelled,
}

/// Durable job description for a pet generation task.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GenerationJob {
    pub id: Uuid,
    pub pet_name: String,
    pub state: GenerationJobState,
    pub progress: f32,
    pub input_photo_count: usize,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl GenerationJob {
    pub fn new(pet_name: impl Into<String>, photo_count: usize) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            pet_name: pet_name.into(),
            state: GenerationJobState::Pending,
            progress: 0.0,
            input_photo_count: photo_count,
            error_message: None,
            created_at: now,
            updated_at: now,
        }
    }
}
