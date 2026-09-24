//! # OpenPet Graphical Control Center
//!
//! Windows native Control Center GUI providing companion status, chat interface,
//! desktop reminders management, screen privacy controls, and localized settings (EN/TR).

#![allow(clippy::too_many_arguments, clippy::manual_range_contains)]

use openpet_i18n::I18nManager;
use openpet_ipc::{default_pipe_name, IpcClient};
use openpet_types::{
    AppSettings, IpcRequest, IpcResponse, PetMetadata, PetState, Reminder, SupportedLocale,
};
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use tracing::{error, info};
use uuid::Uuid;

pub const GUI_WIDTH: i32 = 780;
pub const GUI_HEIGHT: i32 = 560;

// Tab Indices
pub const TAB_STATUS: usize = 0;
pub const TAB_CHAT: usize = 1;
pub const TAB_REMINDERS: usize = 2;
pub const TAB_PRIVACY: usize = 3;
pub const TAB_SETTINGS: usize = 4;

// Control IDs
pub const ID_BTN_TAB_STATUS: usize = 2001;
pub const ID_BTN_TAB_CHAT: usize = 2002;
pub const ID_BTN_TAB_REMINDERS: usize = 2003;
pub const ID_BTN_TAB_PRIVACY: usize = 2004;
pub const ID_BTN_TAB_SETTINGS: usize = 2005;

pub const ID_BTN_PET: usize = 2101;
pub const ID_BTN_FEED: usize = 2102;
pub const ID_BTN_PLAY: usize = 2103;
pub const ID_BTN_START_HOST: usize = 2104;

pub const ID_EDIT_CHAT: usize = 2201;
pub const ID_BTN_SEND_CHAT: usize = 2202;

pub const ID_EDIT_REM_TITLE: usize = 2301;
pub const ID_EDIT_REM_BODY: usize = 2302;
pub const ID_BTN_ADD_REM: usize = 2303;

pub const ID_BTN_TOGGLE_PRIVACY: usize = 2401;

pub const ID_BTN_LANG_EN: usize = 2501;
pub const ID_BTN_LANG_TR: usize = 2502;
pub const ID_BTN_SAVE_SETTINGS: usize = 2503;

/// Shared application state rendered by the GUI.
pub struct ControlCenterState {
    pub active_tab: usize,
    pub locale: SupportedLocale,
    pub i18n: I18nManager,
    pub is_connected: bool,
    pub active_pet: Option<PetMetadata>,
    pub pet_state: Option<PetState>,
    pub privacy_mode: bool,
    pub chat_history: Vec<(String, String)>, // (Sender, Message)
    pub reminders: Vec<Reminder>,
    pub settings: AppSettings,
    pub status_feedback: String,
    pub runtime_handle: tokio::runtime::Handle,
    pub shutdown_flag: Arc<AtomicBool>,

    // Win32 child control HWNDs
    #[cfg(windows)]
    pub hwnd_main: windows_sys::Win32::Foundation::HWND,
    #[cfg(windows)]
    pub hwnd_chat_input: windows_sys::Win32::Foundation::HWND,
    #[cfg(windows)]
    pub hwnd_rem_title: windows_sys::Win32::Foundation::HWND,
    #[cfg(windows)]
    pub hwnd_rem_body: windows_sys::Win32::Foundation::HWND,
}

