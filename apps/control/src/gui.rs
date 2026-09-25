//! # OpenPet Graphical Control Center
//!
//! Windows native Control Center GUI providing companion status, cat care, chat interface,
//! desktop reminders management, structured memory facts, screen privacy controls, and
//! localized settings (EN/TR) with a warm Cozy & Pastel theme.

#![allow(
    clippy::too_many_arguments,
    clippy::manual_range_contains,
    clippy::manual_is_multiple_of,
    clippy::unnecessary_cast
)]

use openpet_i18n::I18nManager;
use openpet_ipc::{default_pipe_name, IpcClient};
use openpet_render::{MimiSpriteSheet, RealisticCompanionSheet};
use openpet_types::{
    AppSettings, CatBreed, CompanionArtStyle, IpcRequest, IpcResponse, MemoryFact, PetMetadata,
    PetState, Reminder, SupportedLocale,
};
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use tracing::{error, info};
use uuid::Uuid;

// Window Layout Dimensions
pub const GUI_WIDTH: i32 = 940;
pub const GUI_HEIGHT: i32 = 640;
pub const SIDEBAR_WIDTH: i32 = 220;

// Tab Indices
pub const TAB_STATUS: usize = 0;
pub const TAB_CHAT: usize = 1;
pub const TAB_REMINDERS: usize = 2;
pub const TAB_MEMORIES: usize = 3;
pub const TAB_PRIVACY: usize = 4;
pub const TAB_SETTINGS: usize = 5;

// Cozy & Pastel Kedi Teması Palette (Win32 COLORREF format: 0x00BBGGRR)
pub const COLOR_BG_CONTENT: u32 = 0x00F9FDFF; // #FFFDF9 - Soft Cream Milk (Content Background)
pub const COLOR_BG_SIDEBAR: u32 = 0x00EAF3FF; // #FFF3EA - Soft Warm Apricot (Sidebar Background)
pub const COLOR_CARD_BG: u32 = 0x00F0F8FF; // #FFF8F0 - Cream Card Background
pub const COLOR_CARD_BORDER: u32 = 0x00DBE8FF; // #FFE8DB - Gentle Card Border
pub const COLOR_PEACH_ACCENT: u32 = 0x0076ABFF; // #FFAB76 - Warm Peach Accent
pub const COLOR_PEACH_LIGHT: u32 = 0x00B8D8FF; // #FFD8B8 - Soft Peach Pill / Active Tab
pub const COLOR_CORAL_PINK: u32 = 0x00A6AAFF; // #FFAAA6 - Pastel Pink / Coral Accent
pub const COLOR_MINT_GREEN: u32 = 0x00D0EBB8; // #B8EBD0 - Soft Mint Green
pub const COLOR_DARK_GREEN: u32 = 0x00578B2E; // #2E8B57 - Sea Green for active / online
pub const COLOR_TEXT_COCOA: u32 = 0x003D3E4A; // #4A3E3D - Espresso / Cocoa Brown Text
pub const COLOR_TEXT_MUTED: u32 = 0x00797B8D; // #8D7B79 - Muted Cocoa Subtext
pub const COLOR_DANGER_RED: u32 = 0x007576FF; // #FF7675 - Soft Red / Delete
pub const COLOR_PROGRESS_TRACK: u32 = 0x00DEE5F0; // #F0E5DE - Progress Bar Track
pub const COLOR_SKY_BLUE: u32 = 0x00FFB070; // Soft Sky Blue (Water / Susuzluk)
pub const COLOR_HONEY_GOLD: u32 = 0x0068D5F6; // Soft Honey Gold (Energy / Enerji)

// Control IDs
pub const ID_EDIT_CHAT: usize = 2201;
pub const ID_EDIT_REM_TITLE: usize = 2301;
pub const ID_EDIT_REM_BODY: usize = 2302;
pub const ID_EDIT_MEM_SEARCH: usize = 2401;

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
    pub memories: Vec<MemoryFact>,
    pub settings: AppSettings,
    pub status_feedback: String,
    pub runtime_handle: tokio::runtime::Handle,
    pub shutdown_flag: Arc<AtomicBool>,
    pub sprite_sheet: MimiSpriteSheet,
    pub realistic_sheet: RealisticCompanionSheet,
    pub frame_counter: u32,

    // Win32 child control HWNDs
    #[cfg(windows)]
    pub hwnd_main: windows_sys::Win32::Foundation::HWND,
    #[cfg(windows)]
    pub hwnd_chat_input: windows_sys::Win32::Foundation::HWND,
    #[cfg(windows)]
    pub hwnd_rem_title: windows_sys::Win32::Foundation::HWND,
    #[cfg(windows)]
    pub hwnd_rem_body: windows_sys::Win32::Foundation::HWND,
    #[cfg(windows)]
    pub hwnd_mem_search: windows_sys::Win32::Foundation::HWND,
}

