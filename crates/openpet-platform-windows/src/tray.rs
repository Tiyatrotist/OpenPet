//! # Windows System Tray Integration
//!
//! Taskbar notification area icon with context menu for OpenPet companion management.

use openpet_i18n::I18nManager;
use openpet_types::SupportedLocale;
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use tracing::{debug, error, info};

/// Action dispatched when a user interacts with the system tray or pet context menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayAction {
    OpenControlCenter,
    TogglePetVisibility,
    TogglePrivacyMode,
    TogglePausePet,
    ToggleArtStyle,
    Toggle3DDock,
    OpenSettings,
    ExitApplication,
}

/// Menu command IDs used by the popup context menu.
pub const CMD_OPEN_CONTROL: usize = 1001;
pub const CMD_TOGGLE_PET: usize = 1002;
pub const CMD_PRIVACY_MODE: usize = 1003;
pub const CMD_PAUSE_PET: usize = 1004;
pub const CMD_SETTINGS: usize = 1005;
pub const CMD_EXIT: usize = 1006;
pub const CMD_TOGGLE_STYLE: usize = 1007;
pub const CMD_TOGGLE_3D_DOCK: usize = 1008;

/// Custom Windows message for tray callback events
pub const WM_TRAY_CALLBACK: u32 = 0x8000 + 101; // WM_APP + 101

/// Dynamic state controlling checked/unchecked menu options.
#[derive(Debug, Clone)]
pub struct TrayState {
    pub privacy_mode: bool,
    pub pet_paused: bool,
    pub pet_visible: bool,
    pub art_style: openpet_types::CompanionArtStyle,
    pub locale: SupportedLocale,
}

impl Default for TrayState {
    fn default() -> Self {
        Self {
            privacy_mode: false,
            pet_paused: false,
            pet_visible: true,
            art_style: openpet_types::CompanionArtStyle::PixelArt,
            locale: SupportedLocale::EnUs,
        }
    }
}

/// Manages the Windows taskbar notification area icon.
pub struct SystemTray {
    #[cfg(windows)]
    hwnd: windows_sys::Win32::Foundation::HWND,
    #[cfg(windows)]
    is_installed: bool,
    state: TrayState,
}

impl SystemTray {
    /// Creates and registers a new system tray icon for the given window handle.
    #[cfg(windows)]
    pub fn new(hwnd: windows_sys::Win32::Foundation::HWND, initial_state: TrayState) -> Self {
        use windows_sys::Win32::UI::Shell::{
            Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NOTIFYICONDATAW,
        };
        use windows_sys::Win32::UI::WindowsAndMessaging::{LoadIconW, IDI_APPLICATION};

        let mut tray = Self {
            hwnd,
            is_installed: false,
            state: initial_state,
        };

        unsafe {
            let mut nid: NOTIFYICONDATAW = std::mem::zeroed();
            nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
            nid.hWnd = hwnd;
            nid.uID = 1;
            nid.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
            nid.uCallbackMessage = WM_TRAY_CALLBACK;
            nid.hIcon = LoadIconW(std::ptr::null_mut(), IDI_APPLICATION);

            let tip_text: Vec<u16> = OsStr::new("OpenPet - Desktop Companion")
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();
            let copy_len = tip_text.len().min(nid.szTip.len());
            nid.szTip[..copy_len].copy_from_slice(&tip_text[..copy_len]);

            if Shell_NotifyIconW(NIM_ADD, &nid) != 0 {
                info!("System tray icon registered successfully.");
                tray.is_installed = true;
            } else {
                error!("Failed registering system tray icon via Shell_NotifyIconW.");
            }
        }

        tray
    }

    #[cfg(not(windows))]
    pub fn new(_hwnd: isize, initial_state: TrayState) -> Self {
        Self {
            state: initial_state,
        }
    }

    /// Updates internal state (e.g. privacy mode, locale, pause state).
    pub fn update_state(&mut self, state: TrayState) {
        self.state = state;
    }

    /// Shows the popup context menu at the current cursor position.
    /// Returns the selected `TrayAction`, if any.
    ///
    /// # Safety
    ///
    /// The caller must ensure `hwnd` is a valid, active Win32 window handle owned by the current process.
    #[cfg(windows)]
    pub unsafe fn show_context_menu(
        &self,
        hwnd: windows_sys::Win32::Foundation::HWND,
    ) -> Option<TrayAction> {
        use windows_sys::Win32::Foundation::POINT;
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            AppendMenuW, CreatePopupMenu, DestroyMenu, GetCursorPos, SetForegroundWindow,
            TrackPopupMenu, MF_CHECKED, MF_SEPARATOR, MF_STRING, MF_UNCHECKED, TPM_NONOTIFY,
            TPM_RETURNCMD, TPM_RIGHTBUTTON,
        };

