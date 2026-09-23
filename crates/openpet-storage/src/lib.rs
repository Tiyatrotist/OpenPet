//! # OpenPet Storage
//!
//! Encrypted/local SQLite persistence layer, schema migrations, and structured facts queries.

pub mod migrations;

use chrono::{DateTime, Utc};
use migrations::run_migrations;
use openpet_types::{AppSettings, MemoryFact, PetId, PetMetadata, RecurrenceRule, Reminder};
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::Mutex;
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("Database error: {0}")]
    Sql(#[from] rusqlite::Error),
    #[error("Serialization error: {0}")]
    Json(#[from] serde_json::Error),
}

pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    /// Opens or creates database at specified path and applies pending migrations.
    pub fn open(path: &Path) -> Result<Self, StorageError> {
        let mut conn = Connection::open(path)?;
        run_migrations(&mut conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Creates an in-memory database for isolated testing.
    pub fn open_in_memory() -> Result<Self, StorageError> {
        let mut conn = Connection::open_in_memory()?;
        run_migrations(&mut conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn save_settings(&self, settings: &AppSettings) -> Result<(), StorageError> {
        let conn = self.conn.lock().unwrap();
        let json = serde_json::to_string(settings)?;
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value_json, updated_at) VALUES ('app_settings', ?1, ?2)",
            params![json, Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }

    pub fn load_settings(&self) -> Result<Option<AppSettings>, StorageError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt =
            conn.prepare("SELECT value_json FROM settings WHERE key = 'app_settings'")?;
        let mut rows = stmt.query([])?;

        if let Some(row) = rows.next()? {
            let json: String = row.get(0)?;
            let settings: AppSettings = serde_json::from_str(&json)?;
            Ok(Some(settings))
        } else {
            Ok(None)
        }
    }

    pub fn insert_memory_fact(&self, fact: &MemoryFact) -> Result<(), StorageError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO memory_facts (id, subject, predicate, object, source_message_id, confidence, user_locked, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                fact.id.to_string(),
                fact.subject,
                fact.predicate,
                fact.object,
                fact.source_message_id.map(|u| u.to_string()),
                fact.confidence,
                if fact.user_locked { 1 } else { 0 },
                fact.created_at.to_rfc3339(),
                fact.updated_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    pub fn list_memory_facts(&self) -> Result<Vec<MemoryFact>, StorageError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, subject, predicate, object, source_message_id, confidence, user_locked, created_at, updated_at
             FROM memory_facts ORDER BY updated_at DESC",
        )?;

        let rows = stmt.query_map([], |row| {
            let id_str: String = row.get(0)?;
            let source_str: Option<String> = row.get(4)?;
            let locked_int: i32 = row.get(6)?;
            let created_str: String = row.get(7)?;
            let updated_str: String = row.get(8)?;

            Ok(MemoryFact {
                id: Uuid::parse_str(&id_str).unwrap_or_default(),
                subject: row.get(1)?,
                predicate: row.get(2)?,
                object: row.get(3)?,
                source_message_id: source_str.and_then(|s| Uuid::parse_str(&s).ok()),
                confidence: row.get(5)?,
                user_locked: locked_int != 0,
                created_at: DateTime::parse_from_rfc3339(&created_str)
                    .map(|d| d.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                updated_at: DateTime::parse_from_rfc3339(&updated_str)
                    .map(|d| d.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
            })
        })?;

        let mut results = Vec::new();
        for r in rows {
            results.push(r?);
        }
        Ok(results)
    }

    pub fn delete_memory_fact(&self, id: Uuid) -> Result<(), StorageError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM memory_facts WHERE id = ?1",
            params![id.to_string()],
        )?;
        Ok(())
    }

    pub fn set_memory_fact_locked(&self, id: Uuid, locked: bool) -> Result<(), StorageError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE memory_facts SET user_locked = ?1, updated_at = ?2 WHERE id = ?3",
            params![
                if locked { 1 } else { 0 },
                Utc::now().to_rfc3339(),
                id.to_string()
            ],
        )?;
        Ok(())
    }

    pub fn search_memory_facts(&self, query: &str) -> Result<Vec<MemoryFact>, StorageError> {
        let conn = self.conn.lock().unwrap();
        let pattern = format!("%{}%", query.trim().to_lowercase());
        let mut stmt = conn.prepare(
            "SELECT id, subject, predicate, object, source_message_id, confidence, user_locked, created_at, updated_at
             FROM memory_facts
             WHERE LOWER(subject) LIKE ?1 OR LOWER(predicate) LIKE ?1 OR LOWER(object) LIKE ?1
             ORDER BY updated_at DESC",
        )?;

        let rows = stmt.query_map(params![pattern], |row| {
            let id_str: String = row.get(0)?;
            let source_str: Option<String> = row.get(4)?;
            let locked_int: i32 = row.get(6)?;
            let created_str: String = row.get(7)?;
            let updated_str: String = row.get(8)?;

            Ok(MemoryFact {
                id: Uuid::parse_str(&id_str).unwrap_or_default(),
                subject: row.get(1)?,
                predicate: row.get(2)?,
                object: row.get(3)?,
                source_message_id: source_str.and_then(|s| Uuid::parse_str(&s).ok()),
                confidence: row.get(5)?,
                user_locked: locked_int != 0,
                created_at: DateTime::parse_from_rfc3339(&created_str)
                    .map(|d| d.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                updated_at: DateTime::parse_from_rfc3339(&updated_str)
                    .map(|d| d.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
            })
        })?;

        let mut results = Vec::new();
        for r in rows {
            results.push(r?);
        }
        Ok(results)
    }

    pub fn insert_reminder(&self, reminder: &Reminder) -> Result<(), StorageError> {
        let conn = self.conn.lock().unwrap();
        let rec_str = match reminder.recurrence {
            RecurrenceRule::Once => "once",
            RecurrenceRule::Daily => "daily",
            RecurrenceRule::Weekly => "weekly",
            RecurrenceRule::Hourly => "hourly",
        };

        conn.execute(
            "INSERT OR REPLACE INTO reminders (id, title, body, schedule, recurrence, enabled, created_at, last_fired_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                reminder.id.to_string(),
                reminder.title,
                reminder.body,
                reminder.schedule.to_rfc3339(),
                rec_str,
                if reminder.enabled { 1 } else { 0 },
                reminder.created_at.to_rfc3339(),
                reminder.last_fired_at.map(|d| d.to_rfc3339()),
            ],
        )?;
        Ok(())
    }

    pub fn list_reminders(&self) -> Result<Vec<Reminder>, StorageError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, title, body, schedule, recurrence, enabled, created_at, last_fired_at
             FROM reminders ORDER BY schedule ASC",
        )?;

        let rows = stmt.query_map([], |row| {
            let id_str: String = row.get(0)?;
            let sched_str: String = row.get(3)?;
            let rec_str: String = row.get(4)?;
            let enabled_int: i32 = row.get(5)?;
            let created_str: String = row.get(6)?;
            let last_fired_str: Option<String> = row.get(7)?;

            let recurrence = match rec_str.as_str() {
                "daily" => RecurrenceRule::Daily,
                "weekly" => RecurrenceRule::Weekly,
                "hourly" => RecurrenceRule::Hourly,
                _ => RecurrenceRule::Once,
            };

            Ok(Reminder {
                id: Uuid::parse_str(&id_str).unwrap_or_default(),
                title: row.get(1)?,
                body: row.get(2)?,
                schedule: DateTime::parse_from_rfc3339(&sched_str)
                    .map(|d| d.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                recurrence,
                enabled: enabled_int != 0,
                created_at: DateTime::parse_from_rfc3339(&created_str)
                    .map(|d| d.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                last_fired_at: last_fired_str.and_then(|s| {
                    DateTime::parse_from_rfc3339(&s)
                        .map(|d| d.with_timezone(&Utc))
                        .ok()
                }),
            })
        })?;

        let mut results = Vec::new();
        for r in rows {
            results.push(r?);
        }
        Ok(results)
    }

    pub fn delete_reminder(&self, id: Uuid) -> Result<(), StorageError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM reminders WHERE id = ?1",
            params![id.to_string()],
        )?;
        Ok(())
    }

    pub fn save_pet(&self, pet: &PetMetadata) -> Result<(), StorageError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO pets (id, name, version, author, description, license, homepage, created_with, provenance, min_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                pet.id.0,
                pet.name,
                pet.version,
                pet.author,
                pet.description,
                pet.license,
                pet.homepage,
                pet.created_with,
                pet.source_provenance,
                pet.minimum_openpet_version,
            ],
        )?;
        Ok(())
    }

    pub fn list_pets(&self) -> Result<Vec<PetMetadata>, StorageError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, version, author, description, license, homepage, created_with, provenance, min_version
             FROM pets",
        )?;

        let rows = stmt.query_map([], |row| {
            let id_str: String = row.get(0)?;
            Ok(PetMetadata {
                id: PetId::new(id_str),
                name: row.get(1)?,
                version: row.get(2)?,
                author: row.get(3)?,
                description: row.get(4)?,
                license: row.get(5)?,
                homepage: row.get(6)?,
                created_with: row.get(7)?,
                source_provenance: row.get(8)?,
                minimum_openpet_version: row.get(9)?,
                tags: Vec::new(),
            })
        })?;

        let mut results = Vec::new();
        for r in rows {
            results.push(r?);
        }
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_crud() {
        let db = Database::open_in_memory().unwrap();

        // Settings test
        let settings = AppSettings {
            privacy_mode: true,
            ..Default::default()
        };
        db.save_settings(&settings).unwrap();

        let loaded = db.load_settings().unwrap().expect("Settings must exist");
        assert!(loaded.privacy_mode);

        // Memory facts test
        let fact = MemoryFact::new("User", "Loves", "Open Source", 0.99);
        db.insert_memory_fact(&fact).unwrap();

        let facts = db.list_memory_facts().unwrap();
        assert_eq!(facts.len(), 1);
        assert_eq!(facts[0].object, "Open Source");

        // Search test
        let found = db.search_memory_facts("source").unwrap();
        assert_eq!(found.len(), 1);

        // Delete test
        db.delete_memory_fact(fact.id).unwrap();
        assert_eq!(db.list_memory_facts().unwrap().len(), 0);
    }
}
