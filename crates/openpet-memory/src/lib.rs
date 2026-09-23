//! # OpenPet Memory Service
//!
//! Structured fact management, search, and context formatting for OpenPet.
//! Ensures all long-term memories are user-auditable, editable, deletable, and lockable.

use openpet_storage::Database;
use openpet_types::MemoryFact;
use std::sync::Arc;
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum MemoryServiceError {
    #[error("Storage error: {0}")]
    Storage(#[from] openpet_storage::StorageError),
}

pub struct MemoryService {
    db: Arc<Database>,
}

impl MemoryService {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    pub fn remember(
        &self,
        subject: impl Into<String>,
        predicate: impl Into<String>,
        object: impl Into<String>,
        confidence: f32,
    ) -> Result<MemoryFact, MemoryServiceError> {
        let fact = MemoryFact::new(subject, predicate, object, confidence);
        self.db.insert_memory_fact(&fact)?;
        Ok(fact)
    }

    pub fn list_all(&self) -> Result<Vec<MemoryFact>, MemoryServiceError> {
        Ok(self.db.list_memory_facts()?)
    }

    pub fn search(&self, query: &str) -> Result<Vec<MemoryFact>, MemoryServiceError> {
        Ok(self.db.search_memory_facts(query)?)
    }

    pub fn forget(&self, id: Uuid) -> Result<(), MemoryServiceError> {
        Ok(self.db.delete_memory_fact(id)?)
    }

    pub fn set_locked(&self, id: Uuid, locked: bool) -> Result<(), MemoryServiceError> {
        Ok(self.db.set_memory_fact_locked(id, locked)?)
    }

    /// Generates compact factual context for injecting into assistant prompts.
    pub fn build_chat_context(&self, relevant_query: &str) -> String {
        let facts = if relevant_query.trim().is_empty() {
            self.list_all().unwrap_or_default()
        } else {
            self.search(relevant_query).unwrap_or_default()
        };

        if facts.is_empty() {
            return String::new();
        }

        let mut lines = Vec::new();
        lines.push("Known user context:".to_string());
        for f in facts.iter().take(10) {
            lines.push(format!("- {}", f.display_summary()));
        }
        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_service_remember_and_build_context() {
        let db = Arc::new(Database::open_in_memory().unwrap());
        let service = MemoryService::new(db);

        service
            .remember("User", "Favorite beverage is", "Green tea", 0.95)
            .unwrap();
        let context = service.build_chat_context("tea");
        assert!(context.contains("Green tea"));

        let all = service.list_all().unwrap();
        assert_eq!(all.len(), 1);

        service.forget(all[0].id).unwrap();
        assert_eq!(service.list_all().unwrap().len(), 0);
    }
}
