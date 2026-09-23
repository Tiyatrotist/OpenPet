use openpet_types::{AnimationCommand, InteractionType, PetState, Reminder, ScreenContext};
use tokio::sync::broadcast;
use uuid::Uuid;

/// Core domain lifecycle and asynchronous notification events.
#[derive(Debug, Clone, PartialEq)]
pub enum AppEvent {
    UserInteraction(InteractionType),
    PetStateChanged(PetState),
    AnimationRequested(AnimationCommand),
    ReminderDue(Reminder),
    ScreenContextChanged(ScreenContext),
    ChatIntent {
        conversation_id: Uuid,
        user_message: String,
    },
    SystemSuspend,
    SystemResume,
    MonitorConfigurationChanged,
    ShutdownRequested,
}

/// Bounded multi-producer, multi-consumer event hub for decoupling host subsystems.
#[derive(Clone)]
pub struct EventBus {
    sender: broadcast::Sender<AppEvent>,
}

impl EventBus {
    /// Constructs a bounded bus with fixed capacity (prevents unbounded memory growth).
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    /// Default capacity event hub.
    pub fn default_bus() -> Self {
        Self::new(256)
    }

    /// Broadcast an event across all active listeners.
    pub fn publish(&self, event: AppEvent) -> Result<usize, broadcast::error::SendError<AppEvent>> {
        self.sender.send(event)
    }

    /// Subscribe to receiving asynchronous application events.
    pub fn subscribe(&self) -> broadcast::Receiver<AppEvent> {
        self.sender.subscribe()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::default_bus()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_event_bus_broadcast() {
        let bus = EventBus::new(16);
        let mut rx = bus.subscribe();

        bus.publish(AppEvent::ShutdownRequested).unwrap();
        let received = rx.recv().await.unwrap();
        assert_eq!(received, AppEvent::ShutdownRequested);
    }
}