#[cfg(windows)]
unsafe extern "system" fn control_center_wndproc(
    hwnd: windows_sys::Win32::Foundation::HWND,
    msg: u32,
    wparam: windows_sys::Win32::Foundation::WPARAM,
    lparam: windows_sys::Win32::Foundation::LPARAM,
) -> windows_sys::Win32::Foundation::LRESULT {
    use windows_sys::Win32::Foundation::{COLORREF, RECT};
    use windows_sys::Win32::Graphics::Gdi::{
        BeginPaint, BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, CreateFontW,
        CreateSolidBrush, DeleteDC, DeleteObject, EndPaint, FillRect, InvalidateRect, SelectObject,
        SetBkColor, SetBkMode, SetTextColor, TextOutW, FW_BOLD, FW_NORMAL, PAINTSTRUCT, SRCCOPY,
        TRANSPARENT,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        DefWindowProcW, DestroyWindow, GetWindowLongPtrW, PostQuitMessage, GWLP_USERDATA, WM_CLOSE,
        WM_CREATE, WM_CTLCOLOREDIT, WM_CTLCOLORSTATIC, WM_DESTROY, WM_ERASEBKGND, WM_LBUTTONDOWN,
        WM_PAINT, WM_TIMER,
    };

    let state_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Mutex<ControlCenterState>;

    match msg {
        WM_CREATE => 0,
        WM_ERASEBKGND => 1, // Suppress flickering, handled in double-buffered WM_PAINT
        WM_PAINT => {
            if !state_ptr.is_null() {
                let state_guard = (*state_ptr).lock().unwrap();
                let mut ps: PAINTSTRUCT = std::mem::zeroed();
                let hdc = BeginPaint(hwnd, &mut ps);

                let mem_dc = CreateCompatibleDC(hdc);
                let hbitmap = CreateCompatibleBitmap(hdc, GUI_WIDTH, GUI_HEIGHT);
                let old_bmp = SelectObject(mem_dc, hbitmap);

                // Palette definitions (Modern Fluent / Catppuccin Dark)
                let color_sidebar: COLORREF = 0x00251818; // #181825
                let color_content: COLORREF = 0x002E1E1E; // #1E1E2E
                let color_card: COLORREF = 0x00443231; // #313244
                let color_accent: COLORREF = 0x00FAB489; // #89B4FA
                let color_text: COLORREF = 0x00F4D6CD; // #CDD6F4
                let color_subtext: COLORREF = 0x00C8ADA6; // #A6ADC8
                let color_green: COLORREF = 0x00A1E3A6; // #A6E3A1
                let color_red: COLORREF = 0x00A88BF3; // #F38BA8

                // Draw Sidebar
                let brush_sidebar = CreateSolidBrush(color_sidebar);
                let rect_sidebar = RECT {
                    left: 0,
                    top: 0,
                    right: 200,
                    bottom: GUI_HEIGHT,
                };
                FillRect(mem_dc, &rect_sidebar, brush_sidebar);
                DeleteObject(brush_sidebar);

                // Draw Content Area
                let brush_content = CreateSolidBrush(color_content);
                let rect_content = RECT {
                    left: 200,
                    top: 0,
                    right: GUI_WIDTH,
                    bottom: GUI_HEIGHT,
                };
                FillRect(mem_dc, &rect_content, brush_content);
                DeleteObject(brush_content);

                SetBkMode(mem_dc, TRANSPARENT as i32);

                // Fonts
                let font_name: Vec<u16> = OsStr::new("Segoe UI")
                    .encode_wide()
                    .chain(std::iter::once(0))
                    .collect();

                let hfont_title = CreateFontW(
                    24,
                    0,
                    0,
                    0,
                    FW_BOLD as i32,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    font_name.as_ptr(),
                );
                let hfont_bold = CreateFontW(
                    17,
                    0,
                    0,
                    0,
                    FW_BOLD as i32,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    font_name.as_ptr(),
                );
                let hfont_regular = CreateFontW(
                    15,
                    0,
                    0,
                    0,
                    FW_NORMAL as i32,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    font_name.as_ptr(),
                );

                // Sidebar Header: App Title
                SelectObject(mem_dc, hfont_title);
                SetTextColor(mem_dc, color_accent);
                let app_title: Vec<u16> = OsStr::new("OpenPet")
                    .encode_wide()
                    .chain(std::iter::once(0))
                    .collect();
                TextOutW(
                    mem_dc,
                    24,
                    22,
                    app_title.as_ptr(),
                    app_title.len() as i32 - 1,
                );

                SelectObject(mem_dc, hfont_regular);
                SetTextColor(mem_dc, color_subtext);
                let sub_text = if state_guard.locale == SupportedLocale::TrTr {
                    "Masaüstü Dostu"
                } else {
                    "Desktop Companion"
                };
                let wide_sub: Vec<u16> = OsStr::new(sub_text)
                    .encode_wide()
                    .chain(std::iter::once(0))
                    .collect();
                TextOutW(mem_dc, 24, 52, wide_sub.as_ptr(), wide_sub.len() as i32 - 1);

                // Sidebar Navigation Tabs
                let tabs = [
                    (
                        TAB_STATUS,
                        if state_guard.locale == SupportedLocale::TrTr {
                            "🏠  Durum"
                        } else {
                            "🏠  Status"
                        },
                    ),
                    (
                        TAB_CHAT,
                        if state_guard.locale == SupportedLocale::TrTr {
                            "💬  Sohbet"
                        } else {
                            "💬  Chat"
                        },
                    ),
                    (
                        TAB_REMINDERS,
                        if state_guard.locale == SupportedLocale::TrTr {
                            "⏰  Hatırlatıcılar"
                        } else {
                            "⏰  Reminders"
                        },
                    ),
                    (
                        TAB_PRIVACY,
                        if state_guard.locale == SupportedLocale::TrTr {
                            "🛡️  Gizlilik"
                        } else {
                            "🛡️  Privacy"
                        },
                    ),
                    (
                        TAB_SETTINGS,
                        if state_guard.locale == SupportedLocale::TrTr {
                            "⚙️  Ayarlar"
                        } else {
                            "⚙️  Settings"
                        },
                    ),
                ];

                for (idx, (_, label)) in tabs.iter().enumerate() {
                    let y = 95 + (idx as i32 * 45);
                    let is_active = state_guard.active_tab == idx;

                    if is_active {
                        let tab_brush = CreateSolidBrush(color_card);
                        let tab_rect = RECT {
                            left: 12,
                            top: y - 5,
                            right: 188,
                            bottom: y + 32,
                        };
                        FillRect(mem_dc, &tab_rect, tab_brush);
                        DeleteObject(tab_brush);

                        // Highlight pill
                        let pill_brush = CreateSolidBrush(color_accent);
                        let pill_rect = RECT {
                            left: 12,
                            top: y - 2,
                            right: 16,
                            bottom: y + 29,
                        };
                        FillRect(mem_dc, &pill_rect, pill_brush);
                        DeleteObject(pill_brush);

                        SelectObject(mem_dc, hfont_bold);
                        SetTextColor(mem_dc, color_accent);
                    } else {
                        SelectObject(mem_dc, hfont_regular);
                        SetTextColor(mem_dc, color_text);
                    }

                    let wide_label: Vec<u16> = OsStr::new(label)
                        .encode_wide()
                        .chain(std::iter::once(0))
                        .collect();
                    TextOutW(
                        mem_dc,
                        28,
                        y + 2,
                        wide_label.as_ptr(),
                        wide_label.len() as i32 - 1,
                    );
                }

                // Sidebar Footer: Connection Badge
                SelectObject(mem_dc, hfont_regular);
                if state_guard.is_connected {
                    SetTextColor(mem_dc, color_green);
                    let conn_text: Vec<u16> = OsStr::new("● ONLINE (Host)")
                        .encode_wide()
                        .chain(std::iter::once(0))
                        .collect();
                    TextOutW(
                        mem_dc,
                        24,
                        GUI_HEIGHT - 45,
                        conn_text.as_ptr(),
                        conn_text.len() as i32 - 1,
                    );
                } else {
                    SetTextColor(mem_dc, color_red);
                    let conn_text: Vec<u16> = OsStr::new("○ OFFLINE")
                        .encode_wide()
                        .chain(std::iter::once(0))
                        .collect();
                    TextOutW(
                        mem_dc,
                        24,
                        GUI_HEIGHT - 45,
                        conn_text.as_ptr(),
                        conn_text.len() as i32 - 1,
                    );
                }

                // Render Content View based on active_tab
                match state_guard.active_tab {
                    TAB_STATUS => render_tab_status(
                        mem_dc,
                        &state_guard,
                        hfont_title,
                        hfont_bold,
                        hfont_regular,
                        color_card,
                        color_text,
                        color_subtext,
                        color_accent,
                        color_green,
                    ),
                    TAB_CHAT => render_tab_chat(
                        mem_dc,
                        &state_guard,
                        hfont_bold,
                        hfont_regular,
                        color_card,
                        color_text,
                        color_subtext,
                        color_accent,
                    ),
                    TAB_REMINDERS => render_tab_reminders(
                        mem_dc,
                        &state_guard,
                        hfont_bold,
                        hfont_regular,
                        color_card,
                        color_text,
                        color_subtext,
                        color_accent,
                        color_green,
                    ),
                    TAB_PRIVACY => render_tab_privacy(
                        mem_dc,
                        &state_guard,
                        hfont_title,
                        hfont_bold,
                        hfont_regular,
                        color_card,
                        color_text,
                        color_subtext,
                        color_accent,
                        color_green,
                    ),
                    TAB_SETTINGS => render_tab_settings(
                        mem_dc,
                        &state_guard,
                        hfont_title,
                        hfont_bold,
                        hfont_regular,
                        color_card,
                        color_text,
                        color_subtext,
                        color_accent,
                    ),
                    _ => {}
                }

                // Blit back-buffer to screen DC
                BitBlt(hdc, 0, 0, GUI_WIDTH, GUI_HEIGHT, mem_dc, 0, 0, SRCCOPY);

                SelectObject(mem_dc, old_bmp);
                DeleteObject(hbitmap);
                DeleteObject(hfont_title);
                DeleteObject(hfont_bold);
                DeleteObject(hfont_regular);
                DeleteDC(mem_dc);
                EndPaint(hwnd, &ps);
            }
            0
        }
        WM_CTLCOLOREDIT | WM_CTLCOLORSTATIC => {
            // Style child edit inputs and static labels dark
            use windows_sys::Win32::Graphics::Gdi::CreateSolidBrush;
            let brush = CreateSolidBrush(0x00443231);
            let hdc = wparam as isize as *mut std::ffi::c_void;
            SetBkColor(hdc, 0x00443231);
            SetTextColor(hdc, 0x00F4D6CD);
            brush as isize
        }
        WM_LBUTTONDOWN => {
            if !state_ptr.is_null() {
                let mut state_guard = (*state_ptr).lock().unwrap();
                let pt_x = (lparam & 0xFFFF) as i16 as i32;
                let pt_y = ((lparam >> 16) & 0xFFFF) as i16 as i32;

                // 1. Sidebar Tab Click Check
                if pt_x >= 12 && pt_x <= 188 {
                    for i in 0..5 {
                        let y = 95 + (i * 45);
                        if pt_y >= y - 5 && pt_y <= y + 32 {
                            state_guard.active_tab = i as usize;
                            update_child_controls_visibility(&state_guard);
                            InvalidateRect(hwnd, std::ptr::null(), 0);
                            return 0;
                        }
                    }
                }

                // 2. Tab-specific interactive button hit testing
                match state_guard.active_tab {
                    TAB_STATUS => {
                        // Quick interaction buttons
                        // [ Pet Mimi ] at (230, 230, 340, 265)
                        if pt_x >= 230 && pt_x <= 340 && pt_y >= 230 && pt_y <= 265 {
                            info!("GUI Action: Pet active companion");
                            send_ipc_async(
                                &state_guard,
                                IpcRequest::InteractPet(openpet_types::InteractionType::Petting {
                                    intensity: 1.0,
                                }),
                            );
                            state_guard.status_feedback =
                                if state_guard.locale == SupportedLocale::TrTr {
                                    "Mimi mırıldanarak başını eline yasladı! ❤️".into()
                                } else {
                                    "Mimi purred and leaned into your hand! ❤️".into()
                                };
                            InvalidateRect(hwnd, std::ptr::null(), 0);
                        }
                        // [ Feed Fish ] at (355, 230, 465, 265)
                        if pt_x >= 355 && pt_x <= 465 && pt_y >= 230 && pt_y <= 265 {
                            info!("GUI Action: Feed active companion");
                            send_ipc_async(
                                &state_guard,
                                IpcRequest::InteractPet(openpet_types::InteractionType::Feed),
                            );
                            state_guard.status_feedback =
                                if state_guard.locale == SupportedLocale::TrTr {
                                    "Mimi lezzetli balığı afiyetle yedi! 🐟".into()
                                } else {
                                    "Mimi happily munched on fresh salmon! 🐟".into()
                                };
                            InvalidateRect(hwnd, std::ptr::null(), 0);
                        }
                        // [ Play ] at (480, 230, 590, 265)
                        if pt_x >= 480 && pt_x <= 590 && pt_y >= 230 && pt_y <= 265 {
                            info!("GUI Action: Play with active companion");
                            send_ipc_async(
                                &state_guard,
                                IpcRequest::InteractPet(openpet_types::InteractionType::Play),
                            );
                            state_guard.status_feedback =
                                if state_guard.locale == SupportedLocale::TrTr {
                                    "Mimi heyecanla zıplayarak oynamaya başladı! 🎾".into()
                                } else {
                                    "Mimi bounced playfully around! 🎾".into()
                                };
                            InvalidateRect(hwnd, std::ptr::null(), 0);
                        }
                        // [ Start Host ] if offline at (230, 350, 430, 385)
                        if !state_guard.is_connected
                            && pt_x >= 230
                            && pt_x <= 430
                            && pt_y >= 350
                            && pt_y <= 385
                        {
                            info!("GUI Action: Launch OpenPet Host");
                            launch_host_process();
                            state_guard.status_feedback =
                                if state_guard.locale == SupportedLocale::TrTr {
                                    "OpenPet Host arka planda başlatılıyor...".into()
                                } else {
                                    "Starting OpenPet Host in background...".into()
                                };
                            InvalidateRect(hwnd, std::ptr::null(), 0);
                        }
                    }
                    TAB_CHAT => {
                        // [ Send Button ] at (675, 480, 750, 515)
                        if pt_x >= 675 && pt_x <= 750 && pt_y >= 480 && pt_y <= 515 {
                            handle_send_chat(&mut state_guard);
                            InvalidateRect(hwnd, std::ptr::null(), 0);
                        }
                    }
                    TAB_REMINDERS => {
                        // [ Add Reminder Button ] at (630, 475, 750, 510)
                        if pt_x >= 630 && pt_x <= 750 && pt_y >= 475 && pt_y <= 510 {
                            handle_add_reminder(&mut state_guard);
                            InvalidateRect(hwnd, std::ptr::null(), 0);
                        }
                    }
                    TAB_PRIVACY => {
                        // [ Toggle Privacy Button ] at (230, 220, 470, 265)
                        if pt_x >= 230 && pt_x <= 470 && pt_y >= 220 && pt_y <= 265 {
                            let next = !state_guard.privacy_mode;
                            state_guard.privacy_mode = next;
                            info!("GUI Action: Toggled privacy mode to {}", next);
                            send_ipc_async(&state_guard, IpcRequest::SetPrivacyMode(next));
                            InvalidateRect(hwnd, std::ptr::null(), 0);
                        }
                    }
                    TAB_SETTINGS => {
                        // [ English Button ] at (230, 110, 340, 145)
                        if pt_x >= 230 && pt_x <= 340 && pt_y >= 110 && pt_y <= 145 {
                            state_guard.locale = SupportedLocale::EnUs;
                            state_guard.i18n.set_locale(SupportedLocale::EnUs);
                            state_guard.settings.locale = SupportedLocale::EnUs;
                            send_ipc_async(
                                &state_guard,
                                IpcRequest::UpdateSettings(state_guard.settings.clone()),
                            );
                            InvalidateRect(hwnd, std::ptr::null(), 0);
                        }
                        // [ Türkçe Button ] at (360, 110, 470, 145)
                        if pt_x >= 360 && pt_x <= 470 && pt_y >= 110 && pt_y <= 145 {
                            state_guard.locale = SupportedLocale::TrTr;
                            state_guard.i18n.set_locale(SupportedLocale::TrTr);
                            state_guard.settings.locale = SupportedLocale::TrTr;
                            send_ipc_async(
                                &state_guard,
                                IpcRequest::UpdateSettings(state_guard.settings.clone()),
                            );
                            InvalidateRect(hwnd, std::ptr::null(), 0);
                        }
                    }
                    _ => {}
                }
            }
            0
        }
        WM_TIMER => {
            // Periodic polling of host state
            if !state_ptr.is_null() {
                let mut state_guard = (*state_ptr).lock().unwrap();
                poll_host_updates(&mut state_guard);
                InvalidateRect(hwnd, std::ptr::null(), 0);
            }
            0
        }
        WM_CLOSE => {
            DestroyWindow(hwnd);
            0
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

// ---------------------------------------------------------------------------
// View Renderers
// ---------------------------------------------------------------------------

#[cfg(windows)]
unsafe fn render_tab_status(
    hdc: windows_sys::Win32::Graphics::Gdi::HDC,
    state: &ControlCenterState,
    hfont_title: windows_sys::Win32::Graphics::Gdi::HFONT,
    hfont_bold: windows_sys::Win32::Graphics::Gdi::HFONT,
    hfont_regular: windows_sys::Win32::Graphics::Gdi::HFONT,
    color_card: u32,
    color_text: u32,
    color_subtext: u32,
    color_accent: u32,
    color_green: u32,
) {
    use windows_sys::Win32::Foundation::RECT;
    use windows_sys::Win32::Graphics::Gdi::{
        CreateSolidBrush, DeleteObject, FillRect, SelectObject, SetTextColor,
    };

    SelectObject(hdc, hfont_title);
    SetTextColor(hdc, color_accent);
    let title_text = if state.locale == SupportedLocale::TrTr {
        "Masaüstü Pet Durumu"
    } else {
        "Desktop Pet Overview"
    };
    draw_str(hdc, 230, 24, title_text);

    // Companion Card
    let card_brush = CreateSolidBrush(color_card);
    let card_rect = RECT {
        left: 230,
        top: 70,
        right: 740,
        bottom: 215,
    };
    FillRect(hdc, &card_rect, card_brush);
    DeleteObject(card_brush);

    SelectObject(hdc, hfont_bold);
    SetTextColor(hdc, color_text);
    let pet_name = state
        .active_pet
        .as_ref()
        .map(|p| p.name.as_str())
        .unwrap_or("Mimi the Cat");
    draw_str(hdc, 250, 85, pet_name);

    SelectObject(hdc, hfont_regular);
    SetTextColor(hdc, color_subtext);
    let author_text = if state.locale == SupportedLocale::TrTr {
        "Geliştirici: Tiyatrotist  |  Lisans: AGPL-3.0  |  Tür: Kedicik"
    } else {
        "Author: Tiyatrotist  |  License: AGPL-3.0  |  Type: Starter Feline"
    };
    draw_str(hdc, 250, 110, author_text);

    // Mood & Energy Bars
    let (mood, energy) = if let Some(ref st) = state.pet_state {
        (st.mood, st.energy)
    } else {
        (0.85, 0.70)
    };

    SetTextColor(hdc, color_text);
    let mood_label = if state.locale == SupportedLocale::TrTr {
        format!("Mutluluk (Mood)   : {:.0}%", mood * 100.0)
    } else {
        format!("Happiness (Mood)  : {:.0}%", mood * 100.0)
    };
    draw_str(hdc, 250, 138, &mood_label);

    let energy_label = if state.locale == SupportedLocale::TrTr {
        format!("Enerji (Energy)   : {:.0}%", energy * 100.0)
    } else {
        format!("Energy Level      : {:.0}%", energy * 100.0)
    };
    draw_str(hdc, 250, 162, &energy_label);

    let behavior_label = if state.locale == SupportedLocale::TrTr {
        "Durum: Aktif ve Sağlıklı (Yerel AI Motoru Açık)"
    } else {
        "Behavior: Active & Playful (Local Deterministic Engine)"
    };
    draw_str(hdc, 250, 186, behavior_label);

    // Interactive Action Buttons
    let btn_brush = CreateSolidBrush(0x004E3A36);

    let buttons = [
        (
            230,
            340,
            if state.locale == SupportedLocale::TrTr {
                "🐾  Sev"
            } else {
                "🐾  Pet"
            },
        ),
        (
            355,
            465,
            if state.locale == SupportedLocale::TrTr {
                "🐟  Besle"
            } else {
                "🐟  Feed"
            },
        ),
        (
            480,
            590,
            if state.locale == SupportedLocale::TrTr {
                "🎾  Oyna"
            } else {
                "🎾  Play"
            },
        ),
    ];

    for (x0, x1, lbl) in buttons {
        let b_rect = RECT {
            left: x0,
            top: 230,
            right: x1,
            bottom: 265,
        };
        FillRect(hdc, &b_rect, btn_brush);
        SelectObject(hdc, hfont_bold);
        SetTextColor(hdc, color_accent);
        draw_str(hdc, x0 + 15, 238, lbl);
    }
    DeleteObject(btn_brush);

    // Status feedback message
    if !state.status_feedback.is_empty() {
        SelectObject(hdc, hfont_regular);
        SetTextColor(hdc, color_green);
        draw_str(hdc, 230, 280, &state.status_feedback);
    }

    if !state.is_connected {
        SelectObject(hdc, hfont_regular);
        SetTextColor(hdc, 0x00A88BF3);
        let notice = if state.locale == SupportedLocale::TrTr {
            "OpenPet Host arka planda çalışmıyor. Başlatmak için tıklayın:"
        } else {
            "OpenPet Host is not running in background. Click to start:"
        };
        draw_str(hdc, 230, 320, notice);

        let start_brush = CreateSolidBrush(color_accent);
        let start_rect = RECT {
            left: 230,
            top: 350,
            right: 430,
            bottom: 385,
        };
        FillRect(hdc, &start_rect, start_brush);
        DeleteObject(start_brush);

        SelectObject(hdc, hfont_bold);
        SetTextColor(hdc, 0x001E1E2E);
        let btn_lbl = if state.locale == SupportedLocale::TrTr {
            "🚀  Peti Masaüstünde Başlat"
        } else {
            "🚀  Launch Desktop Pet"
        };
        draw_str(hdc, 240, 358, btn_lbl);
    }
}

#[cfg(windows)]
unsafe fn render_tab_chat(
    hdc: windows_sys::Win32::Graphics::Gdi::HDC,
    state: &ControlCenterState,
    hfont_bold: windows_sys::Win32::Graphics::Gdi::HFONT,
    hfont_regular: windows_sys::Win32::Graphics::Gdi::HFONT,
    color_card: u32,
    color_text: u32,
    color_subtext: u32,
    color_accent: u32,
) {
    use windows_sys::Win32::Foundation::RECT;
    use windows_sys::Win32::Graphics::Gdi::{
        CreateSolidBrush, DeleteObject, FillRect, SelectObject, SetTextColor,
    };

    SelectObject(hdc, hfont_bold);
    SetTextColor(hdc, color_accent);
    let title = if state.locale == SupportedLocale::TrTr {
        "Petinizle Sohbet Edin"
    } else {
        "Chat with your Companion"
    };
    draw_str(hdc, 230, 24, title);

    // Chat History Box
    let chat_brush = CreateSolidBrush(color_card);
    let chat_rect = RECT {
        left: 230,
        top: 60,
        right: 750,
        bottom: 460,
    };
    FillRect(hdc, &chat_rect, chat_brush);
    DeleteObject(chat_brush);

    SelectObject(hdc, hfont_regular);
    let mut y = 75;
    for (sender, msg) in state.chat_history.iter().rev().take(12).rev() {
        if sender == "User" {
            SetTextColor(hdc, color_accent);
            draw_str(hdc, 245, y, "You: ");
        } else {
            SetTextColor(hdc, 0x00A1E3A6); // Greenish
            draw_str(hdc, 245, y, "Mimi: ");
        }
        SetTextColor(hdc, color_text);
        draw_str(hdc, 295, y, msg);
        y += 28;
    }

    if state.chat_history.is_empty() {
        SetTextColor(hdc, color_subtext);
        let empty_msg = if state.locale == SupportedLocale::TrTr {
            "Henüz mesaj yok. Mimi ile konuşmak için aşağıdan bir şeyler yazın!"
        } else {
            "No messages yet. Type something below to talk with Mimi!"
        };
        draw_str(hdc, 250, 80, empty_msg);
    }

    // Send Button Hitbox
    let send_brush = CreateSolidBrush(color_accent);
    let send_rect = RECT {
        left: 675,
        top: 480,
        right: 750,
        bottom: 515,
    };
    FillRect(hdc, &send_rect, send_brush);
    DeleteObject(send_brush);

    SelectObject(hdc, hfont_bold);
    SetTextColor(hdc, 0x001E1E2E);
    let send_text = if state.locale == SupportedLocale::TrTr {
        "Gönder"
    } else {
        "Send"
    };
    draw_str(hdc, 688, 488, send_text);
}

#[cfg(windows)]
unsafe fn render_tab_reminders(
    hdc: windows_sys::Win32::Graphics::Gdi::HDC,
    state: &ControlCenterState,
    hfont_bold: windows_sys::Win32::Graphics::Gdi::HFONT,
    hfont_regular: windows_sys::Win32::Graphics::Gdi::HFONT,
    color_card: u32,
    color_text: u32,
    color_subtext: u32,
    color_accent: u32,
    color_green: u32,
) {
    use windows_sys::Win32::Foundation::RECT;
    use windows_sys::Win32::Graphics::Gdi::{
        CreateSolidBrush, DeleteObject, FillRect, SelectObject, SetTextColor,
    };

    SelectObject(hdc, hfont_bold);
    SetTextColor(hdc, color_accent);
    let title = if state.locale == SupportedLocale::TrTr {
        "Masaüstü Hatırlatıcıları"
    } else {
        "Desktop Reminders"
    };
    draw_str(hdc, 230, 24, title);

    // List box background
    let list_brush = CreateSolidBrush(color_card);
    let list_rect = RECT {
        left: 230,
        top: 60,
        right: 750,
        bottom: 370,
    };
    FillRect(hdc, &list_rect, list_brush);
    DeleteObject(list_brush);

    SelectObject(hdc, hfont_regular);
    if state.reminders.is_empty() {
        SetTextColor(hdc, color_subtext);
        let empty_str = if state.locale == SupportedLocale::TrTr {
            "Kayıtlı hatırlatıcı yok. Aşağıdan yeni bir hatırlatıcı ekleyebilirsiniz."
        } else {
            "No reminders scheduled. Use the form below to add one."
        };
        draw_str(hdc, 250, 80, empty_str);
    } else {
        let mut y = 75;
        for r in &state.reminders {
            SetTextColor(hdc, color_accent);
            draw_str(hdc, 245, y, &format!("📌 {}", r.title));
            SetTextColor(hdc, color_text);
            draw_str(hdc, 265, y + 20, &r.body);
            SetTextColor(hdc, color_subtext);
            draw_str(
                hdc,
                550,
                y,
                &format!("Time: {}", r.schedule.format("%Y-%m-%d %H:%M")),
            );
            y += 50;
            if y > 350 {
                break;
            }
        }
    }

    // Add reminder section
    SelectObject(hdc, hfont_bold);
    SetTextColor(hdc, color_text);
    let add_title = if state.locale == SupportedLocale::TrTr {
        "Yeni Hatırlatıcı Ekle:"
    } else {
        "Add New Reminder:"
    };
    draw_str(hdc, 230, 390, add_title);

    SelectObject(hdc, hfont_regular);
    SetTextColor(hdc, color_subtext);
    draw_str(
        hdc,
        230,
        425,
        if state.locale == SupportedLocale::TrTr {
            "Başlık:"
        } else {
            "Title:"
        },
    );
    draw_str(
        hdc,
        230,
        465,
        if state.locale == SupportedLocale::TrTr {
            "Mesaj:"
        } else {
            "Body:"
        },
    );

    // Add Button
    let btn_brush = CreateSolidBrush(color_green);
    let btn_rect = RECT {
        left: 630,
        top: 475,
        right: 750,
        bottom: 510,
    };
    FillRect(hdc, &btn_rect, btn_brush);
    DeleteObject(btn_brush);

    SelectObject(hdc, hfont_bold);
    SetTextColor(hdc, 0x001E1E2E);
    let add_btn_text = if state.locale == SupportedLocale::TrTr {
        "+ Ekle"
    } else {
        "+ Add"
    };
    draw_str(hdc, 665, 483, add_btn_text);
}

#[cfg(windows)]
unsafe fn render_tab_privacy(
    hdc: windows_sys::Win32::Graphics::Gdi::HDC,
    state: &ControlCenterState,
    hfont_title: windows_sys::Win32::Graphics::Gdi::HFONT,
    hfont_bold: windows_sys::Win32::Graphics::Gdi::HFONT,
    hfont_regular: windows_sys::Win32::Graphics::Gdi::HFONT,
    color_card: u32,
    color_text: u32,
    color_subtext: u32,
    color_accent: u32,
    color_green: u32,
) {
    use windows_sys::Win32::Foundation::RECT;
    use windows_sys::Win32::Graphics::Gdi::{
        CreateSolidBrush, DeleteObject, FillRect, SelectObject, SetTextColor,
    };

    SelectObject(hdc, hfont_title);
    SetTextColor(hdc, color_accent);
    let title = if state.locale == SupportedLocale::TrTr {
        "Gizlilik ve Güvenlik Güvencesi"
    } else {
        "Privacy & Security Safeguards"
    };
    draw_str(hdc, 230, 24, title);

    // Privacy Card
    let card_brush = CreateSolidBrush(color_card);
    let card_rect = RECT {
        left: 230,
        top: 70,
        right: 740,
        bottom: 200,
    };
    FillRect(hdc, &card_rect, card_brush);
    DeleteObject(card_brush);

    SelectObject(hdc, hfont_bold);
    SetTextColor(hdc, color_text);
    let card_header = if state.locale == SupportedLocale::TrTr {
        "🛡️  Yerel-Öncelikli (Local-First) ve Sıfır-Telemetri Mimarisi"
    } else {
        "🛡️  Local-First Architecture & Zero-Telemetry Guarantee"
    };
    draw_str(hdc, 250, 85, card_header);

    SelectObject(hdc, hfont_regular);
    SetTextColor(hdc, color_subtext);
    let info_1 = if state.locale == SupportedLocale::TrTr {
        "• Verileriniz asla uzaktaki bir sunucuya gönderilmez."
    } else {
        "• Your personal data never leaves your computer."
    };
    let info_2 = if state.locale == SupportedLocale::TrTr {
        "• Ekran analizi varsayılan olarak kesinlikle KAPALIDIR."
    } else {
        "• Desktop screen analysis is strictly disabled by default."
    };
    let info_3 = if state.locale == SupportedLocale::TrTr {
        "• Gizlilik Modu aktifken tüm görsel algılama kancaları anında durdurulur."
    } else {
        "• Privacy Mode instantly severs any screen capture/OCR hooks."
    };
    draw_str(hdc, 250, 115, info_1);
    draw_str(hdc, 250, 140, info_2);
    draw_str(hdc, 250, 165, info_3);

    // Privacy Mode Toggle Button
    let btn_color = if state.privacy_mode {
        color_green
    } else {
        0x00443231 // Inactive card color
    };
    let toggle_brush = CreateSolidBrush(btn_color);
    let toggle_rect = RECT {
        left: 230,
        top: 220,
        right: 520,
        bottom: 265,
    };
    FillRect(hdc, &toggle_rect, toggle_brush);
    DeleteObject(toggle_brush);

    SelectObject(hdc, hfont_bold);
    let text_color = if state.privacy_mode {
        0x001E1E2E
    } else {
        color_accent
    };
    SetTextColor(hdc, text_color);
    let toggle_text = if state.privacy_mode {
        if state.locale == SupportedLocale::TrTr {
            "✓ GİZLİLİK MODU AKTİF (KORUMALI)"
        } else {
            "✓ PRIVACY MODE ACTIVE (GUARDED)"
        }
    } else if state.locale == SupportedLocale::TrTr {
        "🛡️ Gizlilik Modunu Aç (Tıkla)"
    } else {
        "🛡️ Enable Privacy Mode (Click)"
    };
    draw_str(hdc, 245, 232, toggle_text);
}

#[cfg(windows)]
unsafe fn render_tab_settings(
    hdc: windows_sys::Win32::Graphics::Gdi::HDC,
    state: &ControlCenterState,
    hfont_title: windows_sys::Win32::Graphics::Gdi::HFONT,
    hfont_bold: windows_sys::Win32::Graphics::Gdi::HFONT,
    hfont_regular: windows_sys::Win32::Graphics::Gdi::HFONT,
    color_card: u32,
    color_text: u32,
    color_subtext: u32,
    color_accent: u32,
) {
    use windows_sys::Win32::Foundation::RECT;
    use windows_sys::Win32::Graphics::Gdi::{
        CreateSolidBrush, DeleteObject, FillRect, SelectObject, SetTextColor,
    };

    SelectObject(hdc, hfont_title);
    SetTextColor(hdc, color_accent);
    let title = if state.locale == SupportedLocale::TrTr {
        "Uygulama Ayarları"
    } else {
        "Application Settings"
    };
    draw_str(hdc, 230, 24, title);

    SelectObject(hdc, hfont_bold);
    SetTextColor(hdc, color_text);
    let lang_header = if state.locale == SupportedLocale::TrTr {
        "Arayüz Dili / Interface Language:"
    } else {
        "Interface Language:"
    };
    draw_str(hdc, 230, 75, lang_header);

    // Language buttons: English / Türkçe
    let en_active = state.locale == SupportedLocale::EnUs;
    let brush_en = CreateSolidBrush(if en_active { color_accent } else { color_card });
    let rect_en = RECT {
        left: 230,
        top: 105,
        right: 345,
        bottom: 140,
    };
    FillRect(hdc, &rect_en, brush_en);
    DeleteObject(brush_en);

    SelectObject(hdc, hfont_bold);
    SetTextColor(hdc, if en_active { 0x001E1E2E } else { color_text });
    draw_str(hdc, 245, 114, "English");

    let tr_active = state.locale == SupportedLocale::TrTr;
    let brush_tr = CreateSolidBrush(if tr_active { color_accent } else { color_card });
    let rect_tr = RECT {
        left: 360,
        top: 105,
        right: 475,
        bottom: 140,
    };
    FillRect(hdc, &rect_tr, brush_tr);
    DeleteObject(brush_tr);

    SetTextColor(hdc, if tr_active { 0x001E1E2E } else { color_text });
    draw_str(hdc, 380, 114, "Türkçe");

    // Other settings info
    SelectObject(hdc, hfont_bold);
    SetTextColor(hdc, color_text);
    draw_str(hdc, 230, 175, "Rendering & Accessibility:");

    SelectObject(hdc, hfont_regular);
    SetTextColor(hdc, color_subtext);
    let opt1 = if state.locale == SupportedLocale::TrTr {
        "• Animasyon Kalitesi: Yüksek (60 FPS Doğal Hızlanma)"
    } else {
        "• Animation Quality: High (60 FPS Dynamic Acceleration)"
    };
    let opt2 = if state.locale == SupportedLocale::TrTr {
        "• Her Zaman Üstte: Açık (Windows 11 Masaüstü Üzerinde Yüzme)"
    } else {
        "• Always On Top: Enabled (Floating on Windows 11 Desktop)"
    };
    let opt3 = if state.locale == SupportedLocale::TrTr {
        "• Lisans ve Kod: AGPL-3.0 Açık Kaynak - Tiyatrotist"
    } else {
        "• License & Core: AGPL-3.0 Open-Source by Tiyatrotist"
    };
    draw_str(hdc, 230, 205, opt1);
    draw_str(hdc, 230, 230, opt2);
    draw_str(hdc, 230, 255, opt3);
}

// ---------------------------------------------------------------------------
// Helper Methods
// ---------------------------------------------------------------------------

#[cfg(windows)]
fn draw_str(hdc: windows_sys::Win32::Graphics::Gdi::HDC, x: i32, y: i32, text: &str) {
    let wide: Vec<u16> = OsStr::new(text)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    unsafe {
        windows_sys::Win32::Graphics::Gdi::TextOutW(
            hdc,
            x,
            y,
            wide.as_ptr(),
            wide.len() as i32 - 1,
        );
    }
}

#[cfg(windows)]
fn update_child_controls_visibility(state: &ControlCenterState) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_HIDE, SW_SHOW};
    unsafe {
        let is_chat = state.active_tab == TAB_CHAT;
        ShowWindow(
            state.hwnd_chat_input,
            if is_chat { SW_SHOW } else { SW_HIDE },
        );

        let is_rem = state.active_tab == TAB_REMINDERS;
        ShowWindow(state.hwnd_rem_title, if is_rem { SW_SHOW } else { SW_HIDE });
        ShowWindow(state.hwnd_rem_body, if is_rem { SW_SHOW } else { SW_HIDE });
    }
}

fn send_ipc_async(state: &ControlCenterState, req: IpcRequest) {
    let handle = state.runtime_handle.clone();
    let pipe = default_pipe_name();
    handle.spawn(async move {
        if let Ok(mut client) = IpcClient::connect(&pipe).await {
            let _ = client.request(&req).await;
        }
    });
}

fn poll_host_updates(state: &mut ControlCenterState) {
    let pipe = default_pipe_name();
    let handle = state.runtime_handle.clone();

    // Async poll
    handle.block_on(async {
        if let Ok(mut client) = IpcClient::connect(&pipe).await {
            state.is_connected = true;

            // Fetch Active Pet
            if let Ok(IpcResponse::ActivePet(Some(pet))) =
                client.request(&IpcRequest::GetActivePet).await
            {
                state.active_pet = Some(pet);
            }

            // Fetch Pet State
            if let Ok(IpcResponse::PetState(pet_st)) =
                client.request(&IpcRequest::GetPetState).await
            {
                state.pet_state = Some(pet_st);
            }

            // Fetch Reminders
            if let Ok(IpcResponse::Reminders(rems)) =
                client.request(&IpcRequest::ListReminders).await
            {
                state.reminders = rems;
            }

            // Fetch Settings
            if let Ok(IpcResponse::Settings(st)) = client.request(&IpcRequest::GetSettings).await {
                if state.locale != st.locale {
                    state.locale = st.locale;
                    state.i18n.set_locale(st.locale);
                }
                state.settings = st.clone();
                state.privacy_mode = st.privacy_mode;
            }
        } else {
            state.is_connected = false;
        }
    });
}

fn handle_send_chat(state: &mut ControlCenterState) {
    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            GetWindowTextLengthW, GetWindowTextW, SetWindowTextW,
        };
        unsafe {
            let len = GetWindowTextLengthW(state.hwnd_chat_input);
            if len > 0 {
                let mut buf = vec![0u16; (len + 1) as usize];
                GetWindowTextW(state.hwnd_chat_input, buf.as_mut_ptr(), len + 1);
                let text = String::from_utf16_lossy(&buf[..len as usize]);
                let trimmed = text.trim().to_string();

                if !trimmed.is_empty() {
                    // Clear input
                    let empty_wide: Vec<u16> = OsStr::new("")
                        .encode_wide()
                        .chain(std::iter::once(0))
                        .collect();
                    SetWindowTextW(state.hwnd_chat_input, empty_wide.as_ptr());

                    state.chat_history.push(("User".into(), trimmed.clone()));

                    // Send to host via IPC
                    let req = IpcRequest::SendChatMessage {
                        conversation_id: Uuid::new_v4(),
                        content: trimmed,
                    };
                    let pipe = default_pipe_name();
                    let handle = state.runtime_handle.clone();
                    let locale = state.locale;
                    handle.block_on(async {
                        if let Ok(mut client) = IpcClient::connect(&pipe).await {
                            if let Ok(IpcResponse::ChatMessage(reply)) = client.request(&req).await
                            {
                                state.chat_history.push(("Mimi".into(), reply.content));
                                return;
                            }
                        }
                        // Offline companion fallback
                        let fallback = if locale == SupportedLocale::TrTr {
                            "*mırıldanarak patisini uzatır* (Masaüstü Pet Bağlantısı)"
                        } else {
                            "*purrs softly and stretches paw* (Desktop Pet Companion)"
                        };
                        state.chat_history.push(("Mimi".into(), fallback.into()));
                    });
                }
            }
        }
    }
}

