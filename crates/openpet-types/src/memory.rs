use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A structured memory fact representing user-associated context learned by OpenPet.
///
/// OpenPet stores context as transparent, structured, user-auditable subject-predicate-object
/// facts rather than opaque, raw, unredacted chat dumps.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryFact {
    /// Unique fact identifier
    pub id: Uuid,
    /// The subject entity (e.g. "user", "pet", "favorite_editor")
    pub subject: String,
    /// The relational predicate (e.g. "likes", "prefers", "has_birthday")
    pub predicate: String,
    /// The factual object value (e.g. "hot chocolate", "rust", "october 15")
    pub object: String,
    /// Optional reference to message that originated this fact
    pub source_message_id: Option<Uuid>,
    /// Confidence score between 0.0 and 1.0
    pub confidence: f32,
    /// If true, automatic AI pruning cannot delete or alter this fact
    pub user_locked: bool,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

impl MemoryFact {
    /// Constructs a new unlocked memory fact with current timestamp.
    pub fn new(
        subject: impl Into<String>,
        predicate: impl Into<String>,
        object: impl Into<String>,
        confidence: f32,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            subject: subject.into().trim().to_lowercase(),
            predicate: predicate.into().trim().to_lowercase(),
            object: object.into().trim().to_string(),
            source_message_id: None,
            confidence: confidence.clamp(0.0, 1.0),
            user_locked: false,
            created_at: now,
            updated_at: now,
        }
    }

    /// Formats the fact as a clean sentence representation.
    pub fn display_summary(&self) -> String {
        format!("{} {} {}", self.subject, self.predicate, self.object)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_fact_new() {
        let fact = MemoryFact::new("User", "Likes", "Tea", 0.95);
        assert_eq!(fact.subject, "user");
        assert_eq!(fact.predicate, "likes");
        assert_eq!(fact.object, "Tea");
        assert!(!fact.user_locked);
    }
}
