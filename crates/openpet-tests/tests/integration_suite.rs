use chrono::{Duration, Utc};
use openpet_behavior::BehaviorEngine;
use openpet_memory::MemoryService;
use openpet_reminders::ReminderService;
use openpet_storage::Database;
use openpet_types::{
    AppSettings, InteractionType, RecurrenceRule, ScreenPosition,
};
use std::sync::Arc;

#[test]
fn test_end_to_end_behavior_and_persistence() {
    let db = Arc::new(Database::open_in_memory().unwrap());

    // 1. Settings persistence
    let mut settings = AppSettings::default();
    settings.launch_on_startup = true;
    db.save_settings(&settings).unwrap();

    let loaded = db.load_settings().unwrap().expect("Settings should load");
    assert!(loaded.launch_on_startup);

    // 2. Behavior engine deterministic interactions
    let mut engine = BehaviorEngine::new_with_seed(42);

    // Drag sequence
    engine.handle_interaction(InteractionType::DragStart { x: 100.0, y: 200.0 });
    assert!(engine.is_dragging());
    assert_eq!(engine.position(), ScreenPosition::new(100.0, 200.0));

    // Petting increases bond and mood
    let initial_bond = engine.state().bond;
    engine.handle_interaction(InteractionType::Petting { intensity: 1.0 });
    assert!(engine.state().bond > initial_bond);
}

#[test]
fn test_end_to_end_memory_and_reminders() {
    let db = Arc::new(Database::open_in_memory().unwrap());
    let memory = MemoryService::new(db.clone());
    let reminders = ReminderService::new(db.clone());

    // Memory operations
    memory.remember("Owner", "Favorite drink is", "Espresso", 0.9).unwrap();
    let facts = memory.search("Espresso").unwrap();
    assert_eq!(facts.len(), 1);
    assert_eq!(facts[0].subject, "owner");

    // Reminder operations
    let due_time = Utc::now() - Duration::minutes(5); // already due
    let reminder = reminders
        .schedule_reminder("Water plants", "Front porch", due_time, RecurrenceRule::Daily)
        .unwrap();

    let fired = reminders.evaluate_due_reminders(Utc::now()).unwrap();
    assert_eq!(fired.len(), 1);
    assert_eq!(fired[0].id, reminder.id);
    // Recurrence should advance schedule by 1 day
    assert!(fired[0].schedule > Utc::now());
}