fn handle_add_reminder(state: &mut ControlCenterState) {
    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            GetWindowTextLengthW, GetWindowTextW, SetWindowTextW,
        };
        unsafe {
            let t_len = GetWindowTextLengthW(state.hwnd_rem_title);
            let b_len = GetWindowTextLengthW(state.hwnd_rem_body);

            if t_len > 0 {
                let mut t_buf = vec![0u16; (t_len + 1) as usize];
                GetWindowTextW(state.hwnd_rem_title, t_buf.as_mut_ptr(), t_len + 1);
                let title = String::from_utf16_lossy(&t_buf[..t_len as usize])
                    .trim()
                    .to_string();

                let mut b_buf = vec![0u16; (b_len + 1) as usize];
                GetWindowTextW(state.hwnd_rem_body, b_buf.as_mut_ptr(), b_len + 1);
                let body = String::from_utf16_lossy(&b_buf[..b_len as usize])
                    .trim()
                    .to_string();

                let req = IpcRequest::CreateReminder {
                    title: title.clone(),
                    body: body.clone(),
                    schedule_utc: chrono::Utc::now() + chrono::Duration::minutes(15),
                    recurrence: openpet_types::RecurrenceRule::Once,
                };

                let pipe = default_pipe_name();
                let handle = state.runtime_handle.clone();
                handle.block_on(async {
                    if let Ok(mut client) = IpcClient::connect(&pipe).await {
                        let _ = client.request(&req).await;
                    }
                });

                // Clear input boxes
                let empty_wide: Vec<u16> = OsStr::new("")
                    .encode_wide()
                    .chain(std::iter::once(0))
                    .collect();
                SetWindowTextW(state.hwnd_rem_title, empty_wide.as_ptr());
                SetWindowTextW(state.hwnd_rem_body, empty_wide.as_ptr());

                poll_host_updates(state);
            }
        }
    }
}

