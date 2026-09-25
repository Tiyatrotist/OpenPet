//! # OpenPet Reminders
//!
//! Desktop reminder scheduler, sleep/resume overdue evaluation, and confirmed tool execution gate.

use chrono::{DateTime, Duration, Utc};
use openpet_platform_windows::{NotificationDispatcher, WindowsToastDispatcher};
use openpet_storage::Database;
use openpet_types::{RecurrenceRule, Reminder, ToolCallProposal};
use std::sync::Arc;
use thiserror::Error;
use tracing::info;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum ReminderError {
    #[error("Storage error: {0}")]
    Storage(#[from] openpet_storage::StorageError),
    #[error("Security violation: tool call '{0}' was rejected or unconfirmed by user")]
    ToolConfirmationRequired(String),
}

pub struct ReminderService {
    db: Arc<Database>,
    notifier: Box<dyn NotificationDispatcher>,
}

impl ReminderService {
    pub fn new(db: Arc<Database>) -> Self {
        Self {
            db,
            notifier: Box::new(WindowsToastDispatcher::new()),
        }
    }

    pub fn schedule_reminder(
        &self,
        title: impl Into<String>,
        body: impl Into<String>,
        schedule: DateTime<Utc>,
        recurrence: RecurrenceRule,
    ) -> Result<Reminder, ReminderError> {
        let reminder = Reminder::new(title, body, schedule, recurrence);
        self.db.insert_reminder(&reminder)?;
        Ok(reminder)
    }

    pub fn list_reminders(&self) -> Result<Vec<Reminder>, ReminderError> {
        Ok(self.db.list_reminders()?)
    }

    pub fn cancel_reminder(&self, id: Uuid) -> Result<(), ReminderError> {
        Ok(self.db.delete_reminder(id)?)
    }

    pub fn toggle_reminder(&self, id: Uuid) -> Result<bool, ReminderError> {
        Ok(self.db.toggle_reminder(id)?)
    }

    /// Evaluates all active reminders, fires due notifications, and reschedules recurrences.
    pub fn evaluate_due_reminders(
        &self,
        now: DateTime<Utc>,
    ) -> Result<Vec<Reminder>, ReminderError> {
        let reminders = self.db.list_reminders()?;
        let mut fired = Vec::new();

        for mut r in reminders {
            if r.is_due(now) {
                info!("Reminder due: [{}] {}", r.title, r.body);
                let _ = self.notifier.show_toast(&r.title, &r.body);
                r.last_fired_at = Some(now);

                // Update schedule based on recurrence, catching up overdue intervals to prevent notification storms
                match r.recurrence {
                    RecurrenceRule::Once => {
                        r.enabled = false;
                    }
                    RecurrenceRule::Daily => {
                        while r.schedule <= now {
                            r.schedule += Duration::days(1);
                        }
                    }
                    RecurrenceRule::Weekly => {
                        while r.schedule <= now {
                            r.schedule += Duration::weeks(1);
                        }
                    }
                    RecurrenceRule::Hourly => {
                        while r.schedule <= now {
                            r.schedule += Duration::hours(1);
                        }
                    }
                }

                self.db.insert_reminder(&r)?;
                fired.push(r);
            }
        }

        Ok(fired)
    }

    /// Called on system resume from sleep/hibernate to process overdue tasks gracefully.
    pub fn handle_system_resume(&self) -> Result<Vec<Reminder>, ReminderError> {
        let now = Utc::now();
        info!("System resumed: evaluating overdue reminders at {}", now);
        self.evaluate_due_reminders(now)
    }

    /// Gated execution of an LLM tool proposal.
    ///
    /// CRITICAL SECURITY RULE:
    /// LLM can never mutate reminders unless `proposal.user_confirmed` is explicitly true!
    pub fn execute_tool_proposal(
        &self,
        proposal: &ToolCallProposal,
    ) -> Result<String, ReminderError> {
        if !proposal.user_confirmed {
            return Err(ReminderError::ToolConfirmationRequired(
                proposal.name.clone(),
            ));
        }

        match proposal.name.as_str() {
            "reminder.create" => {
                #[derive(serde::Deserialize)]
                struct CreateArgs {
                    title: String,
                    body: String,
                    minutes_from_now: u32,
                }

                let args: CreateArgs = serde_json::from_str(&proposal.arguments_json)
                    .map_err(|e| ReminderError::ToolConfirmationRequired(e.to_string()))?;

                let schedule = Utc::now() + Duration::minutes(args.minutes_from_now as i64);
                let reminder =
                    self.schedule_reminder(args.title, args.body, schedule, RecurrenceRule::Once)?;
                Ok(format!("Reminder created with ID: {}", reminder.id))
            }
            "reminder.delete" => {
                #[derive(serde::Deserialize)]
                struct DeleteArgs {
                    id: String,
                }
                let args: DeleteArgs = serde_json::from_str(&proposal.arguments_json)
                    .map_err(|e| ReminderError::ToolConfirmationRequired(e.to_string()))?;

                if let Ok(id) = Uuid::parse_str(&args.id) {
                    self.cancel_reminder(id)?;
                    Ok(format!("Reminder {} deleted", id))
                } else {
                    Err(ReminderError::ToolConfirmationRequired(
                        "Invalid UUID".into(),
                    ))
                }
            }
            other => Err(ReminderError::ToolConfirmationRequired(format!(
                "Unknown tool: {}",
                other
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unconfirmed_tool_call_fails() {
        let db = Arc::new(Database::open_in_memory().unwrap());
        let service = ReminderService::new(db);

        let unconfirmed = ToolCallProposal {
            id: "call_1".into(),
            name: "reminder.create".into(),
            arguments_json: r#"{"title":"Walk","body":"Go outside","minutes_from_now":10}"#.into(),
            requires_user_confirmation: true,
            user_confirmed: false, // NOT confirmed!
        };

        let result = service.execute_tool_proposal(&unconfirmed);
        assert!(matches!(
            result,
            Err(ReminderError::ToolConfirmationRequired(_))
        ));
    }

    #[test]
    fn test_confirmed_tool_call_succeeds() {
        let db = Arc::new(Database::open_in_memory().unwrap());
        let service = ReminderService::new(db);

        let confirmed = ToolCallProposal {
            id: "call_2".into(),
            name: "reminder.create".into(),
            arguments_json: r#"{"title":"Walk","body":"Go outside","minutes_from_now":10}"#.into(),
            requires_user_confirmation: true,
            user_confirmed: true, // User clicked confirm!
        };

        let result = service.execute_tool_proposal(&confirmed);
        assert!(result.is_ok());

        let list = service.list_reminders().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].title, "Walk");
    }

    #[test]
    fn test_overdue_recurring_reminder_catches_up_past_now() {
        let db = Arc::new(Database::open_in_memory().unwrap());
        let service = ReminderService::new(db);

        // Schedule daily reminder 3 days ago (e.g. system was asleep over weekend)
        let three_days_ago = Utc::now() - Duration::days(3);
        let reminder = service
            .schedule_reminder(
                "Feed fish",
                "Aquarium",
                three_days_ago,
                RecurrenceRule::Daily,
            )
            .unwrap();

        let fired = service.evaluate_due_reminders(Utc::now()).unwrap();
        assert_eq!(fired.len(), 1);
        assert_eq!(fired[0].id, reminder.id);
        // The new schedule MUST be strictly in the future relative to now!
        assert!(fired[0].schedule > Utc::now());

        // A subsequent evaluation immediately afterwards should NOT fire again!
        let second_check = service.evaluate_due_reminders(Utc::now()).unwrap();
        assert_eq!(second_check.len(), 0);
    }

    #[test]
    fn test_reminder_toggle() {
        let db = Arc::new(Database::open_in_memory().unwrap());
        let service = ReminderService::new(db);

        let rem = service
            .schedule_reminder(
                "Cat Nap",
                "Take a rest",
                Utc::now() + Duration::hours(1),
                RecurrenceRule::Once,
            )
            .unwrap();
        assert!(rem.enabled);

        let active = service.toggle_reminder(rem.id).unwrap();
        assert!(!active);

        let list = service.list_reminders().unwrap();
        assert!(!list[0].enabled);

        let active_again = service.toggle_reminder(rem.id).unwrap();
        assert!(active_again);

        let list_again = service.list_reminders().unwrap();
        assert!(list_again[0].enabled);
    }
}