#[cfg(windows)]
unsafe extern "system" fn control_center_wndproc(
    hwnd: windows_sys::Win32::Foundation::HWND,
    msg: u32,
    wparam: windows_sys::Win32::Foundation::WPARAM,
    lparam: windows_sys::Win32::Foundation::LPARAM,
) -> windows_sys::Win32::Foundation::LRESULT {
    use windows_sys::Win32::Foundation::RECT;
    use windows_sys::Win32::Graphics::Gdi::{
        BeginPaint, BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, CreateFontW, CreatePen,
        CreateSolidBrush, DeleteDC, DeleteObject, EndPaint, FillRect, InvalidateRect, LineTo,
        MoveToEx, SelectObject, SetBkColor, SetBkMode, SetTextColor, FW_BOLD, FW_NORMAL,
        PAINTSTRUCT, PS_SOLID, SRCCOPY, TRANSPARENT,
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

                // 1. Draw Sidebar Background
                let brush_sidebar = CreateSolidBrush(COLOR_BG_SIDEBAR);
                let rect_sidebar = RECT {
                    left: 0,
                    top: 0,
                    right: SIDEBAR_WIDTH,
                    bottom: GUI_HEIGHT,
                };
                FillRect(mem_dc, &rect_sidebar, brush_sidebar);
                DeleteObject(brush_sidebar);

                // 2. Draw Content Area Background
                let brush_content = CreateSolidBrush(COLOR_BG_CONTENT);
                let rect_content = RECT {
                    left: SIDEBAR_WIDTH,
                    top: 0,
                    right: GUI_WIDTH,
                    bottom: GUI_HEIGHT,
                };
                FillRect(mem_dc, &rect_content, brush_content);
                DeleteObject(brush_content);

                // 3. Draw Vertical Divider Line between sidebar and content
                let divider_pen = CreatePen(PS_SOLID, 1, COLOR_CARD_BORDER);
                let old_p = SelectObject(mem_dc, divider_pen);
                MoveToEx(mem_dc, SIDEBAR_WIDTH, 0, std::ptr::null_mut());
                LineTo(mem_dc, SIDEBAR_WIDTH, GUI_HEIGHT);
                SelectObject(mem_dc, old_p);
                DeleteObject(divider_pen);

                SetBkMode(mem_dc, TRANSPARENT as i32);

                // Modern Fonts
                let font_name: Vec<u16> = OsStr::new("Segoe UI")
                    .encode_wide()
                    .chain(std::iter::once(0))
                    .collect();

                let hfont_title = CreateFontW(
                    22,
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
                let hfont_subtitle = CreateFontW(
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
                let hfont_bold = CreateFontW(
                    15,
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
                    14,
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
                let hfont_small = CreateFontW(
                    12,
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

                // Sidebar Header: App Title & Cute Paw
                SelectObject(mem_dc, hfont_title);
                SetTextColor(mem_dc, COLOR_TEXT_COCOA);
                draw_str(mem_dc, 22, 18, "OpenPet 🐾");

                SelectObject(mem_dc, hfont_small);
                SetTextColor(mem_dc, COLOR_TEXT_MUTED);
                let sub_text = if state_guard.locale == SupportedLocale::TrTr {
                    "Masaüstü Dostu & Bakım"
                } else {
                    "Desktop Companion & Care"
                };
                draw_str(mem_dc, 24, 46, sub_text);

                // Sidebar Navigation Tabs (6 Tabs)
                let tabs = [
                    (
                        TAB_STATUS,
                        if state_guard.locale == SupportedLocale::TrTr {
                            "🐾  Durum & Bakım"
                        } else {
                            "🐾  Status & Care"
                        },
                    ),
                    (
                        TAB_CHAT,
                        if state_guard.locale == SupportedLocale::TrTr {
                            "💬  Sohbet"
                        } else {
                            "💬  Cozy Chat"
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
                        TAB_MEMORIES,
                        if state_guard.locale == SupportedLocale::TrTr {
                            "🧠  Hatıralar"
                        } else {
                            "🧠  Memories"
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
                    let y = 80 + (idx as i32 * 46);
                    let is_active = state_guard.active_tab == idx;

                    if is_active {
                        // Active tab pill
                        draw_card(
                            mem_dc,
                            12,
                            y - 4,
                            208,
                            y + 36,
                            COLOR_PEACH_LIGHT,
                            COLOR_PEACH_ACCENT,
                        );

                        // Highlight strip on left
                        let strip_brush = CreateSolidBrush(COLOR_PEACH_ACCENT);
                        let strip_rect = RECT {
                            left: 12,
                            top: y - 2,
                            right: 17,
                            bottom: y + 34,
                        };
                        FillRect(mem_dc, &strip_rect, strip_brush);
                        DeleteObject(strip_brush);

                        SelectObject(mem_dc, hfont_bold);
                        SetTextColor(mem_dc, COLOR_TEXT_COCOA);
                    } else {
                        SelectObject(mem_dc, hfont_regular);
                        SetTextColor(mem_dc, COLOR_TEXT_COCOA);
                    }

                    draw_str(mem_dc, 26, y + 5, label);
                }

                // Sidebar Footer: Connection Badge & Author
                SelectObject(mem_dc, hfont_bold);
                if state_guard.is_connected {
                    SetTextColor(mem_dc, COLOR_DARK_GREEN);
                    draw_str(mem_dc, 22, GUI_HEIGHT - 65, "● ÇEVRİMİÇİ (Host)");
                } else {
                    SetTextColor(mem_dc, COLOR_DANGER_RED);
                    draw_str(mem_dc, 22, GUI_HEIGHT - 65, "○ ÇEVRİMDIŞI");
                }

                SelectObject(mem_dc, hfont_small);
                SetTextColor(mem_dc, COLOR_TEXT_MUTED);
                draw_str(mem_dc, 22, GUI_HEIGHT - 40, "v0.1.0-alpha • Tiyatrotist");

                // Render Content View based on active_tab
                match state_guard.active_tab {
                    TAB_STATUS => render_tab_status(
                        mem_dc,
                        &state_guard,
                        hfont_title,
                        hfont_subtitle,
                        hfont_bold,
                        hfont_regular,
                    ),
                    TAB_CHAT => render_tab_chat(
                        mem_dc,
                        &state_guard,
                        hfont_title,
                        hfont_bold,
                        hfont_regular,
                    ),
                    TAB_REMINDERS => render_tab_reminders(
                        mem_dc,
                        &state_guard,
                        hfont_title,
                        hfont_bold,
                        hfont_regular,
                    ),
                    TAB_MEMORIES => render_tab_memories(
                        mem_dc,
                        &state_guard,
                        hfont_title,
                        hfont_bold,
                        hfont_regular,
                    ),
                    TAB_PRIVACY => render_tab_privacy(
                        mem_dc,
                        &state_guard,
                        hfont_title,
                        hfont_bold,
                        hfont_regular,
                    ),
                    TAB_SETTINGS => render_tab_settings(
                        mem_dc,
                        &state_guard,
                        hfont_title,
                        hfont_bold,
                        hfont_regular,
                    ),
                    _ => {}
                }

                // Blit back-buffer to screen DC
                BitBlt(hdc, 0, 0, GUI_WIDTH, GUI_HEIGHT, mem_dc, 0, 0, SRCCOPY);

                SelectObject(mem_dc, old_bmp);
                DeleteObject(hbitmap);
                DeleteObject(hfont_title);
                DeleteObject(hfont_subtitle);
                DeleteObject(hfont_bold);
                DeleteObject(hfont_regular);
                DeleteObject(hfont_small);
                DeleteDC(mem_dc);
                EndPaint(hwnd, &ps);
            }
            0
        }
        WM_CTLCOLOREDIT | WM_CTLCOLORSTATIC => {
            // Style child edit inputs and static labels with cozy cream background & cocoa text
            use windows_sys::Win32::Graphics::Gdi::CreateSolidBrush;
            let brush = CreateSolidBrush(COLOR_CARD_BG);
            let hdc = wparam as isize as *mut std::ffi::c_void;
            SetBkColor(hdc, COLOR_CARD_BG);
            SetTextColor(hdc, COLOR_TEXT_COCOA);
            brush as isize
        }
        WM_LBUTTONDOWN => {
            if !state_ptr.is_null() {
                let mut state_guard = (*state_ptr).lock().unwrap();
                let pt_x = (lparam & 0xFFFF) as i16 as i32;
                let pt_y = ((lparam >> 16) & 0xFFFF) as i16 as i32;

                // 1. Sidebar Tab Click Check
                if pt_x >= 12 && pt_x <= 208 {
                    for i in 0..6 {
                        let y = 80 + (i * 46);
                        if pt_y >= y - 4 && pt_y <= y + 36 {
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
                        // Art style toggle pills at y: 16..46
                        if pt_y >= 16 && pt_y <= 46 {
                            if pt_x >= 655 && pt_x <= 780 {
                                info!("GUI Action: Switch to Pixel Art style");
                                state_guard.settings.art_style = CompanionArtStyle::PixelArt;
                                send_ipc_async(
                                    &state_guard,
                                    IpcRequest::UpdateSettings(state_guard.settings.clone()),
                                );
                                state_guard.status_feedback = if state_guard.locale
                                    == SupportedLocale::TrTr
                                {
                                    "Görsel stil Piksel Sanatı (Mimi) olarak ayarlandı! 🎨".into()
                                } else {
                                    "Art style switched to Pixel Art (Mimi)! 🎨".into()
                                };
                                InvalidateRect(hwnd, std::ptr::null(), 0);
                            } else if pt_x >= 790 && pt_x <= 915 {
                                info!("GUI Action: Switch to Realistic (Oreo) style");
                                state_guard.settings.art_style = CompanionArtStyle::Realistic;
                                send_ipc_async(
                                    &state_guard,
                                    IpcRequest::UpdateSettings(state_guard.settings.clone()),
                                );
                                state_guard.status_feedback =
                                    if state_guard.locale == SupportedLocale::TrTr {
                                        "Görsel stil Gerçek Kedi (Oreo) olarak ayarlandı! 📸".into()
                                    } else {
                                        "Art style switched to Real Cat (Oreo)! 📸".into()
                                    };
                                InvalidateRect(hwnd, std::ptr::null(), 0);
                            }
                        }

                        // Quick Care Buttons at y = 250..290:
                        // 1. Feed: 248..373
                        if pt_y >= 250 && pt_y <= 290 && pt_x >= 248 && pt_x <= 373 {
                            info!("GUI Action: Feed companion");
                            send_ipc_async(
                                &state_guard,
                                IpcRequest::InteractPet(openpet_types::InteractionType::Feed),
                            );
                            state_guard.status_feedback =
                                if state_guard.locale == SupportedLocale::TrTr {
                                    "Mimi lezzetli balığı afiyetle yedi! 🐟".into()
                                } else {
                                    "Mimi happily ate the delicious fish! 🐟".into()
                                };
                            InvalidateRect(hwnd, std::ptr::null(), 0);
                        }
                        // 2. Water: 383..508
                        if pt_y >= 250 && pt_y <= 290 && pt_x >= 383 && pt_x <= 508 {
                            info!("GUI Action: Water companion");
                            send_ipc_async(
                                &state_guard,
                                IpcRequest::InteractPet(openpet_types::InteractionType::Water),
                            );
                            state_guard.status_feedback =
                                if state_guard.locale == SupportedLocale::TrTr {
                                    "Mimi taze ve serin suyu lıkır lıkır içti! 💧".into()
                                } else {
                                    "Mimi happily lapped up fresh water! 💧".into()
                                };
                            InvalidateRect(hwnd, std::ptr::null(), 0);
                        }
                        // 3. Groom: 518..643
                        if pt_y >= 250 && pt_y <= 290 && pt_x >= 518 && pt_x <= 643 {
                            info!("GUI Action: Groom companion");
                            send_ipc_async(
                                &state_guard,
                                IpcRequest::InteractPet(openpet_types::InteractionType::Groom),
                            );
                            state_guard.status_feedback =
                                if state_guard.locale == SupportedLocale::TrTr {
                                    "Mimi keyifle mırıldanarak tarandı, tüyleri yumuşacık oldu! ✨"
                                        .into()
                                } else {
                                    "Mimi purred as you brushed its coat! ✨".into()
                                };
                            InvalidateRect(hwnd, std::ptr::null(), 0);
                        }
                        // 4. Play: 653..778
                        if pt_y >= 250 && pt_y <= 290 && pt_x >= 653 && pt_x <= 778 {
                            info!("GUI Action: Play with companion");
                            send_ipc_async(
                                &state_guard,
                                IpcRequest::InteractPet(openpet_types::InteractionType::Play),
                            );
                            state_guard.status_feedback =
                                if state_guard.locale == SupportedLocale::TrTr {
                                    "Mimi neşeyle zıplayarak oynamaya başladı! 🎾".into()
                                } else {
                                    "Mimi jumped excitedly to play with you! 🎾".into()
                                };
                            InvalidateRect(hwnd, std::ptr::null(), 0);
                        }
                        // 5. Pet: 788..913
                        if pt_y >= 250 && pt_y <= 290 && pt_x >= 788 && pt_x <= 913 {
                            info!("GUI Action: Pet companion");
                            send_ipc_async(
                                &state_guard,
                                IpcRequest::InteractPet(openpet_types::InteractionType::Petting {
                                    intensity: 1.0,
                                }),
                            );
                            state_guard.status_feedback =
                                if state_guard.locale == SupportedLocale::TrTr {
                                    "Mimi mırıldanarak sevgine karşılık verdi! ❤️".into()
                                } else {
                                    "Mimi purred warmly under your gentle touch! ❤️".into()
                                };
                            InvalidateRect(hwnd, std::ptr::null(), 0);
                        }

                        // Breed Selector Chips at y = 365..400:
                        if pt_y >= 365 && pt_y <= 400 {
                            let breeds = [
                                (265, 345, CatBreed::Tabby),
                                (353, 438, CatBreed::Tuxedo),
                                (446, 521, CatBreed::Calico),
                                (529, 614, CatBreed::Ginger),
                                (622, 702, CatBreed::Siamese),
                                (710, 785, CatBreed::Black),
                                (793, 873, CatBreed::White),
                            ];
                            for (x0, x1, breed) in breeds {
                                if pt_x >= x0 && pt_x <= x1 {
                                    state_guard.settings.cat_breed = breed;
                                    state_guard.sprite_sheet =
                                        MimiSpriteSheet::generate_for_breed(breed);
                                    send_ipc_async(
                                        &state_guard,
                                        IpcRequest::UpdateSettings(state_guard.settings.clone()),
                                    );
                                    state_guard.status_feedback =
                                        if state_guard.locale == SupportedLocale::TrTr {
                                            format!(
                                                "Kedi ırkı değiştirildi: {} 🐾",
                                                breed.display_name(true)
                                            )
                                        } else {
                                            format!(
                                                "Companion breed switched to: {} 🐾",
                                                breed.display_name(false)
                                            )
                                        };
                                    InvalidateRect(hwnd, std::ptr::null(), 0);
                                    break;
                                }
                            }
                        }

                        // Launch Host Button if offline at (265..530, 535..575)
                        if !state_guard.is_connected
                            && pt_x >= 265
                            && pt_x <= 530
                            && pt_y >= 535
                            && pt_y <= 575
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
                        // Quick conversational chips at y = 478..510
                        if pt_y >= 478 && pt_y <= 510 {
                            let chips = if state_guard.locale == SupportedLocale::TrTr {
                                [
                                    (248, 365, "Nasılsın? 🐾"),
                                    (373, 513, "Seni seviyorum! ❤️"),
                                    (521, 666, "Mola verelim mi? ☕"),
                                    (674, 804, "Balık ister misin? 🐟"),
                                    (812, 915, "Oyun oynayalım! 🧶"),
                                ]
                            } else {
                                [
                                    (248, 365, "How are you? 🐾"),
                                    (373, 513, "I love you! ❤️"),
                                    (521, 666, "Take a break? ☕"),
                                    (674, 804, "Want a fish? 🐟"),
                                    (812, 915, "Let's play! 🧶"),
                                ]
                            };

                            for (x0, x1, text) in chips {
                                if pt_x >= x0 && pt_x <= x1 {
                                    send_chat_message_text(&mut state_guard, text.to_string());
                                    InvalidateRect(hwnd, std::ptr::null(), 0);
                                    break;
                                }
                            }
                        }

                        // Send Button at (818..915, 525..563)
                        if pt_x >= 818 && pt_x <= 915 && pt_y >= 525 && pt_y <= 563 {
                            handle_send_chat(&mut state_guard);
                            InvalidateRect(hwnd, std::ptr::null(), 0);
                        }
                    }
                    TAB_REMINDERS => {
                        // Quick preset reminder chips at y = 70..102
                        if pt_y >= 70 && pt_y <= 102 {
                            let is_tr = state_guard.locale == SupportedLocale::TrTr;
                            let presets = [
                                (
                                    248,
                                    395,
                                    if is_tr {
                                        "💧 Su Vakti!"
                                    } else {
                                        "💧 Hydration Time!"
                                    },
                                    if is_tr {
                                        "Bir bardak serin su içip ferahla."
                                    } else {
                                        "Drink a fresh glass of water."
                                    },
                                    30,
                                ),
                                (
                                    405,
                                    575,
                                    if is_tr {
                                        "🧘 Postür Düzelt!"
                                    } else {
                                        "🧘 Posture Check!"
                                    },
                                    if is_tr {
                                        "Omuzlarını geriye al ve dik otur."
                                    } else {
                                        "Roll shoulders back and sit upright."
                                    },
                                    45,
                                ),
                                (
                                    585,
                                    750,
                                    if is_tr {
                                        "👀 20-20-20 Kuralı!"
                                    } else {
                                        "👀 Rest Your Eyes!"
                                    },
                                    if is_tr {
                                        "20 saniye boyunca 6 metre uzağa bak."
                                    } else {
                                        "Look 20 feet away for 20 seconds."
                                    },
                                    20,
                                ),
                                (
                                    760,
                                    915,
                                    if is_tr {
                                        "🚶 Kısa Bir Mola!"
                                    } else {
                                        "🚶 Quick Stretch!"
                                    },
                                    if is_tr {
                                        "Ayağa kalk, esne ve birkaç adım yürü."
                                    } else {
                                        "Stand up, stretch and walk around."
                                    },
                                    60,
                                ),
                            ];

                            for (x0, x1, title, body, mins) in presets {
                                if pt_x >= x0 && pt_x <= x1 {
                                    schedule_quick_reminder(&mut state_guard, title, body, mins);
                                    InvalidateRect(hwnd, std::ptr::null(), 0);
                                    break;
                                }
                            }
                        }

                        // Delete reminder buttons [❌ Sil]
                        // Toggle reminder button [Aktif / Kapalı]
                        let rem_count = state_guard.reminders.len().min(5);
                        for idx in 0..rem_count {
                            let y = 125 + (idx as i32 * 64);
                            if pt_x >= 740 && pt_x <= 825 && pt_y >= y + 10 && pt_y <= y + 42 {
                                let rem_id = state_guard.reminders[idx].id;
                                state_guard.reminders[idx].enabled =
                                    !state_guard.reminders[idx].enabled;
                                send_ipc_async(&state_guard, IpcRequest::ToggleReminder(rem_id));
                                InvalidateRect(hwnd, std::ptr::null(), 0);
                                break;
                            }
                        }

                        // Delete reminder buttons [❌ Sil]
                        for idx in 0..rem_count {
                            let y = 125 + (idx as i32 * 64);
                            if pt_x >= 835 && pt_x <= 895 && pt_y >= y + 10 && pt_y <= y + 42 {
                                let rem_id = state_guard.reminders[idx].id;
                                send_ipc_async(&state_guard, IpcRequest::DeleteReminder(rem_id));
                                state_guard.reminders.remove(idx);
                                InvalidateRect(hwnd, std::ptr::null(), 0);
                                break;
                            }
                        }

                        // Add Custom Reminder Button at (705..898, 504..574)
                        if pt_x >= 705 && pt_x <= 898 && pt_y >= 504 && pt_y <= 574 {
                            handle_add_reminder(&mut state_guard);
                            InvalidateRect(hwnd, std::ptr::null(), 0);
                        }
                    }
                    TAB_MEMORIES => {
                        // Clear / All search buttons at (740..820 and 828..895, y: 72..100)
                        if pt_y >= 72
                            && pt_y <= 100
                            && ((pt_x >= 740 && pt_x <= 820) || (pt_x >= 828 && pt_x <= 895))
                        {
                            use windows_sys::Win32::UI::WindowsAndMessaging::SetWindowTextW;
                            let empty: [u16; 1] = [0];
                            unsafe {
                                SetWindowTextW(state_guard.hwnd_mem_search, empty.as_ptr());
                            }
                            InvalidateRect(hwnd, std::ptr::null(), 0);
                        }

                        // Delete memory button [❌ Unut] for filtered items
                        let query = get_mem_search_query(state_guard.hwnd_mem_search);
                        let filtered_ids: Vec<Uuid> = state_guard
                            .memories
                            .iter()
                            .filter(|m| {
                                if query.is_empty() {
                                    true
                                } else {
                                    m.subject.to_lowercase().contains(&query)
                                        || m.predicate.to_lowercase().contains(&query)
                                        || m.object.to_lowercase().contains(&query)
                                }
                            })
                            .map(|m| m.id)
                            .collect();

                        let count = filtered_ids.len().min(5);
                        for (idx, mem_id) in filtered_ids.iter().take(count).enumerate() {
                            let y = 122 + (idx as i32 * 68);
                            if pt_x >= 825 && pt_x <= 895 && pt_y >= y + 12 && pt_y <= y + 48 {
                                let mem_id = *mem_id;
                                send_ipc_async(&state_guard, IpcRequest::DeleteMemory(mem_id));
                                state_guard.memories.retain(|m| m.id != mem_id);
                                InvalidateRect(hwnd, std::ptr::null(), 0);
                                break;
                            }
                        }
                    }
                    TAB_PRIVACY => {
                        // Toggle Privacy Mode Button at (248..680, 340..405)
                        if pt_x >= 248 && pt_x <= 680 && pt_y >= 340 && pt_y <= 405 {
                            let next = !state_guard.privacy_mode;
                            state_guard.privacy_mode = next;
                            state_guard.settings.privacy_mode = next;
                            info!("GUI Action: Toggled privacy mode to {}", next);
                            send_ipc_async(&state_guard, IpcRequest::SetPrivacyMode(next));
                            InvalidateRect(hwnd, std::ptr::null(), 0);
                        }
                    }
                    TAB_SETTINGS => {
                        // Interface Language Buttons (y: 92..132)
                        // Türkçe at (248..410)
                        if pt_x >= 248 && pt_x <= 410 && pt_y >= 92 && pt_y <= 132 {
                            state_guard.locale = SupportedLocale::TrTr;
                            state_guard.i18n.set_locale(SupportedLocale::TrTr);
                            state_guard.settings.locale = SupportedLocale::TrTr;
                            send_ipc_async(
                                &state_guard,
                                IpcRequest::UpdateSettings(state_guard.settings.clone()),
                            );
                            InvalidateRect(hwnd, std::ptr::null(), 0);
                        }
                        // English at (425..585)
                        if pt_x >= 425 && pt_x <= 585 && pt_y >= 92 && pt_y <= 132 {
                            state_guard.locale = SupportedLocale::EnUs;
                            state_guard.i18n.set_locale(SupportedLocale::EnUs);
                            state_guard.settings.locale = SupportedLocale::EnUs;
                            send_ipc_async(
                                &state_guard,
                                IpcRequest::UpdateSettings(state_guard.settings.clone()),
                            );
                            InvalidateRect(hwnd, std::ptr::null(), 0);
                        }

                        // Art Style Buttons at y: 170..210:
                        // Pixel Art (248..470)
                        if pt_x >= 248 && pt_x <= 470 && pt_y >= 170 && pt_y <= 210 {
                            state_guard.settings.art_style = CompanionArtStyle::PixelArt;
                            send_ipc_async(
                                &state_guard,
                                IpcRequest::UpdateSettings(state_guard.settings.clone()),
                            );
                            InvalidateRect(hwnd, std::ptr::null(), 0);
                        }
                        // Realistic Oreo (485..710)
                        if pt_x >= 485 && pt_x <= 710 && pt_y >= 170 && pt_y <= 210 {
                            state_guard.settings.art_style = CompanionArtStyle::Realistic;
                            send_ipc_async(
                                &state_guard,
                                IpcRequest::UpdateSettings(state_guard.settings.clone()),
                            );
                            InvalidateRect(hwnd, std::ptr::null(), 0);
                        }

                        // Cat Breed Buttons at y: 248..285:
                        if pt_y >= 248 && pt_y <= 285 {
                            let breeds = [
                                (248, 328, CatBreed::Tabby),
                                (336, 421, CatBreed::Tuxedo),
                                (429, 504, CatBreed::Calico),
                                (512, 597, CatBreed::Ginger),
                                (605, 685, CatBreed::Siamese),
                                (693, 768, CatBreed::Black),
                                (776, 856, CatBreed::White),
                            ];
                            for (x0, x1, breed) in breeds {
                                if pt_x >= x0 && pt_x <= x1 {
                                    state_guard.settings.cat_breed = breed;
                                    state_guard.sprite_sheet =
                                        MimiSpriteSheet::generate_for_breed(breed);
                                    send_ipc_async(
                                        &state_guard,
                                        IpcRequest::UpdateSettings(state_guard.settings.clone()),
                                    );
                                    InvalidateRect(hwnd, std::ptr::null(), 0);
                                    break;
                                }
                            }
                        }

                        // Always On Top button (730..895, 342..378)
                        if pt_x >= 730 && pt_x <= 895 && pt_y >= 342 && pt_y <= 378 {
                            let next = !state_guard.settings.always_on_top;
                            state_guard.settings.always_on_top = next;
                            send_ipc_async(
                                &state_guard,
                                IpcRequest::UpdateSettings(state_guard.settings.clone()),
                            );
                            InvalidateRect(hwnd, std::ptr::null(), 0);
                        }

                        // Privacy Mode button (730..895, 392..428)
                        if pt_x >= 730 && pt_x <= 895 && pt_y >= 392 && pt_y <= 428 {
                            let next = !state_guard.privacy_mode;
                            state_guard.privacy_mode = next;
                            state_guard.settings.privacy_mode = next;
                            send_ipc_async(&state_guard, IpcRequest::SetPrivacyMode(next));
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
            // Animate pet frames and periodically poll host state
            if !state_ptr.is_null() {
                let mut state_guard = (*state_ptr).lock().unwrap();
                state_guard.frame_counter = state_guard.frame_counter.wrapping_add(1);
                if state_guard.frame_counter % 2 == 0 {
                    poll_host_updates(&mut state_guard);
                }
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
    hfont_subtitle: windows_sys::Win32::Graphics::Gdi::HFONT,
    hfont_bold: windows_sys::Win32::Graphics::Gdi::HFONT,
    hfont_regular: windows_sys::Win32::Graphics::Gdi::HFONT,
) {
    use windows_sys::Win32::Graphics::Gdi::{SelectObject, SetTextColor};

    SelectObject(hdc, hfont_title);
    SetTextColor(hdc, COLOR_TEXT_COCOA);
    let title_text = if state.locale == SupportedLocale::TrTr {
        "🐾  Kedi Durumu & İhtiyaçlar"
    } else {
        "🐾  Companion Overview & Care"
    };
    draw_str(hdc, 248, 20, title_text);

    // Art Style Switcher Pills at Top Right (x: 655..780, 790..915, y: 16..46)
    let is_pixel = state.settings.art_style == CompanionArtStyle::PixelArt;
    let (bg_pixel, bdr_pixel) = if is_pixel {
        (COLOR_PEACH_LIGHT, COLOR_PEACH_ACCENT)
    } else {
        (COLOR_CARD_BG, COLOR_CARD_BORDER)
    };
    let (bg_real, bdr_real) = if !is_pixel {
        (COLOR_PEACH_LIGHT, COLOR_PEACH_ACCENT)
    } else {
        (COLOR_CARD_BG, COLOR_CARD_BORDER)
    };
    let lbl_pixel = if state.locale == SupportedLocale::TrTr {
        "🎨  Piksel"
    } else {
        "🎨  Pixel Art"
    };
    let lbl_real = if state.locale == SupportedLocale::TrTr {
        "📸  Gerçek (Oreo)"
    } else {
        "📸  Real Cat"
    };
    draw_button(
        hdc,
        655,
        16,
        780,
        46,
        lbl_pixel,
        bg_pixel,
        bdr_pixel,
        COLOR_TEXT_COCOA,
        hfont_bold,
    );
    draw_button(
        hdc,
        790,
        16,
        915,
        46,
        lbl_real,
        bg_real,
        bdr_real,
        COLOR_TEXT_COCOA,
        hfont_bold,
    );

    // Main Companion Card (x: 248..915, y: 55..235)
    draw_card(hdc, 248, 55, 915, 235, COLOR_CARD_BG, COLOR_CARD_BORDER);

    // Draw Animated Scaled Cat Sprite inside Card at (262, 65)
    draw_cat_sprite(hdc, 262, 65, state);

    // Breed Label under sprite
    SelectObject(hdc, hfont_bold);
    SetTextColor(hdc, COLOR_TEXT_MUTED);
    let is_tr = state.locale == SupportedLocale::TrTr;
    let breed_name = if state.settings.art_style == CompanionArtStyle::Realistic {
        if is_tr {
            "Bıyıklı Smokin (Oreo)".to_string()
        } else {
            "Mustache Tuxedo (Oreo)".to_string()
        }
    } else {
        format!("🐱 {}", state.settings.cat_breed.display_name(is_tr))
    };
    draw_str(hdc, 266, 200, &breed_name);

    // Pet Info on Right Side of Card
    SelectObject(hdc, hfont_subtitle);
    SetTextColor(hdc, COLOR_TEXT_COCOA);
    let pet_name = if state.settings.art_style == CompanionArtStyle::Realistic {
        "Oreo"
    } else {
        state
            .active_pet
            .as_ref()
            .map(|p| p.name.as_str())
            .unwrap_or("Mimi")
    };
    draw_str(hdc, 410, 68, pet_name);

    // Current Behavior Badge
    let (behavior_desc, _) = get_pet_mood_and_frame(state);

    draw_card(hdc, 480, 68, 680, 92, COLOR_PEACH_LIGHT, COLOR_PEACH_ACCENT);
    SelectObject(hdc, hfont_bold);
    SetTextColor(hdc, COLOR_TEXT_COCOA);
    draw_str(hdc, 492, 72, behavior_desc);

    // 5 Live Needs Progress Bars
    let (hunger, thirst, energy, bond, hygiene) = if let Some(ref st) = state.pet_state {
        (st.hunger, st.thirst, st.energy, st.mood, st.hygiene)
    } else {
        (0.75, 0.85, 0.65, 0.90, 0.80)
    };

    let bars = [
        (
            104,
            hunger,
            COLOR_PEACH_ACCENT,
            if state.locale == SupportedLocale::TrTr {
                "🐟 Açlık"
            } else {
                "🐟 Hunger"
            },
        ),
        (
            128,
            thirst,
            COLOR_SKY_BLUE,
            if state.locale == SupportedLocale::TrTr {
                "💧 Susuzluk"
            } else {
                "💧 Thirst"
            },
        ),
        (
            152,
            energy,
            COLOR_HONEY_GOLD,
            if state.locale == SupportedLocale::TrTr {
                "⚡ Enerji"
            } else {
                "⚡ Energy"
            },
        ),
        (
            176,
            bond,
            COLOR_CORAL_PINK,
            if state.locale == SupportedLocale::TrTr {
                "❤️ Sevgi / Bağ"
            } else {
                "❤️ Affection"
            },
        ),
        (
            200,
            hygiene,
            COLOR_MINT_GREEN,
            if state.locale == SupportedLocale::TrTr {
                "✨ Temizlik"
            } else {
                "✨ Hygiene"
            },
        ),
    ];

    for (y, val, color, label) in bars {
        draw_progress_bar(
            hdc,
            530,
            y + 2,
            835,
            y + 14,
            val,
            color,
            label,
            &format!("{:.0}%", val * 100.0),
            hfont_regular,
        );
    }

    // Quick Care Buttons at y = 250..290 (5 cute buttons)
    let care_buttons = [
        (
            248,
            373,
            if state.locale == SupportedLocale::TrTr {
                "🐟 Mama Ver"
            } else {
                "🐟 Feed"
            },
        ),
        (
            383,
            508,
            if state.locale == SupportedLocale::TrTr {
                "💧 Su Ver"
            } else {
                "💧 Water"
            },
        ),
        (
            518,
            643,
            if state.locale == SupportedLocale::TrTr {
                "🪮 Tara"
            } else {
                "🪮 Groom"
            },
        ),
        (
            653,
            778,
            if state.locale == SupportedLocale::TrTr {
                "🎾 Oynat"
            } else {
                "🎾 Play"
            },
        ),
        (
            788,
            913,
            if state.locale == SupportedLocale::TrTr {
                "❤️ Sev"
            } else {
                "❤️ Pet"
            },
        ),
    ];

    for (x0, x1, lbl) in care_buttons {
        draw_button(
            hdc,
            x0,
            250,
            x1,
            290,
            lbl,
            COLOR_CARD_BG,
            COLOR_PEACH_ACCENT,
            COLOR_TEXT_COCOA,
            hfont_bold,
        );
    }

    // Feedback message
    if !state.status_feedback.is_empty() {
        SelectObject(hdc, hfont_bold);
        SetTextColor(hdc, COLOR_DARK_GREEN);
        draw_str(hdc, 250, 298, &state.status_feedback);
    }

    // Breed Selector Card at y = 325..445
    draw_card(hdc, 248, 325, 915, 445, COLOR_CARD_BG, COLOR_CARD_BORDER);

    SelectObject(hdc, hfont_bold);
    SetTextColor(hdc, COLOR_TEXT_COCOA);
    let breed_title = if state.locale == SupportedLocale::TrTr {
        "🐱 Gerçek Kedi Irkları & Kürk Görünümleri"
    } else {
        "🐱 Domestic Feline Breeds & Coat Palettes"
    };
    draw_str(hdc, 265, 335, breed_title);

    let breeds = [
        (265, 345, CatBreed::Tabby, "Tekir"),
        (353, 438, CatBreed::Tuxedo, "Smokin"),
        (446, 521, CatBreed::Calico, "Calico"),
        (529, 614, CatBreed::Ginger, "Sarıman"),
        (622, 702, CatBreed::Siamese, "Siyam"),
        (710, 785, CatBreed::Black, "Siyah"),
        (793, 873, CatBreed::White, "Beyaz"),
    ];

    for (x0, x1, brd, name) in breeds {
        let is_selected = state.settings.cat_breed == brd;
        let bg = if is_selected {
            COLOR_PEACH_ACCENT
        } else {
            COLOR_CARD_BG
        };
        let border = if is_selected {
            COLOR_TEXT_COCOA
        } else {
            COLOR_CARD_BORDER
        };
        draw_button(
            hdc,
            x0,
            365,
            x1,
            400,
            name,
            bg,
            border,
            COLOR_TEXT_COCOA,
            hfont_bold,
        );
    }

    SelectObject(hdc, hfont_regular);
    SetTextColor(hdc, COLOR_TEXT_MUTED);
    let breed_info = match state.settings.cat_breed {
        CatBreed::Tabby => {
            if state.locale == SupportedLocale::TrTr {
                "Klasik sevimli sokak kedisi tekir çizgileri ve yeşil gözler 🐾"
            } else {
                "Classic friendly domestic tabby with striped coat & emerald eyes 🐾"
            }
        }
        CatBreed::Tuxedo => {
            if state.locale == SupportedLocale::TrTr {
                "Siyah smokin frak ve beyaz göğüs deseni, sarı-yeşil gözler 🎩"
            } else {
                "Dapper black tuxedo with white chest and yellow-green eyes 🎩"
            }
        }
        CatBreed::Calico => {
            if state.locale == SupportedLocale::TrTr {
                "Üç renkli (beyaz, turuncu, siyah) şans getiren Japon Bobtail deseni 🍀"
            } else {
                "Tri-color calico coat traditionally celebrated for good luck 🍀"
            }
        }
        CatBreed::Ginger => {
            if state.locale == SupportedLocale::TrTr {
                "Sıcak turuncu tekir çizgileri, obur ve oyuncu sarıman ☀️"
            } else {
                "Warm orange mackerel tabby, sweet and playful marmalade cat ☀️"
            }
        }
        CatBreed::Siamese => {
            if state.locale == SupportedLocale::TrTr {
                "Açık krem gövde, koyu kahve maske ve derin safir mavi gözler 💎"
            } else {
                "Cream body with seal points and deep sapphire blue eyes 💎"
            }
        }
        CatBreed::Black => {
            if state.locale == SupportedLocale::TrTr {
                "Parlak ipeksi gece siyahı kürk ve büyüleyici altın sarısı gözler 🌙"
            } else {
                "Silky midnight black coat with radiant amber eyes 🌙"
            }
        }
        CatBreed::White => {
            if state.locale == SupportedLocale::TrTr {
                "Pamuk gibi bembeyaz kürk ve parıltılı turkuaz gözler ❄️"
            } else {
                "Pristine snowy white coat with bright turquoise eyes ❄️"
            }
        }
    };
    draw_str(hdc, 265, 412, breed_info);

    // Host Status / Launcher Card at y = 460..605
    draw_card(hdc, 248, 460, 915, 605, COLOR_CARD_BG, COLOR_CARD_BORDER);
    if !state.is_connected {
        SelectObject(hdc, hfont_bold);
        SetTextColor(hdc, COLOR_DANGER_RED);
        let notice_title = if state.locale == SupportedLocale::TrTr {
            "⚠️ OpenPet Host Arka Planda Çalışmıyor"
        } else {
            "⚠️ OpenPet Host is Offline"
        };
        draw_str(hdc, 265, 475, notice_title);

        SelectObject(hdc, hfont_regular);
        SetTextColor(hdc, COLOR_TEXT_MUTED);
        let notice_body = if state.locale == SupportedLocale::TrTr {
            "Masaüstünde yüzen kedi penceresini ve yerel IPC servislerini başlatmak için tıklayın:"
        } else {
            "Click below to launch the floating companion pet window and local IPC services:"
        };
        draw_str(hdc, 265, 502, notice_body);

        draw_button(
            hdc,
            265,
            535,
            530,
            575,
            if state.locale == SupportedLocale::TrTr {
                "🚀 Masaüstü Petini Başlat"
            } else {
                "🚀 Launch Desktop Pet"
            },
            COLOR_PEACH_ACCENT,
            COLOR_CARD_BORDER,
            COLOR_TEXT_COCOA,
            hfont_bold,
        );
    } else {
        SelectObject(hdc, hfont_bold);
        SetTextColor(hdc, COLOR_DARK_GREEN);
        let active_title = if state.locale == SupportedLocale::TrTr {
            "✨ Masaüstü Pet Penceresi Aktif ve Bağlı"
        } else {
            "✨ Desktop Companion Window is Active & Connected"
        };
        draw_str(hdc, 265, 475, active_title);

        SelectObject(hdc, hfont_regular);
        SetTextColor(hdc, COLOR_TEXT_COCOA);
        let tip1 = if state.locale == SupportedLocale::TrTr {
            "• Çift Tıklama: Masaüstündeki pete çift tıklayarak bu Kontrol Merkezini açabilirsiniz."
        } else {
            "• Double Click: Double clicking the desktop pet brings up this Control Center."
        };
        let tip2 = if state.locale == SupportedLocale::TrTr {
            "• Konuşma Baloncuğu: Petin başı üzerinde anlık masaüstü tepkileri ve konuşmalar gösterilir."
        } else {
            "• Speech Bubble: Instant desktop thoughts and reactions appear in a cozy speech bubble above the pet."
        };
        let tip3 = if state.locale == SupportedLocale::TrTr {
            "• Şeffaf Tıklama Geçirgenliği: Petin çevresindeki boş alanlar fare tıklamalarını arkadaki pencerelere iletir."
        } else {
            "• Click-Through Transparency: Empty areas around the cat allow seamless clicking into background windows."
        };
        draw_str(hdc, 265, 505, tip1);
        draw_str(hdc, 265, 532, tip2);
        draw_str(hdc, 265, 560, tip3);
    }
}

#[cfg(windows)]
unsafe fn render_tab_chat(
    hdc: windows_sys::Win32::Graphics::Gdi::HDC,
    state: &ControlCenterState,
    hfont_title: windows_sys::Win32::Graphics::Gdi::HFONT,
    hfont_bold: windows_sys::Win32::Graphics::Gdi::HFONT,
    hfont_regular: windows_sys::Win32::Graphics::Gdi::HFONT,
) {
    use windows_sys::Win32::Graphics::Gdi::{SelectObject, SetTextColor};

    SelectObject(hdc, hfont_title);
    SetTextColor(hdc, COLOR_TEXT_COCOA);
    let title = if state.locale == SupportedLocale::TrTr {
        "💬  Mimi ile Sohbet"
    } else {
        "💬  Chat with Mimi"
    };
    draw_str(hdc, 248, 20, title);

    // Companion Mood Badge at Top Right
    draw_card(hdc, 750, 18, 915, 46, COLOR_MINT_GREEN, COLOR_CARD_BORDER);
    SelectObject(hdc, hfont_bold);
    SetTextColor(hdc, COLOR_TEXT_COCOA);
    let mood_badge = if state.locale == SupportedLocale::TrTr {
        "Ruh Hali: Mutlu ✨"
    } else {
        "Mood: Happy ✨"
    };
    draw_str(hdc, 765, 23, mood_badge);

    // Chat History Box (x: 248..915, y: 55..465)
    draw_card(hdc, 248, 55, 915, 465, COLOR_CARD_BG, COLOR_CARD_BORDER);

    SelectObject(hdc, hfont_regular);
    if state.chat_history.is_empty() {
        SetTextColor(hdc, COLOR_TEXT_MUTED);
        let empty_msg = if state.locale == SupportedLocale::TrTr {
            "Henüz mesaj yok. Mimi ile konuşmak için aşağıdan bir şeyler yazın veya hızlı butonlara tıklayın!"
        } else {
            "No messages yet. Type below or click one of the quick chips to chat with Mimi!"
        };
        draw_str(hdc, 270, 75, empty_msg);
    } else {
        let mut y = 70;
        for (sender, msg) in state.chat_history.iter().rev().take(8).rev() {
            if sender == "User" {
                // User chat bubble (right-aligned, warm peach fill)
                draw_card(
                    hdc,
                    450,
                    y,
                    895,
                    y + 40,
                    COLOR_PEACH_LIGHT,
                    COLOR_PEACH_ACCENT,
                );
                SelectObject(hdc, hfont_bold);
                SetTextColor(hdc, COLOR_TEXT_COCOA);
                draw_str(hdc, 465, y + 4, "Sen: ");
                SelectObject(hdc, hfont_regular);
                draw_str(hdc, 505, y + 4, msg);
            } else {
                // Mimi chat bubble (left-aligned, milk white fill)
                draw_card(
                    hdc,
                    265,
                    y,
                    750,
                    y + 40,
                    COLOR_BG_CONTENT,
                    COLOR_CARD_BORDER,
                );
                SelectObject(hdc, hfont_bold);
                SetTextColor(hdc, COLOR_DARK_GREEN);
                draw_str(hdc, 280, y + 4, "🐾 Mimi: ");
                SelectObject(hdc, hfont_regular);
                SetTextColor(hdc, COLOR_TEXT_COCOA);
                draw_str(hdc, 345, y + 4, msg);
            }
            y += 48;
            if y > 420 {
                break;
            }
        }
    }

    // Quick Conversational Chips at y = 478..510
    let chips = if state.locale == SupportedLocale::TrTr {
        [
            (248, 365, "Nasılsın? 🐾"),
            (373, 513, "Seni seviyorum! ❤️"),
            (521, 666, "Mola verelim mi? ☕"),
            (674, 804, "Balık ister misin? 🐟"),
            (812, 915, "Oyun oynayalım! 🧶"),
        ]
    } else {
        [
            (248, 365, "How are you? 🐾"),
            (373, 513, "I love you! ❤️"),
            (521, 666, "Take a break? ☕"),
            (674, 804, "Want a fish? 🐟"),
            (812, 915, "Let's play! 🧶"),
        ]
    };

    for (x0, x1, text) in chips {
        draw_button(
            hdc,
            x0,
            478,
            x1,
            510,
            text,
            COLOR_CARD_BG,
            COLOR_CARD_BORDER,
            COLOR_TEXT_COCOA,
            hfont_bold,
        );
    }

    // Send Button at (818..915, 525..563)
    let send_lbl = if state.locale == SupportedLocale::TrTr {
        "Gönder 🐾"
    } else {
        "Send 🐾"
    };
    draw_button(
        hdc,
        818,
        525,
        915,
        563,
        send_lbl,
        COLOR_PEACH_ACCENT,
        COLOR_CARD_BORDER,
        COLOR_TEXT_COCOA,
        hfont_bold,
    );
}

#[cfg(windows)]
unsafe fn render_tab_reminders(
    hdc: windows_sys::Win32::Graphics::Gdi::HDC,
    state: &ControlCenterState,
    hfont_title: windows_sys::Win32::Graphics::Gdi::HFONT,
    hfont_bold: windows_sys::Win32::Graphics::Gdi::HFONT,
    hfont_regular: windows_sys::Win32::Graphics::Gdi::HFONT,
) {
    use windows_sys::Win32::Graphics::Gdi::{SelectObject, SetTextColor};

    SelectObject(hdc, hfont_title);
    SetTextColor(hdc, COLOR_TEXT_COCOA);
    let title = if state.locale == SupportedLocale::TrTr {
        "⏰  Masaüstü Hatırlatıcıları"
    } else {
        "⏰  Desktop Reminders"
    };
    draw_str(hdc, 248, 20, title);

    SelectObject(hdc, hfont_regular);
    SetTextColor(hdc, COLOR_TEXT_MUTED);
    let sub = if state.locale == SupportedLocale::TrTr {
        "Masaüstünde Mimi ile sağlıklı ve odaklı çalışma hatırlatıcıları"
    } else {
        "Gentle reminders from Mimi on your desktop to stay healthy & focused"
    };
    draw_str(hdc, 248, 46, sub);

    // Quick Preset Chips at y = 70..102
    let presets = [
        (248, 395, "💧 Su İç (30 dk)"),
        (405, 575, "🧘 Postür Düzelt (45 dk)"),
        (585, 750, "👀 Göz Dinlendir (20 dk)"),
        (760, 915, "🚶 Kısa Yürüyüş (60 dk)"),
    ];
    for (x0, x1, text) in presets {
        draw_button(
            hdc,
            x0,
            70,
            x1,
            102,
            text,
            COLOR_CARD_BG,
            COLOR_PEACH_ACCENT,
            COLOR_TEXT_COCOA,
            hfont_bold,
        );
    }

    // Reminders List Box (x: 248..915, y: 112..450)
    draw_card(hdc, 248, 112, 915, 450, COLOR_CARD_BG, COLOR_CARD_BORDER);

    if state.reminders.is_empty() {
        SelectObject(hdc, hfont_regular);
        SetTextColor(hdc, COLOR_TEXT_MUTED);
        let empty_str = if state.locale == SupportedLocale::TrTr {
            "Kayıtlı hatırlatıcı yok. Yukarıdaki hızlı butonları veya aşağıdaki formu kullanabilirsiniz."
        } else {
            "No reminders scheduled. Use the quick presets above or the form below to add one."
        };
        draw_str(hdc, 265, 130, empty_str);
    } else {
        let count = state.reminders.len().min(5);
        for idx in 0..count {
            let r = &state.reminders[idx];
            let y = 125 + (idx as i32 * 64);

            // Card background for each reminder
            draw_card(
                hdc,
                260,
                y,
                903,
                y + 56,
                COLOR_BG_CONTENT,
                COLOR_CARD_BORDER,
            );

            SelectObject(hdc, hfont_bold);
            SetTextColor(hdc, COLOR_TEXT_COCOA);
            draw_str(hdc, 275, y + 6, &format!("📌 {}", r.title));

            SelectObject(hdc, hfont_regular);
            SetTextColor(hdc, COLOR_TEXT_COCOA);
            draw_str(hdc, 275, y + 28, &r.body);

            SelectObject(hdc, hfont_regular);
            SetTextColor(hdc, COLOR_TEXT_MUTED);
            draw_str(
                hdc,
                640,
                y + 6,
                &format!("⏰ {}", r.schedule.format("%H:%M")),
            );

            // Toggle Button [Aktif ✓] / [Kapalı ⏸]
            let (toggle_text, toggle_bg, toggle_border, toggle_fg) = if r.enabled {
                (
                    if state.locale == SupportedLocale::TrTr {
                        "Aktif ✓"
                    } else {
                        "Active ✓"
                    },
                    COLOR_MINT_GREEN,
                    COLOR_DARK_GREEN,
                    COLOR_DARK_GREEN,
                )
            } else {
                (
                    if state.locale == SupportedLocale::TrTr {
                        "Kapalı ⏸"
                    } else {
                        "Off ⏸"
                    },
                    COLOR_BG_CONTENT,
                    COLOR_CARD_BORDER,
                    COLOR_TEXT_MUTED,
                )
            };
            draw_button(
                hdc,
                740,
                y + 10,
                825,
                y + 42,
                toggle_text,
                toggle_bg,
                toggle_border,
                toggle_fg,
                hfont_bold,
            );

            // Delete Button [❌ Sil]
            let del_text = if state.locale == SupportedLocale::TrTr {
                "Sil"
            } else {
                "Del"
            };
            draw_button(
                hdc,
                835,
                y + 10,
                895,
                y + 42,
                del_text,
                COLOR_DANGER_RED,
                COLOR_CARD_BORDER,
                COLOR_BG_CONTENT,
                hfont_bold,
            );
        }
    }

    // Add Custom Reminder Form (x: 248..915, y: 465..605)
    draw_card(hdc, 248, 465, 915, 605, COLOR_CARD_BG, COLOR_CARD_BORDER);

    SelectObject(hdc, hfont_bold);
    SetTextColor(hdc, COLOR_TEXT_COCOA);
    let add_title = if state.locale == SupportedLocale::TrTr {
        "Yeni Hatırlatıcı Ekle:"
    } else {
        "Add Custom Reminder:"
    };
    draw_str(hdc, 265, 476, add_title);

    SelectObject(hdc, hfont_regular);
    SetTextColor(hdc, COLOR_TEXT_MUTED);
    draw_str(
        hdc,
        265,
        508,
        if state.locale == SupportedLocale::TrTr {
            "Başlık:"
        } else {
            "Title:"
        },
    );
    draw_str(
        hdc,
        265,
        548,
        if state.locale == SupportedLocale::TrTr {
            "Mesaj:"
        } else {
            "Body:"
        },
    );

    // Add Button
    let add_btn_text = if state.locale == SupportedLocale::TrTr {
        "+ Hatırlatıcı Ekle ⏰"
    } else {
        "+ Add Reminder ⏰"
    };
    draw_button(
        hdc,
        705,
        504,
        898,
        574,
        add_btn_text,
        COLOR_MINT_GREEN,
        COLOR_CARD_BORDER,
        COLOR_TEXT_COCOA,
        hfont_bold,
    );
}

#[cfg(windows)]
fn get_mem_search_query(hwnd_edit: windows_sys::Win32::Foundation::HWND) -> String {
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetWindowTextLengthW, GetWindowTextW};
    unsafe {
        let len = GetWindowTextLengthW(hwnd_edit);
        if len > 0 {
            let mut buf = vec![0u16; len as usize + 1];
            GetWindowTextW(hwnd_edit, buf.as_mut_ptr(), len + 1);
            String::from_utf16_lossy(&buf[..len as usize])
                .trim()
                .to_lowercase()
        } else {
            String::new()
        }
    }
}

#[cfg(windows)]
unsafe fn render_tab_memories(
    hdc: windows_sys::Win32::Graphics::Gdi::HDC,
    state: &ControlCenterState,
    hfont_title: windows_sys::Win32::Graphics::Gdi::HFONT,
    hfont_bold: windows_sys::Win32::Graphics::Gdi::HFONT,
    hfont_regular: windows_sys::Win32::Graphics::Gdi::HFONT,
) {
    use windows_sys::Win32::Graphics::Gdi::{SelectObject, SetTextColor};

    SelectObject(hdc, hfont_title);
    SetTextColor(hdc, COLOR_TEXT_COCOA);
    let title = if state.locale == SupportedLocale::TrTr {
        "🧠  Kedi Hatıraları & Bellek"
    } else {
        "🧠  Companion Memories & Facts"
    };
    draw_str(hdc, 248, 20, title);

    SelectObject(hdc, hfont_regular);
    SetTextColor(hdc, COLOR_TEXT_MUTED);
    let sub = if state.locale == SupportedLocale::TrTr {
        "Mimi'nin senin hakkında öğrendiği kalıcı hatıralar (Tamamen yerel SQLite, şeffaf ve denetlenebilir)"
    } else {
        "Auditable facts learned about you (100% local SQLite, transparent & under your control)"
    };
    draw_str(hdc, 248, 46, sub);

    // Search bar header card (x: 248..915, y: 68..104)
    draw_card(hdc, 248, 68, 915, 104, COLOR_CARD_BG, COLOR_CARD_BORDER);

    SelectObject(hdc, hfont_bold);
    SetTextColor(hdc, COLOR_TEXT_COCOA);
    let search_lbl = if state.locale == SupportedLocale::TrTr {
        "🔍  Ara:"
    } else {
        "🔍  Search:"
    };
    draw_str(hdc, 265, 76, search_lbl);

    // Clear / All Buttons
    let clear_text = if state.locale == SupportedLocale::TrTr {
        "Temizle"
    } else {
        "Clear"
    };
    draw_button(
        hdc,
        740,
        72,
        820,
        100,
        clear_text,
        COLOR_BG_CONTENT,
        COLOR_CARD_BORDER,
        COLOR_TEXT_COCOA,
        hfont_regular,
    );

    let all_text = if state.locale == SupportedLocale::TrTr {
        "Tümü"
    } else {
        "All"
    };
    draw_button(
        hdc,
        828,
        72,
        895,
        100,
        all_text,
        COLOR_PEACH_LIGHT,
        COLOR_PEACH_ACCENT,
        COLOR_TEXT_COCOA,
        hfont_bold,
    );

    // Memories List Container (x: 248..915, y: 112..510)
    draw_card(hdc, 248, 112, 915, 510, COLOR_CARD_BG, COLOR_CARD_BORDER);

    let query = get_mem_search_query(state.hwnd_mem_search);
    let filtered_memories: Vec<&MemoryFact> = state
        .memories
        .iter()
        .filter(|m| {
            if query.is_empty() {
                true
            } else {
                m.subject.to_lowercase().contains(&query)
                    || m.predicate.to_lowercase().contains(&query)
                    || m.object.to_lowercase().contains(&query)
            }
        })
        .collect();

    if filtered_memories.is_empty() {
        SelectObject(hdc, hfont_regular);
        SetTextColor(hdc, COLOR_TEXT_MUTED);
        let empty_msg = if state.memories.is_empty() {
            if state.locale == SupportedLocale::TrTr {
                "Henüz kayıtlı hatıra yok. Sohbet ekranında 'Bunu hatırla: Kahvemi şekersiz içerim' diyerek Mimi'ye not aldırabilirsiniz."
            } else {
                "No memories stored yet. You can tell Mimi in chat: 'Remember that I drink coffee with no sugar'!"
            }
        } else if state.locale == SupportedLocale::TrTr {
            "Aramanızla eşleşen hatıra bulunamadı."
        } else {
            "No memories match your search filter."
        };
        draw_str(hdc, 265, 125, empty_msg);
    } else {
        let count = filtered_memories.len().min(5);
        for (idx, m) in filtered_memories.iter().take(count).enumerate() {
            let y = 122 + (idx as i32 * 68);

            draw_card(
                hdc,
                260,
                y,
                903,
                y + 60,
                COLOR_BG_CONTENT,
                COLOR_CARD_BORDER,
            );

            // Tags
            SelectObject(hdc, hfont_bold);
            SetTextColor(hdc, COLOR_DARK_GREEN);
            draw_str(hdc, 275, y + 6, &format!("[{}] {}", m.predicate, m.subject));

            SelectObject(hdc, hfont_regular);
            SetTextColor(hdc, COLOR_TEXT_COCOA);
            draw_str(hdc, 275, y + 32, &format!("\"{}\"", m.object));

            // Confidence Badge
            SelectObject(hdc, hfont_regular);
            SetTextColor(hdc, COLOR_TEXT_MUTED);
            let conf_str = format!("Güven: {:.0}%", m.confidence * 100.0);
            draw_str(hdc, 710, y + 16, &conf_str);

            // Delete Button [❌ Unut]
            let forget_btn = if state.locale == SupportedLocale::TrTr {
                "Unut"
            } else {
                "Forget"
            };
            draw_button(
                hdc,
                825,
                y + 12,
                895,
                y + 48,
                forget_btn,
                COLOR_DANGER_RED,
                COLOR_CARD_BORDER,
                COLOR_BG_CONTENT,
                hfont_bold,
            );
        }
    }

    // Local Privacy Safeguard Card (x: 248..915, y: 520..605)
    draw_card(hdc, 248, 520, 915, 605, COLOR_CARD_BG, COLOR_CARD_BORDER);
    SelectObject(hdc, hfont_bold);
    SetTextColor(hdc, COLOR_DARK_GREEN);
    draw_str(
        hdc,
        265,
        532,
        "🛡️  Yerel-Öncelikli Bellek & Tam Denetim Güvencesi",
    );

    SelectObject(hdc, hfont_regular);
    SetTextColor(hdc, COLOR_TEXT_MUTED);
    let guard_text = if state.locale == SupportedLocale::TrTr {
        "Hatıralar yalnızca bilgisayarınızdaki yerel veritabanında saklanır. Hiçbir zaman uzaktaki bir sunucuya aktarılmaz. İstediğiniz hatırayı silebilirsiniz."
    } else {
        "Memories are stored strictly on your local SQLite database and never uploaded to remote servers. You can forget any fact at any time."
    };
    draw_str(hdc, 265, 558, guard_text);
}

#[cfg(windows)]
unsafe fn render_tab_privacy(
    hdc: windows_sys::Win32::Graphics::Gdi::HDC,
    state: &ControlCenterState,
    hfont_title: windows_sys::Win32::Graphics::Gdi::HFONT,
    hfont_bold: windows_sys::Win32::Graphics::Gdi::HFONT,
    hfont_regular: windows_sys::Win32::Graphics::Gdi::HFONT,
) {
    use windows_sys::Win32::Graphics::Gdi::{SelectObject, SetTextColor};

    SelectObject(hdc, hfont_title);
    SetTextColor(hdc, COLOR_TEXT_COCOA);
    let title = if state.locale == SupportedLocale::TrTr {
        "🛡️  Gizlilik ve Güvenlik Güvencesi"
    } else {
        "🛡️  Privacy & Security Safeguards"
    };
    draw_str(hdc, 248, 20, title);

    SelectObject(hdc, hfont_regular);
    SetTextColor(hdc, COLOR_TEXT_MUTED);
    let sub = if state.locale == SupportedLocale::TrTr {
        "Sıfır-Telemetri, Yerel-Öncelikli (Local-First) ve Şeffaf Açık Kaynak Güvencesi"
    } else {
        "Zero-Telemetry, Local-First Architecture & Auditable Open Source"
    };
    draw_str(hdc, 248, 46, sub);

    // Safeguards Card (x: 248..915, y: 75..320)
    draw_card(hdc, 248, 75, 915, 320, COLOR_CARD_BG, COLOR_CARD_BORDER);

    SelectObject(hdc, hfont_bold);
    SetTextColor(hdc, COLOR_TEXT_COCOA);
    let card_header = if state.locale == SupportedLocale::TrTr {
        "🛡️  Kullanıcı Gizliliği ve Güvenlik Standartları"
    } else {
        "🛡️  User Privacy and Security Commitments"
    };
    draw_str(hdc, 265, 90, card_header);

    SelectObject(hdc, hfont_regular);
    SetTextColor(hdc, COLOR_TEXT_COCOA);
    let info_1 = if state.locale == SupportedLocale::TrTr {
        "• Yerel-Öncelikli Mimari: Tüm verileriniz (sohbet, hatırlatıcılar, hafıza) yalnızca yerel SQLite veritabanında saklanır."
    } else {
        "• Local-First Architecture: All data (chat, reminders, memories) resides strictly on your local SQLite database."
    };
    let info_2 = if state.locale == SupportedLocale::TrTr {
        "• Sıfır Telemetri: Uygulamada hiçbir izleme, telemetri veya harici analitik kütüphanesi bulunmaz."
    } else {
        "• Zero-Telemetry Guarantee: Absolutely zero tracking, analytics, or background call-homes exist in the codebase."
    };
    let info_3 = if state.locale == SupportedLocale::TrTr {
        "• Ekran Analizi Varsayılan Olarak KAPALIDIR: Kullanıcı açıkça izin vermedikçe hiçbir ekran yakalama veya OCR çalışmaz."
    } else {
        "• Screen Analysis Disabled by Default: No screen grabbing or OCR ever runs without explicit manual opt-in."
    };
    let info_4 = if state.locale == SupportedLocale::TrTr {
        "• Gizlilik Modu Koruması: Gizlilik Modu devredeyken tüm görsel algılama bileşenleri anında devreden çıkarılır."
    } else {
        "• Privacy Mode Guard: Enabling Privacy Mode immediately severs all visual inspection hooks."
    };
    draw_str(hdc, 265, 125, info_1);
    draw_str(hdc, 265, 160, info_2);
    draw_str(hdc, 265, 195, info_3);
    draw_str(hdc, 265, 230, info_4);

    // Privacy Mode Toggle Button at (248..680, 340..405)
    let (btn_bg, btn_border, toggle_text) = if state.privacy_mode {
        (
            COLOR_MINT_GREEN,
            COLOR_DARK_GREEN,
            if state.locale == SupportedLocale::TrTr {
                "✓ GİZLİLİK MODU AKTİF (KORUMALI) 🛡️"
            } else {
                "✓ PRIVACY MODE ACTIVE (GUARDED) 🛡️"
            },
        )
    } else {
        (
            COLOR_PEACH_LIGHT,
            COLOR_PEACH_ACCENT,
            if state.locale == SupportedLocale::TrTr {
                "🛡️ Gizlilik Modunu Açmak İçin Tıklayın"
            } else {
                "🛡️ Click to Enable Privacy Mode"
            },
        )
    };

    draw_button(
        hdc,
        248,
        340,
        680,
        405,
        toggle_text,
        btn_bg,
        btn_border,
        COLOR_TEXT_COCOA,
        hfont_bold,
    );

    // Architecture Info Card (x: 248..915, y: 425..605)
    draw_card(hdc, 248, 425, 915, 605, COLOR_CARD_BG, COLOR_CARD_BORDER);

    SelectObject(hdc, hfont_bold);
    SetTextColor(hdc, COLOR_TEXT_COCOA);
    draw_str(hdc, 265, 440, "🔒  Açık Kaynak Mühendislik Standartları");

    SelectObject(hdc, hfont_regular);
    SetTextColor(hdc, COLOR_TEXT_MUTED);
    let arch_1 = if state.locale == SupportedLocale::TrTr {
        "• Süreç İzolasyonu: IPC Named Pipe (openpet-host-pipe) üzerinden en az yetki prensibiyle iletişim kurulur."
    } else {
        "• Process Isolation: Communication occurs via strictly scoped local named pipes with least privilege."
    };
    let arch_2 = if state.locale == SupportedLocale::TrTr {
        "• Veri Denetlenebilirliği: Bellekteki veya diskteki tüm veriler SQLite üzerinden doğrudan okunabilir ve silinebilir."
    } else {
        "• Transparent Auditability: Every fact, reminder, and setting is stored in plain SQLite files you own."
    };
    let arch_3 = if state.locale == SupportedLocale::TrTr {
        "• Lisans & Bağımsızlık: AGPL-3.0 lisanslıdır, kaynak kodları şeffaf bir biçimde GitHub üzerinde mevcuttur."
    } else {
        "• AGPL-3.0 Open-Source: Fully inspectable and verifiable by the community, maintained by Tiyatrotist."
    };
    draw_str(hdc, 265, 475, arch_1);
    draw_str(hdc, 265, 510, arch_2);
    draw_str(hdc, 265, 545, arch_3);
}

#[cfg(windows)]
unsafe fn render_tab_settings(
    hdc: windows_sys::Win32::Graphics::Gdi::HDC,
    state: &ControlCenterState,
    hfont_title: windows_sys::Win32::Graphics::Gdi::HFONT,
    hfont_bold: windows_sys::Win32::Graphics::Gdi::HFONT,
    hfont_regular: windows_sys::Win32::Graphics::Gdi::HFONT,
) {
    use windows_sys::Win32::Graphics::Gdi::{SelectObject, SetTextColor};

    SelectObject(hdc, hfont_title);
    SetTextColor(hdc, COLOR_TEXT_COCOA);
    let title = if state.locale == SupportedLocale::TrTr {
        "⚙️  Uygulama Ayarları"
    } else {
        "⚙️  Application Settings"
    };
    draw_str(hdc, 248, 20, title);

    SelectObject(hdc, hfont_regular);
    SetTextColor(hdc, COLOR_TEXT_MUTED);
    let sub = if state.locale == SupportedLocale::TrTr {
        "Arayüz dili, kedi görünümü ve masaüstü tercihleri"
    } else {
        "Interface language, companion appearance, and desktop preferences"
    };
    draw_str(hdc, 248, 46, sub);

    // Section 1: Dil Seçimi / Interface Language
    SelectObject(hdc, hfont_bold);
    SetTextColor(hdc, COLOR_TEXT_COCOA);
    let lang_header = if state.locale == SupportedLocale::TrTr {
        "Arayüz Dili / Interface Language:"
    } else {
        "Interface Language:"
    };
    draw_str(hdc, 248, 70, lang_header);

    // Language Buttons (y: 92..132)
    let tr_active = state.locale == SupportedLocale::TrTr;
    let bg_tr = if tr_active {
        COLOR_PEACH_ACCENT
    } else {
        COLOR_CARD_BG
    };
    let border_tr = if tr_active {
        COLOR_TEXT_COCOA
    } else {
        COLOR_CARD_BORDER
    };
    draw_button(
        hdc,
        248,
        92,
        410,
        132,
        "🇹🇷  Türkçe",
        bg_tr,
        border_tr,
        COLOR_TEXT_COCOA,
        hfont_bold,
    );

    let en_active = state.locale == SupportedLocale::EnUs;
    let bg_en = if en_active {
        COLOR_PEACH_ACCENT
    } else {
        COLOR_CARD_BG
    };
    let border_en = if en_active {
        COLOR_TEXT_COCOA
    } else {
        COLOR_CARD_BORDER
    };
    draw_button(
        hdc,
        425,
        92,
        585,
        132,
        "🇬🇧  English",
        bg_en,
        border_en,
        COLOR_TEXT_COCOA,
        hfont_bold,
    );

    // Section 2: Görsel Sanat Stili / Companion Art Style
    SelectObject(hdc, hfont_bold);
    SetTextColor(hdc, COLOR_TEXT_COCOA);
    let style_header = if state.locale == SupportedLocale::TrTr {
        "Görsel Sanat Stili / Art Style:"
    } else {
        "Companion Art Style:"
    };
    draw_str(hdc, 248, 147, style_header);

    let is_pixel = state.settings.art_style == CompanionArtStyle::PixelArt;
    let bg_pixel = if is_pixel {
        COLOR_PEACH_ACCENT
    } else {
        COLOR_CARD_BG
    };
    let border_pixel = if is_pixel {
        COLOR_TEXT_COCOA
    } else {
        COLOR_CARD_BORDER
    };
    draw_button(
        hdc,
        248,
        170,
        470,
        210,
        if state.locale == SupportedLocale::TrTr {
            "🎨  Piksel Sanatı (Mimi)"
        } else {
            "🎨  Pixel Art (Mimi)"
        },
        bg_pixel,
        border_pixel,
        COLOR_TEXT_COCOA,
        hfont_bold,
    );

    let bg_real = if !is_pixel {
        COLOR_PEACH_ACCENT
    } else {
        COLOR_CARD_BG
    };
    let border_real = if !is_pixel {
        COLOR_TEXT_COCOA
    } else {
        COLOR_CARD_BORDER
    };
    draw_button(
        hdc,
        485,
        170,
        710,
        210,
        if state.locale == SupportedLocale::TrTr {
            "📸  Gerçek Kedi (Oreo Smokin)"
        } else {
            "📸  Real Cat (Oreo Tuxedo)"
        },
        bg_real,
        border_real,
        COLOR_TEXT_COCOA,
        hfont_bold,
    );

    // Section 3: Piksel Kedi Cinsi / Companion Breed
    SelectObject(hdc, hfont_bold);
    SetTextColor(hdc, COLOR_TEXT_COCOA);
    let breed_header = if state.locale == SupportedLocale::TrTr {
        "Piksel Kedi Deseni / Coat Pattern (Mimi):"
    } else {
        "Pixel Coat Pattern (Mimi):"
    };
    draw_str(hdc, 248, 225, breed_header);

    let breeds = [
        (248, 328, CatBreed::Tabby, "Tekir"),
        (336, 421, CatBreed::Tuxedo, "Smokin"),
        (429, 504, CatBreed::Calico, "Calico"),
        (512, 597, CatBreed::Ginger, "Sarıman"),
        (605, 685, CatBreed::Siamese, "Siyam"),
        (693, 768, CatBreed::Black, "Siyah"),
        (776, 856, CatBreed::White, "Beyaz"),
    ];

    for (x0, x1, brd, name) in breeds {
        let is_selected = state.settings.cat_breed == brd;
        let bg = if is_selected {
            COLOR_PEACH_ACCENT
        } else {
            COLOR_CARD_BG
        };
        let border = if is_selected {
            COLOR_TEXT_COCOA
        } else {
            COLOR_CARD_BORDER
        };
        draw_button(
            hdc,
            x0,
            248,
            x1,
            285,
            name,
            bg,
            border,
            COLOR_TEXT_COCOA,
            hfont_bold,
        );
    }

    // Section 4: Masaüstü & Performans Tercihleri
    draw_card(hdc, 248, 305, 915, 510, COLOR_CARD_BG, COLOR_CARD_BORDER);

    SelectObject(hdc, hfont_bold);
    SetTextColor(hdc, COLOR_TEXT_COCOA);
    let title_pref = if state.locale == SupportedLocale::TrTr {
        "🖥️  Masaüstü Tercihleri & Güvenlik"
    } else {
        "🖥️  Desktop Preferences & Security"
    };
    draw_str(hdc, 265, 318, title_pref);

    // 1. Always On Top
    SelectObject(hdc, hfont_bold);
    SetTextColor(hdc, COLOR_TEXT_COCOA);
    let aot_label = if state.locale == SupportedLocale::TrTr {
        "Her Zaman Üstte (Always On Top):"
    } else {
        "Always On Top:"
    };
    draw_str(hdc, 265, 345, aot_label);

    SelectObject(hdc, hfont_regular);
    SetTextColor(hdc, COLOR_TEXT_MUTED);
    let aot_sub = if state.locale == SupportedLocale::TrTr {
        "Pet penceresi diğer pencerelerin üzerinde görünür"
    } else {
        "Pet floats above active desktop windows"
    };
    draw_str(hdc, 265, 365, aot_sub);

    let (aot_btn_text, aot_bg, aot_border, aot_fg) = if state.settings.always_on_top {
        (
            if state.locale == SupportedLocale::TrTr {
                "Açık (Üstte) ✓"
            } else {
                "ON (Top) ✓"
            },
            COLOR_MINT_GREEN,
            COLOR_DARK_GREEN,
            COLOR_DARK_GREEN,
        )
    } else {
        (
            if state.locale == SupportedLocale::TrTr {
                "Kapalı ✕"
            } else {
                "OFF ✕"
            },
            COLOR_BG_CONTENT,
            COLOR_CARD_BORDER,
            COLOR_TEXT_MUTED,
        )
    };
    draw_button(
        hdc,
        730,
        342,
        895,
        378,
        aot_btn_text,
        aot_bg,
        aot_border,
        aot_fg,
        hfont_bold,
    );

    // 2. Privacy Mode
    SelectObject(hdc, hfont_bold);
    SetTextColor(hdc, COLOR_TEXT_COCOA);
    let priv_label = if state.locale == SupportedLocale::TrTr {
        "Gizlilik Modu (Privacy Mode):"
    } else {
        "Privacy Mode:"
    };
    draw_str(hdc, 265, 395, priv_label);

    SelectObject(hdc, hfont_regular);
    SetTextColor(hdc, COLOR_TEXT_MUTED);
    let priv_sub = if state.locale == SupportedLocale::TrTr {
        "Görsel algılama ve ekran kancalarını anında durdurur"
    } else {
        "Halts screen capture and inspection hooks"
    };
    draw_str(hdc, 265, 415, priv_sub);

    let (priv_btn_text, priv_bg, priv_border, priv_fg) = if state.privacy_mode {
        (
            if state.locale == SupportedLocale::TrTr {
                "Aktif 🛡️"
            } else {
                "Active 🛡️"
            },
            COLOR_MINT_GREEN,
            COLOR_DARK_GREEN,
            COLOR_DARK_GREEN,
        )
    } else {
        (
            if state.locale == SupportedLocale::TrTr {
                "Kapalı ✕"
            } else {
                "Disabled ✕"
            },
            COLOR_BG_CONTENT,
            COLOR_CARD_BORDER,
            COLOR_TEXT_MUTED,
        )
    };
    draw_button(
        hdc,
        730,
        392,
        895,
        428,
        priv_btn_text,
        priv_bg,
        priv_border,
        priv_fg,
        hfont_bold,
    );

    // Info lines
    SelectObject(hdc, hfont_regular);
    SetTextColor(hdc, COLOR_TEXT_MUTED);
    let anim_info = if state.locale == SupportedLocale::TrTr {
        "• Görsel Motoru: Fotogerçekçi Katmanlı & Piksel Sanatı, 60 FPS Dinamik Render."
    } else {
        "• Visual Engine: Photorealistic Layered & Pixel Art, 60 FPS Dynamic Rendering."
    };
    let author_info = if state.locale == SupportedLocale::TrTr {
        "• Lisans ve Geliştirici: Tiyatrotist • AGPL-3.0 Açık Kaynak."
    } else {
        "• Author & License: Tiyatrotist • AGPL-3.0 Open-Source."
    };
    draw_str(hdc, 265, 448, anim_info);
    draw_str(hdc, 265, 472, author_info);

    // Save Feedback Text
    SelectObject(hdc, hfont_regular);
    SetTextColor(hdc, COLOR_DARK_GREEN);
    let save_note = if state.locale == SupportedLocale::TrTr {
        "✓ Değişiklikler anında yerel SQLite veritabanına kaydedilir."
    } else {
        "✓ Settings are automatically persisted to local SQLite storage."
    };
    draw_str(hdc, 248, 525, save_note);
}

// ---------------------------------------------------------------------------
// Helper Methods & Win32 Drawing
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
unsafe fn draw_card(
    hdc: windows_sys::Win32::Graphics::Gdi::HDC,
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
    bg_color: u32,
    border_color: u32,
) {
    use windows_sys::Win32::Graphics::Gdi::{
        CreatePen, CreateSolidBrush, DeleteObject, RoundRect, SelectObject, PS_SOLID,
    };
    let pen = CreatePen(PS_SOLID, 1, border_color);
    let brush = CreateSolidBrush(bg_color);
    let old_p = SelectObject(hdc, pen);
    let old_b = SelectObject(hdc, brush);

    RoundRect(hdc, left, top, right, bottom, 12, 12);

    SelectObject(hdc, old_p);
    SelectObject(hdc, old_b);
    DeleteObject(pen);
    DeleteObject(brush);
}

#[cfg(windows)]
unsafe fn draw_button(
    hdc: windows_sys::Win32::Graphics::Gdi::HDC,
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
    text: &str,
    bg_color: u32,
    border_color: u32,
    text_color: u32,
    hfont: windows_sys::Win32::Graphics::Gdi::HFONT,
) {
    use windows_sys::Win32::Foundation::RECT;
    use windows_sys::Win32::Graphics::Gdi::{
        DrawTextW, SelectObject, SetTextColor, DT_CENTER, DT_NOPREFIX, DT_SINGLELINE, DT_VCENTER,
    };

    draw_card(hdc, left, top, right, bottom, bg_color, border_color);

    let old_f = SelectObject(hdc, hfont);
    SetTextColor(hdc, text_color);
    let wide: Vec<u16> = OsStr::new(text)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let mut rect = RECT {
        left,
        top,
        right,
        bottom,
    };
    DrawTextW(
        hdc,
        wide.as_ptr(),
        -1,
        &mut rect,
        DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
    );
    SelectObject(hdc, old_f);
}

#[cfg(windows)]
unsafe fn draw_progress_bar(
    hdc: windows_sys::Win32::Graphics::Gdi::HDC,
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
    ratio: f32,
    fill_color: u32,
    label: &str,
    pct_str: &str,
    hfont: windows_sys::Win32::Graphics::Gdi::HFONT,
) {
    use windows_sys::Win32::Graphics::Gdi::{
        CreatePen, CreateSolidBrush, DeleteObject, RoundRect, SelectObject, SetTextColor, PS_SOLID,
    };

    // Label on left
    let old_f = SelectObject(hdc, hfont);
    SetTextColor(hdc, COLOR_TEXT_COCOA);
    draw_str(hdc, left - 110, top - 2, label);

    // Track
    let pen = CreatePen(PS_SOLID, 1, COLOR_CARD_BORDER);
    let track_brush = CreateSolidBrush(COLOR_PROGRESS_TRACK);
    let old_p = SelectObject(hdc, pen);
    let old_b = SelectObject(hdc, track_brush);
    RoundRect(hdc, left, top, right, bottom, 8, 8);

    // Fill
    let fill_w = ((right - left) as f32 * ratio.clamp(0.0, 1.0)) as i32;
    if fill_w > 4 {
        let fill_brush = CreateSolidBrush(fill_color);
        SelectObject(hdc, fill_brush);
        RoundRect(hdc, left, top, left + fill_w, bottom, 8, 8);
        DeleteObject(fill_brush);
    }

    SelectObject(hdc, old_p);
    SelectObject(hdc, old_b);
    DeleteObject(pen);
    DeleteObject(track_brush);

    // Percentage on right
    draw_str(hdc, right + 12, top - 2, pct_str);
    SelectObject(hdc, old_f);
}

fn get_pet_mood_and_frame(state: &ControlCenterState) -> (&'static str, &'static str) {
    let is_tr = state.locale == SupportedLocale::TrTr;
    let tick = state.frame_counter;
    if let Some(ref st) = state.pet_state {
        if st.sleepiness > 0.85 {
            (
                if is_tr {
                    "💤 Uyuyor (Sleeping)"
                } else {
                    "💤 Sleeping"
                },
                if tick % 2 == 0 { "sleep_0" } else { "sleep_1" },
            )
        } else if st.mood > 0.85 {
            (
                if is_tr {
                    "🐱 Mırıldanıyor (Purring)"
                } else {
                    "🐱 Purring"
                },
                if tick % 2 == 0 { "purr_0" } else { "purr_1" },
            )
        } else if st.boredom > 0.80 {
            (
                if is_tr {
                    "⚡ Koşturuyor (Zoomies)"
                } else {
                    "⚡ Zoomies!"
                },
                if tick % 2 == 0 {
                    "zoomies_0"
                } else {
                    "zoomies_1"
                },
            )
        } else if st.energy < 0.25 {
            (
                if is_tr {
                    "🍞 Ekmek Pozu (Loaf)"
                } else {
                    "🍞 Loaf"
                },
                "loaf_0",
            )
        } else if st.bond > 0.80 {
            (
                if is_tr {
                    "🐾 Hamur Yoğuruyor (Kneading)"
                } else {
                    "🐾 Kneading"
                },
                if tick % 2 == 0 { "knead_0" } else { "knead_1" },
            )
        } else if st.curiosity > 0.70 {
            (
                if is_tr {
                    "👀 Meraklı (Curious)"
                } else {
                    "👀 Curious"
                },
                if tick % 2 == 0 {
                    "curious_0"
                } else {
                    "curious_1"
                },
            )
        } else {
            (
                if is_tr {
                    "✨ Neşeli (Happy)"
                } else {
                    "✨ Happy & Alert"
                },
                if tick % 2 == 0 { "idle_0" } else { "idle_1" },
            )
        }
    } else {
        (
            if is_tr {
                "✨ Dinleniyor (Idle)"
            } else {
                "✨ Resting (Idle)"
            },
            if tick % 2 == 0 { "idle_0" } else { "idle_1" },
        )
    }
}

#[cfg(windows)]
unsafe fn draw_cat_sprite(
    hdc: windows_sys::Win32::Graphics::Gdi::HDC,
    x: i32,
    y: i32,
    state: &ControlCenterState,
) {
    use windows_sys::Win32::Graphics::Gdi::{
        SetDIBitsToDevice, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, RGBQUAD,
    };

    let (_, frame_name) = get_pet_mood_and_frame(state);

    // Render 128x128 sprite with cream background #FFF8F0 (BGR 0x00F0F8FF -> RGB 0x00FFF8F0)
    let (pixels, out_w, out_h) = if state.settings.art_style == CompanionArtStyle::Realistic {
        state
            .realistic_sheet
            .render_frame_bgra(frame_name, 128, 128, Some(0x00FFF8F0))
    } else {
        state
            .sprite_sheet
            .render_frame_bgra_scaled(frame_name, 2, Some(0x00FFF8F0))
    };

    let bmi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: out_w as i32,
            biHeight: -(out_h as i32), // Top-down DIB
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB,
            biSizeImage: 0,
            biXPelsPerMeter: 0,
            biYPelsPerMeter: 0,
            biClrUsed: 0,
            biClrImportant: 0,
        },
        bmiColors: [RGBQUAD {
            rgbBlue: 0,
            rgbGreen: 0,
            rgbRed: 0,
            rgbReserved: 0,
        }],
    };

    SetDIBitsToDevice(
        hdc,
        x,
        y,
        out_w as u32,
        out_h as u32,
        0,
        0,
        0,
        out_h as u32,
        pixels.as_ptr() as *const _,
        &bmi,
        DIB_RGB_COLORS,
    );
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

        let is_mem = state.active_tab == TAB_MEMORIES;
        ShowWindow(
            state.hwnd_mem_search,
            if is_mem { SW_SHOW } else { SW_HIDE },
        );
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

            // Fetch Memories
            if let Ok(IpcResponse::Memories(mems)) = client.request(&IpcRequest::ListMemories).await
            {
                state.memories = mems;
            }

            // Fetch Settings
            if let Ok(IpcResponse::Settings(st)) = client.request(&IpcRequest::GetSettings).await {
                if state.locale != st.locale {
                    state.locale = st.locale;
                    state.i18n.set_locale(st.locale);
                }
                if state.settings.cat_breed != st.cat_breed {
                    state.sprite_sheet = MimiSpriteSheet::generate_for_breed(st.cat_breed);
                }
                state.settings = st.clone();
                state.privacy_mode = st.privacy_mode;
            }
        } else {
            state.is_connected = false;
        }
    });
}

fn send_chat_message_text(state: &mut ControlCenterState, text: String) {
    let trimmed = text.trim().to_string();
    if trimmed.is_empty() {
        return;
    }

    state.chat_history.push(("User".into(), trimmed.clone()));

    let req = IpcRequest::SendChatMessage {
        conversation_id: Uuid::new_v4(),
        content: trimmed,
    };
    let pipe = default_pipe_name();
    let handle = state.runtime_handle.clone();
    let locale = state.locale;

    handle.block_on(async {
        if let Ok(mut client) = IpcClient::connect(&pipe).await {
            if let Ok(IpcResponse::ChatMessage(reply)) = client.request(&req).await {
                state.chat_history.push(("Mimi".into(), reply.content));
                return;
            }
        }
        // Fallback response if offline
        let fallback = if locale == SupportedLocale::TrTr {
            "*mırıldanarak patisini sana uzatır* (Masaüstü Pet Bağlantısı)"
        } else {
            "*purrs softly and reaches out a paw* (Desktop Pet Companion)"
        };
        state.chat_history.push(("Mimi".into(), fallback.into()));
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

                // Clear input
                let empty_wide: Vec<u16> = OsStr::new("")
                    .encode_wide()
                    .chain(std::iter::once(0))
                    .collect();
                SetWindowTextW(state.hwnd_chat_input, empty_wide.as_ptr());

                send_chat_message_text(state, text);
            }
        }
    }
}

fn schedule_quick_reminder(state: &mut ControlCenterState, title: &str, body: &str, minutes: i64) {
    let req = IpcRequest::CreateReminder {
        title: title.to_string(),
        body: body.to_string(),
        schedule_utc: chrono::Utc::now() + chrono::Duration::minutes(minutes),
        recurrence: openpet_types::RecurrenceRule::Once,
    };

    let pipe = default_pipe_name();
    let handle = state.runtime_handle.clone();
    handle.block_on(async {
        if let Ok(mut client) = IpcClient::connect(&pipe).await {
            let _ = client.request(&req).await;
        }
    });

    poll_host_updates(state);
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
                    title,
                    body,
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
        let bg_brush = CreateSolidBrush(COLOR_BG_CONTENT);

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

        let win_title: Vec<u16> = OsStr::new("OpenPet Control Center 🐾")
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let hwnd = CreateWindowExW(
            0,
            class_name.as_ptr(),
            win_title.as_ptr(),
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            120,
            80,
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

        // 1. Chat input edit control at (248, 525, 560, 38)
        let hwnd_chat_input = CreateWindowExW(
            0,
            edit_class.as_ptr(),
            std::ptr::null(),
            WS_CHILD | WS_BORDER | (ES_AUTOHSCROLL as u32),
            248,
            525,
            560,
            38,
            hwnd,
            ID_EDIT_CHAT as isize as _,
            hinstance,
            std::ptr::null_mut(),
        );

        // 2. Reminder title input at (345, 504, 340, 30)
        let hwnd_rem_title = CreateWindowExW(
            0,
            edit_class.as_ptr(),
            std::ptr::null(),
            WS_CHILD | WS_BORDER | (ES_AUTOHSCROLL as u32),
            345,
            504,
            340,
            30,
            hwnd,
            ID_EDIT_REM_TITLE as isize as _,
            hinstance,
            std::ptr::null_mut(),
        );

        // 3. Reminder body input at (345, 544, 340, 30)
        let hwnd_rem_body = CreateWindowExW(
            0,
            edit_class.as_ptr(),
            std::ptr::null(),
            WS_CHILD | WS_BORDER | (ES_AUTOHSCROLL as u32),
            345,
            544,
            340,
            30,
            hwnd,
            ID_EDIT_REM_BODY as isize as _,
            hinstance,
            std::ptr::null_mut(),
        );

        // Set font on edit controls
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

        // 4. Memory search input at (345, 73, 380, 28)
        let hwnd_mem_search = CreateWindowExW(
            0,
            edit_class.as_ptr(),
            std::ptr::null(),
            WS_CHILD | WS_BORDER | (ES_AUTOHSCROLL as u32),
            345,
            73,
            380,
            28,
            hwnd,
            ID_EDIT_MEM_SEARCH as isize as _,
            hinstance,
            std::ptr::null_mut(),
        );
        windows_sys::Win32::UI::WindowsAndMessaging::SendMessageW(
            hwnd_mem_search,
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
            ShowWindow(hwnd_mem_search, SW_HIDE);
        } else if cur_tab == TAB_REMINDERS {
            ShowWindow(hwnd_chat_input, SW_HIDE);
            ShowWindow(hwnd_rem_title, SW_SHOW);
            ShowWindow(hwnd_rem_body, SW_SHOW);
            ShowWindow(hwnd_mem_search, SW_HIDE);
        } else if cur_tab == TAB_MEMORIES {
            ShowWindow(hwnd_chat_input, SW_HIDE);
            ShowWindow(hwnd_rem_title, SW_HIDE);
            ShowWindow(hwnd_rem_body, SW_HIDE);
            ShowWindow(hwnd_mem_search, SW_SHOW);
        } else {
            ShowWindow(hwnd_chat_input, SW_HIDE);
            ShowWindow(hwnd_rem_title, SW_HIDE);
            ShowWindow(hwnd_rem_body, SW_HIDE);
            ShowWindow(hwnd_mem_search, SW_HIDE);
        }

        let i18n = I18nManager::new(initial_locale);
        let default_breed = CatBreed::Tabby;
        let sprite_sheet = MimiSpriteSheet::generate_for_breed(default_breed);
        let realistic_sheet = RealisticCompanionSheet::new();

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
            memories: Vec::new(),
            settings: AppSettings::default(),
            status_feedback: String::new(),
            runtime_handle: handle,
            shutdown_flag: Arc::new(AtomicBool::new(false)),
            sprite_sheet,
            realistic_sheet,
            frame_counter: 0,
            hwnd_main: hwnd,
            hwnd_chat_input,
            hwnd_rem_title,
            hwnd_rem_body,
            hwnd_mem_search,
        }));

        SetWindowLongPtrW(hwnd, GWLP_USERDATA, Box::into_raw(state) as isize);

        // Poll host on launch
        let state_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Mutex<ControlCenterState>;
        if !state_ptr.is_null() {
            let mut state_guard = (*state_ptr).lock().unwrap();
            poll_host_updates(&mut state_guard);
        }

        // Set animation & poll timer (fires every 500ms)
        SetTimer(hwnd, 1, 500, None);

        info!("OpenPet Control Center GUI window displayed (Cozy & Pastel theme).");

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