fn launch_host_process() {
    use std::process::Command;
    let current_exe = std::env::current_exe().unwrap_or_default();
    let parent_dir = current_exe.parent().unwrap_or(std::path::Path::new(""));

    let candidates = [
        parent_dir.join("openpet-host.exe"),
        parent_dir.join("openpet-host"),
        std::path::PathBuf::from("target/debug/openpet-host.exe"),
        std::path::PathBuf::from("target/release/openpet-host.exe"),
    ];

    for candidate in &candidates {
        if candidate.exists() {
            info!("Launching host from {:?}", candidate);
            let _ = Command::new(candidate).spawn();
            return;
        }
    }

    let mut cmd = Command::new("cargo");
    cmd.args(["run", "-p", "openpet-host"]);
    let _ = cmd.spawn();
}

/// Main entrypoint launching the graphical Control Center window.
pub fn run_control_center_gui(initial_locale: SupportedLocale, initial_tab: usize) {
    #[cfg(windows)]
    run_windows_gui_loop(initial_locale, initial_tab);

    #[cfg(not(windows))]
    {
        let _ = (initial_locale, initial_tab);
        println!("Graphical Control Center is currently optimized for Windows 10/11.");
    }
}

#[cfg(windows)]
fn run_windows_gui_loop(initial_locale: SupportedLocale, initial_tab: usize) {
    use windows_sys::Win32::Graphics::Gdi::{
        CreateFontW, CreateSolidBrush, DeleteObject, FW_NORMAL,
    };
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DispatchMessageW, GetMessageW, GetWindowLongPtrW, LoadCursorW, LoadIconW,
        RegisterClassExW, SetForegroundWindow, SetTimer, SetWindowLongPtrW, ShowWindow,
        TranslateMessage, CS_HREDRAW, CS_VREDRAW, ES_AUTOHSCROLL, GWLP_USERDATA, IDC_ARROW,
        IDI_APPLICATION, MSG, SW_HIDE, SW_RESTORE, SW_SHOW, WNDCLASSEXW, WS_BORDER, WS_CHILD,
        WS_OVERLAPPEDWINDOW, WS_VISIBLE,
    };

    let rt = tokio::runtime::Runtime::new().expect("Tokio runtime required for Control Center GUI");
    let handle = rt.handle().clone();

    unsafe {
        let class_name: Vec<u16> = OsStr::new("OpenPet_ControlCenterWindowClass")
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        // If an instance is already open, restore and bring to foreground
        let existing = windows_sys::Win32::UI::WindowsAndMessaging::FindWindowW(
            class_name.as_ptr(),
            std::ptr::null(),
        );
        if !existing.is_null() {
            ShowWindow(existing, SW_RESTORE);
            SetForegroundWindow(existing);
            info!("Existing OpenPet Control Center window focused and brought to front.");
            return;
        }

        let hinstance = GetModuleHandleW(std::ptr::null());
        let bg_brush = CreateSolidBrush(0x001E1E2E);

        let wnd_class = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(control_center_wndproc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinstance,
            hIcon: LoadIconW(std::ptr::null_mut(), IDI_APPLICATION),
            hCursor: LoadCursorW(std::ptr::null_mut(), IDC_ARROW),
            hbrBackground: bg_brush,
            lpszMenuName: std::ptr::null(),
            lpszClassName: class_name.as_ptr(),
            hIconSm: std::ptr::null_mut(),
        };

        RegisterClassExW(&wnd_class);

        let win_title: Vec<u16> = OsStr::new("OpenPet Control Center")
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let hwnd = CreateWindowExW(
            0,
            class_name.as_ptr(),
            win_title.as_ptr(),
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            150,
            120,
            GUI_WIDTH,
            GUI_HEIGHT,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            hinstance,
            std::ptr::null_mut(),
        );

        if hwnd.is_null() {
            error!("Failed creating OpenPet Control Center window!");
            DeleteObject(bg_brush);
            return;
        }

        let edit_class: Vec<u16> = OsStr::new("EDIT")
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        // 1. Chat input edit control
        let hwnd_chat_input = CreateWindowExW(
            0,
            edit_class.as_ptr(),
            std::ptr::null(),
            WS_CHILD | WS_BORDER | (ES_AUTOHSCROLL as u32),
            230,
            480,
            430,
            35,
            hwnd,
            ID_EDIT_CHAT as isize as _,
            hinstance,
            std::ptr::null_mut(),
        );

        // 2. Reminder title input
        let hwnd_rem_title = CreateWindowExW(
            0,
            edit_class.as_ptr(),
            std::ptr::null(),
            WS_CHILD | WS_BORDER | (ES_AUTOHSCROLL as u32),
            280,
            420,
            330,
            30,
            hwnd,
            ID_EDIT_REM_TITLE as isize as _,
            hinstance,
            std::ptr::null_mut(),
        );

        // 3. Reminder body input
        let hwnd_rem_body = CreateWindowExW(
            0,
            edit_class.as_ptr(),
            std::ptr::null(),
            WS_CHILD | WS_BORDER | (ES_AUTOHSCROLL as u32),
            280,
            465,
            330,
            30,
            hwnd,
            ID_EDIT_REM_BODY as isize as _,
            hinstance,
            std::ptr::null_mut(),
        );

        // Set modern font on edit controls
        let font_name: Vec<u16> = OsStr::new("Segoe UI")
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let edit_font = CreateFontW(
            15,
            0,
            0,
            0,
            FW_NORMAL as i32,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            font_name.as_ptr(),
        );
        use windows_sys::Win32::UI::WindowsAndMessaging::WM_SETFONT;
        windows_sys::Win32::UI::WindowsAndMessaging::SendMessageW(
            hwnd_chat_input,
            WM_SETFONT,
            edit_font as usize,
            1,
        );
        windows_sys::Win32::UI::WindowsAndMessaging::SendMessageW(
            hwnd_rem_title,
            WM_SETFONT,
            edit_font as usize,
            1,
        );
        windows_sys::Win32::UI::WindowsAndMessaging::SendMessageW(
            hwnd_rem_body,
            WM_SETFONT,
            edit_font as usize,
            1,
        );

        // Initially show/hide edit controls based on initial_tab
        let cur_tab = initial_tab.min(TAB_SETTINGS);
        if cur_tab == TAB_CHAT {
            ShowWindow(hwnd_chat_input, SW_SHOW);
            ShowWindow(hwnd_rem_title, SW_HIDE);
            ShowWindow(hwnd_rem_body, SW_HIDE);
        } else if cur_tab == TAB_REMINDERS {
            ShowWindow(hwnd_chat_input, SW_HIDE);
            ShowWindow(hwnd_rem_title, SW_SHOW);
            ShowWindow(hwnd_rem_body, SW_SHOW);
        } else {
            ShowWindow(hwnd_chat_input, SW_HIDE);
            ShowWindow(hwnd_rem_title, SW_HIDE);
            ShowWindow(hwnd_rem_body, SW_HIDE);
        }

        let i18n = I18nManager::new(initial_locale);
        let state = Box::new(Mutex::new(ControlCenterState {
            active_tab: cur_tab,
            locale: initial_locale,
            i18n,
            is_connected: false,
            active_pet: None,
            pet_state: None,
            privacy_mode: false,
            chat_history: Vec::new(),
            reminders: Vec::new(),
            settings: AppSettings::default(),
            status_feedback: String::new(),
            runtime_handle: handle,
            shutdown_flag: Arc::new(AtomicBool::new(false)),
            hwnd_main: hwnd,
            hwnd_chat_input,
            hwnd_rem_title,
            hwnd_rem_body,
        }));

        SetWindowLongPtrW(hwnd, GWLP_USERDATA, Box::into_raw(state) as isize);

        // Poll host on launch
        let state_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Mutex<ControlCenterState>;
        if !state_ptr.is_null() {
            let mut state_guard = (*state_ptr).lock().unwrap();
            poll_host_updates(&mut state_guard);
        }

        // Set refresh timer (every 1 second)
        SetTimer(hwnd, 1, 1000, None);

        info!("OpenPet Control Center GUI window displayed.");

        // Message pump
        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        DeleteObject(bg_brush);
        DeleteObject(edit_font);

        let state_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Mutex<ControlCenterState>;
        if !state_ptr.is_null() {
            let _ = Box::from_raw(state_ptr);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
        }
        info!("OpenPet Control Center GUI exited.");
    }
}
