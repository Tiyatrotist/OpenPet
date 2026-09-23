use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Recurrence cadence for desktop reminders.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecurrenceRule {
    Once,
    Daily,
    Weekly,
    Hourly,
}

/// A desktop reminder managed by OpenPet.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Reminder {
    pub id: Uuid,
    pub title: String,
    pub body: String,
    /// Scheduled trigger time in UTC
    pub schedule: DateTime<Utc>,
    /// Recurrence pattern
    pub recurrence: RecurrenceRule,
    /// Active flag
    pub enabled: bool,
    /// Creation time
    pub created_at: DateTime<Utc>,
    /// Last time this reminder fired
    pub last_fired_at: Option<DateTime<Utc>>,
}

impl Reminder {
    pub fn new(
        title: impl Into<String>,
        body: impl Into<String>,
        schedule: DateTime<Utc>,
        recurrence: RecurrenceRule,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            title: title.into().trim().to_string(),
            body: body.into().trim().to_string(),
            schedule,
            recurrence,
            enabled: true,
            created_at: Utc::now(),
            last_fired_at: None,
        }
    }

    /// Checks if the reminder is due relative to a given timestamp.
    pub fn is_due(&self, now: DateTime<Utc>) -> bool {
        self.enabled && self.schedule <= now
    }
}

/// Record of an executed reminder event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReminderRun {
    pub id: Uuid,
    pub reminder_id: Uuid,
    pub fired_at: DateTime<Utc>,
    pub acknowledged_at: Option<DateTime<Utc>>,
}