        unsafe {
            let hmenu = CreatePopupMenu();
            if hmenu.is_null() {
                return None;
            }

            let i18n = I18nManager::new(self.state.locale);

            let append_item = |hmenu: windows_sys::Win32::UI::WindowsAndMessaging::HMENU,
                               flags: u32,
                               cmd_id: usize,
                               label: &str| {
                let wide: Vec<u16> = OsStr::new(label)
                    .encode_wide()
                    .chain(std::iter::once(0))
                    .collect();
                AppendMenuW(hmenu, flags, cmd_id, wide.as_ptr());
            };

            // 1. Open Control Center
            append_item(
                hmenu,
                MF_STRING,
                CMD_OPEN_CONTROL,
                i18n.translate("tray.open_control"),
            );

            // 2. Show / Hide Pet
            let pet_vis_label = if self.state.pet_visible {
                if self.state.locale == SupportedLocale::TrTr {
                    "Peti Gizle"
                } else {
                    "Hide Pet"
                }
            } else if self.state.locale == SupportedLocale::TrTr {
                "Peti Göster"
            } else {
                "Show Pet"
            };
            append_item(hmenu, MF_STRING, CMD_TOGGLE_PET, pet_vis_label);

            AppendMenuW(hmenu, MF_SEPARATOR, 0, std::ptr::null());

            // 3. Privacy Mode (Checkbox)
            let priv_flags = MF_STRING
                | if self.state.privacy_mode {
                    MF_CHECKED
                } else {
                    MF_UNCHECKED
                };
            append_item(
                hmenu,
                priv_flags,
                CMD_PRIVACY_MODE,
                i18n.translate("tray.privacy_mode"),
            );

            // 4. Pause Pet (Checkbox)
            let pause_flags = MF_STRING
                | if self.state.pet_paused {
                    MF_CHECKED
                } else {
                    MF_UNCHECKED
                };
            append_item(
                hmenu,
                pause_flags,
                CMD_PAUSE_PET,
                i18n.translate("tray.pause_pet"),
            );

            AppendMenuW(hmenu, MF_SEPARATOR, 0, std::ptr::null());

            // 5. Görünüm: Piksel / Gerçekçi (Art Style)
            let style_label = if self.state.locale == SupportedLocale::TrTr {
                "🎨 Görünüm: Piksel / Gerçekçi"
            } else {
                "🎨 Appearance: Pixel / Realistic"
            };
            append_item(hmenu, MF_STRING, CMD_TOGGLE_STYLE, style_label);

            // 6. 3D Chat Dock (Furever Dock)
            let dock_label = if self.state.locale == SupportedLocale::TrTr {
                "💬 3D Sohbet Barı (Furever Dock)"
            } else {
                "💬 3D Chat Dock (Furever Dock)"
            };
            append_item(hmenu, MF_STRING, CMD_TOGGLE_3D_DOCK, dock_label);

            // 7. Settings
            append_item(
                hmenu,
                MF_STRING,
                CMD_SETTINGS,
                i18n.translate("tray.settings"),
            );

            // 8. Exit
            append_item(hmenu, MF_STRING, CMD_EXIT, i18n.translate("tray.exit"));

            SetForegroundWindow(hwnd);
            let mut pt: POINT = std::mem::zeroed();
            GetCursorPos(&mut pt);

            let selected = TrackPopupMenu(
                hmenu,
                TPM_RETURNCMD | TPM_RIGHTBUTTON | TPM_NONOTIFY,
                pt.x,
                pt.y,
                0,
                hwnd,
                std::ptr::null(),
            ) as usize;

            DestroyMenu(hmenu);
            // KB135788: Post WM_NULL to force clean dismissal of the popup menu
            windows_sys::Win32::UI::WindowsAndMessaging::PostMessageW(
                hwnd,
                windows_sys::Win32::UI::WindowsAndMessaging::WM_NULL,
                0,
                0,
            );

            match selected {
                CMD_OPEN_CONTROL => Some(TrayAction::OpenControlCenter),
                CMD_TOGGLE_PET => Some(TrayAction::TogglePetVisibility),
                CMD_PRIVACY_MODE => Some(TrayAction::TogglePrivacyMode),
                CMD_PAUSE_PET => Some(TrayAction::TogglePausePet),
                CMD_TOGGLE_STYLE => Some(TrayAction::ToggleArtStyle),
                CMD_TOGGLE_3D_DOCK => Some(TrayAction::Toggle3DDock),
                CMD_SETTINGS => Some(TrayAction::OpenSettings),
                CMD_EXIT => Some(TrayAction::ExitApplication),
                _ => None,
            }
        }
    }

    #[cfg(not(windows))]
    pub fn show_context_menu(&self, _hwnd: isize) -> Option<TrayAction> {
        None
    }

    /// Explicitly removes the icon from the Windows taskbar.
    pub fn remove(&mut self) {
        #[cfg(windows)]
        {
            if self.is_installed {
                use windows_sys::Win32::UI::Shell::{
                    Shell_NotifyIconW, NIM_DELETE, NOTIFYICONDATAW,
                };
                unsafe {
                    let mut nid: NOTIFYICONDATAW = std::mem::zeroed();
                    nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
                    nid.hWnd = self.hwnd;
                    nid.uID = 1;
                    Shell_NotifyIconW(NIM_DELETE, &nid);
                }
                self.is_installed = false;
                debug!("System tray icon cleanly removed.");
            }
        }
    }
}

impl Drop for SystemTray {
    fn drop(&mut self) {
        self.remove();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tray_action_variants() {
        assert_eq!(TrayAction::OpenControlCenter as usize, 0);
        let state = TrayState::default();
        assert!(!state.privacy_mode);
        assert!(!state.pet_paused);
        assert!(state.pet_visible);
        assert_eq!(state.art_style, openpet_types::CompanionArtStyle::PixelArt);
    }
}
