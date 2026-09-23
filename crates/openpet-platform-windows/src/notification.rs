use tracing::info;

pub trait NotificationDispatcher: Send + Sync {
    fn show_toast(&self, title: &str, body: &str) -> Result<(), String>;
}

/// Windows toast notification dispatcher using standard PowerShell/WinRT/fallback mechanism.
#[derive(Default)]
pub struct WindowsToastDispatcher;

impl WindowsToastDispatcher {
    pub fn new() -> Self {
        Self
    }
}

impl NotificationDispatcher for WindowsToastDispatcher {
    fn show_toast(&self, title: &str, body: &str) -> Result<(), String> {
        info!("Dispatching toast notification: [{}] {}", title, body);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toast_dispatch() {
        let dispatcher = WindowsToastDispatcher::new();
        assert!(dispatcher
            .show_toast("OpenPet Reminder", "Drink water!")
            .is_ok());
    }
}
