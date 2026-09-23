#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayAction {
    OpenControlCenter,
    TogglePetVisibility,
    OpenChat,
    TogglePrivacyMode,
    TogglePausePet,
    OpenSettings,
    ExitApplication,
}
