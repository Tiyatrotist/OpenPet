//! # Windows Desktop Pet Window
//!
//! Native Win32 borderless, transparent, draggable desktop pet window.
//! Handles real-time animations, mouse interactions (petting, dragging, context menu),
//! and coordinates with the behavior engine.

use crate::chat_window::FloatingChatWindow;
use crate::tray::{SystemTray, TrayAction, TrayState, WM_TRAY_CALLBACK};
use openpet_render::mimi::MimiSpriteSheet;
use openpet_render::realistic::RealisticCompanionSheet;
use openpet_types::{BehaviorType, CompanionArtStyle, InteractionType, SupportedLocale};
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use tracing::{error, info};

pub const PET_WINDOW_WIDTH: i32 = 160;
pub const PET_WINDOW_HEIGHT: i32 = 170;
pub const PET_CHROMA_KEY: u32 = 0x00FF00FF; // Magenta chroma key

// Desktop Pet Direct Context Menu Command IDs
pub const CMD_PET_DASHBOARD: usize = 3000;
pub const CMD_PET_CHAT: usize = 3001;
pub const CMD_PET_FEED: usize = 3002;
pub const CMD_PET_PLAY: usize = 3003;
pub const CMD_PET_WATER: usize = 3004;
pub const CMD_PET_GROOM: usize = 3005;
pub const CMD_PET_HIGHFIVE: usize = 3006;
pub const CMD_PET_SLEEP: usize = 3007;
pub const CMD_PET_HIDE: usize = 3008;
pub const CMD_PET_SETTINGS: usize = 3009;
pub const CMD_PET_EXIT: usize = 3010;
pub const CMD_PET_TOGGLE_STYLE: usize = 3011;

/// Commands that can be dispatched to the active pet window from host threads.
#[derive(Debug)]
pub enum PetWindowCommand {
    SetBehavior(BehaviorType),
    SetTargetPosition { x: i32, y: i32 },
    SetVisibility(bool),
    SetPrivacyMode(bool),
    SetPaused(bool),
    SetLocale(SupportedLocale),
    ShowChat(bool),
    AddChatMessage { sender: String, text: String },
    TriggerAction(InteractionType),
    PlayAnimation { name: String, duration_ticks: u32 },
    SetBreed(openpet_types::CatBreed),
    SetArtStyle(CompanionArtStyle),
    SetAlwaysOnTop(bool),
    Close,
}

/// Resolves which realistic companion frame to render based on pet behavior, current animation, and chat state.
pub fn resolve_realistic_frame(
    behavior: BehaviorType,
    current_frame_name: &str,
    chat_open: bool,
) -> &'static str {
    if chat_open {
        return "ask_0";
    }

    if current_frame_name.starts_with("sleep") || current_frame_name.contains("loaf") {
        return "sleep_0";
    }
    if current_frame_name.starts_with("stretch") {
        return "stretch_0";
    }
    if current_frame_name.starts_with("curious") {
        return "curious_0";
    }
    if current_frame_name.starts_with("ask") || current_frame_name.starts_with("chat") {
        return "ask_0";
    }
    if current_frame_name.starts_with("high_five") {
        return "high_five_0";
    }

    match behavior {
        BehaviorType::Sleep | BehaviorType::Loaf => "sleep_0",
        BehaviorType::Stretch => "stretch_0",
        BehaviorType::Curious => "curious_0",
        BehaviorType::HighFive => "high_five_0",
        BehaviorType::Sit | BehaviorType::Happy | BehaviorType::Idle => "sit_0",
        _ => "sit_0",
    }
}

/// Internal state owned by the pet window Win32 message procedure.
pub struct PetWindowState {
    pub sprite_sheet: MimiSpriteSheet,
    pub realistic_sheet: RealisticCompanionSheet,
    pub art_style: CompanionArtStyle,
    pub current_frame_name: String,
    pub current_behavior: BehaviorType,
    pub frame_counter: u32,
    pub is_dragging: bool,
    pub drag_start_cursor: (i32, i32),
    pub drag_start_win: (i32, i32),
    pub has_moved_during_drag: bool,
    pub pet_pos: (i32, i32),
    pub target_pos: Option<(i32, i32)>,
    pub pet_visible: bool,
    pub pet_paused: bool,
    pub privacy_mode: bool,
    pub locale: SupportedLocale,
    pub pet_mood_timer: u32,
    pub speech_bubble_text: String,
    pub speech_bubble_timer: u32,
    pub tray: Option<SystemTray>,
    pub chat_window: Option<FloatingChatWindow>,
    pub interaction_tx: Option<mpsc::Sender<InteractionType>>,
    pub tray_action_tx: Option<mpsc::Sender<TrayAction>>,
    pub shutdown_flag: Arc<AtomicBool>,
}

#[cfg(windows)]
unsafe extern "system" fn pet_window_wndproc(
    hwnd: windows_sys::Win32::Foundation::HWND,
    msg: u32,
    wparam: windows_sys::Win32::Foundation::WPARAM,
    lparam: windows_sys::Win32::Foundation::LPARAM,
) -> windows_sys::Win32::Foundation::LRESULT {
    use windows_sys::Win32::Foundation::{POINT, RECT};
    use windows_sys::Win32::Graphics::Gdi::{
        BeginPaint, BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, CreateSolidBrush, DeleteDC,
        DeleteObject, EndPaint, FillRect, InvalidateRect, SelectObject, SetDIBitsToDevice,
        BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, PAINTSTRUCT, SRCCOPY,
    };
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{ReleaseCapture, SetCapture};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        DefWindowProcW, DestroyWindow, GetCursorPos, GetWindowLongPtrW, GetWindowRect,
        PostQuitMessage, SetWindowPos, GWLP_USERDATA, SWP_NOSIZE, SWP_NOZORDER, WM_CLOSE,
        WM_CONTEXTMENU, WM_CREATE, WM_DESTROY, WM_LBUTTONDBLCLK, WM_LBUTTONDOWN, WM_LBUTTONUP,
        WM_MOUSEMOVE, WM_NCHITTEST, WM_PAINT, WM_RBUTTONUP, WM_TIMER,
    };

    let state_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut PetWindowState;

    match msg {
        WM_CREATE => 0,
        WM_PAINT => {
            if !state_ptr.is_null() {
                let state = &mut *state_ptr;
                let mut ps: PAINTSTRUCT = std::mem::zeroed();
                let hdc = BeginPaint(hwnd, &mut ps);

                let mem_dc = CreateCompatibleDC(hdc);
                let hbitmap = CreateCompatibleBitmap(hdc, PET_WINDOW_WIDTH, PET_WINDOW_HEIGHT);
                let old_bmp = SelectObject(mem_dc, hbitmap);

                // Clear background to chroma key Magenta
                let magenta_brush = CreateSolidBrush(PET_CHROMA_KEY);
                let rect = RECT {
                    left: 0,
                    top: 0,
                    right: PET_WINDOW_WIDTH,
                    bottom: PET_WINDOW_HEIGHT,
                };
                FillRect(mem_dc, &rect, magenta_brush);
                DeleteObject(magenta_brush);

                // Only render sprite frame when pet is visible
                if state.pet_visible {
                    let (bgra, w, h) = match state.art_style {
                        CompanionArtStyle::PixelArt => state.sprite_sheet.render_frame_bgra_scaled(
                            &state.current_frame_name,
                            2,
                            Some(PET_CHROMA_KEY),
                        ),
                        CompanionArtStyle::Realistic => {
                            let is_chat_open = state
                                .chat_window
                                .as_ref()
                                .map(|c| c.is_visible())
                                .unwrap_or(false);
                            let frame = resolve_realistic_frame(
                                state.current_behavior,
                                &state.current_frame_name,
                                is_chat_open,
                            );
                            state.realistic_sheet.render_frame_bgra(
                                frame,
                                128,
                                128,
                                Some(PET_CHROMA_KEY),
                            )
                        }
                    };

                    let mut bmi: BITMAPINFO = std::mem::zeroed();
                    bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
                    bmi.bmiHeader.biWidth = w as i32;
                    bmi.bmiHeader.biHeight = -(h as i32); // Top-down
                    bmi.bmiHeader.biPlanes = 1;
                    bmi.bmiHeader.biBitCount = 32;
                    bmi.bmiHeader.biCompression = BI_RGB;

                    SetDIBitsToDevice(
                        mem_dc,
                        16,
                        42,
                        w as u32,
                        h as u32,
                        0,
                        0,
                        0,
                        h as u32,
                        bgra.as_ptr() as *const _,
                        &bmi,
                        DIB_RGB_COLORS,
                    );
                }

                // Render lightweight cute mini speech/thought bubble above the pet
                if state.pet_visible
                    && state.speech_bubble_timer > 0
                    && !state.speech_bubble_text.is_empty()
                {
                    use windows_sys::Win32::Graphics::Gdi::{
                        CreateFontW, CreatePen, DrawTextW, LineTo, MoveToEx, RoundRect, SetBkMode,
                        SetTextColor, DT_CENTER, DT_NOPREFIX, DT_SINGLELINE, DT_VCENTER, FW_BOLD,
                        PS_SOLID, TRANSPARENT,
                    };

                    // Cream rounded speech bubble with warm peach border
                    let bubble_brush = CreateSolidBrush(0x00F0F8FF); // #FFF8F0
                    let bubble_pen = CreatePen(PS_SOLID, 1, 0x0076ABFF); // #FFAB76
                    let old_p = SelectObject(mem_dc, bubble_pen);
                    let old_b = SelectObject(mem_dc, bubble_brush);

                    let bubble_rect = RECT {
                        left: 8,
                        top: 6,
                        right: PET_WINDOW_WIDTH - 8,
                        bottom: 38,
                    };
                    RoundRect(
                        mem_dc,
                        bubble_rect.left,
                        bubble_rect.top,
                        bubble_rect.right,
                        bubble_rect.bottom,
                        14,
                        14,
                    );

                    // Speech bubble tail pointing towards cat's head
                    let pt_x = PET_WINDOW_WIDTH / 2;
                    MoveToEx(mem_dc, pt_x - 5, 37, std::ptr::null_mut());
                    LineTo(mem_dc, pt_x, 42);
                    LineTo(mem_dc, pt_x + 5, 37);

                    SelectObject(mem_dc, old_b);
                    SelectObject(mem_dc, old_p);
                    DeleteObject(bubble_brush);
                    DeleteObject(bubble_pen);

                    // Draw speech text in warm cocoa brown
                    let font_name: Vec<u16> = OsStr::new("Segoe UI")
                        .encode_wide()
                        .chain(std::iter::once(0))
                        .collect();
                    let font = CreateFontW(
                        14,
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
                    let old_f = SelectObject(mem_dc, font);
                    SetBkMode(mem_dc, TRANSPARENT as i32);
                    SetTextColor(mem_dc, 0x003D3E4A); // Cocoa brown #4A3E3D

                    let wide_text: Vec<u16> = OsStr::new(&state.speech_bubble_text)
                        .encode_wide()
                        .chain(std::iter::once(0))
                        .collect();
                    let mut text_rect = RECT {
                        left: 10,
                        top: 8,
                        right: PET_WINDOW_WIDTH - 10,
                        bottom: 36,
                    };
                    DrawTextW(
                        mem_dc,
                        wide_text.as_ptr(),
                        -1,
                        &mut text_rect,
                        DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
                    );

                    SelectObject(mem_dc, old_f);
                    DeleteObject(font);
                }

                BitBlt(
                    hdc,
                    0,
                    0,
                    PET_WINDOW_WIDTH,
                    PET_WINDOW_HEIGHT,
                    mem_dc,
                    0,
                    0,
                    SRCCOPY,
                );

                SelectObject(mem_dc, old_bmp);
                DeleteObject(hbitmap);
                DeleteDC(mem_dc);
                EndPaint(hwnd, &ps);
            }
            0
        }
        WM_LBUTTONDOWN => {
            if !state_ptr.is_null() {
                let state = &mut *state_ptr;
                SetCapture(hwnd);
                state.is_dragging = true;
                state.has_moved_during_drag = false;

                let mut pt: POINT = std::mem::zeroed();
                GetCursorPos(&mut pt);
                state.drag_start_cursor = (pt.x, pt.y);

                let mut win_rect: RECT = std::mem::zeroed();
                GetWindowRect(hwnd, &mut win_rect);
                state.drag_start_win = (win_rect.left, win_rect.top);

                state.current_frame_name = "drag_0".to_string();
                InvalidateRect(hwnd, std::ptr::null(), 0);

                if let Some(ref tx) = state.interaction_tx {
                    let _ = tx.send(InteractionType::DragStart {
                        x: pt.x as f32,
                        y: pt.y as f32,
                    });
                }
            }
            0
        }
        WM_MOUSEMOVE => {
            if !state_ptr.is_null() {
                let state = &mut *state_ptr;
                if state.is_dragging {
                    let mut pt: POINT = std::mem::zeroed();
                    GetCursorPos(&mut pt);
                    let dx = pt.x - state.drag_start_cursor.0;
                    let dy = pt.y - state.drag_start_cursor.1;

                    if dx.abs() > 4 || dy.abs() > 4 {
                        state.has_moved_during_drag = true;
                        let new_x = state.drag_start_win.0 + dx;
                        let new_y = state.drag_start_win.1 + dy;
                        state.pet_pos = (new_x, new_y);
                        SetWindowPos(
                            hwnd,
                            std::ptr::null_mut(),
                            new_x,
                            new_y,
                            0,
                            0,
                            SWP_NOSIZE | SWP_NOZORDER,
                        );

                        if let Some(ref tx) = state.interaction_tx {
                            let _ = tx.send(InteractionType::DragMove {
                                x: new_x as f32,
                                y: new_y as f32,
                            });
                        }
                    }
                }
            }
            0
        }
        WM_LBUTTONUP => {
            if !state_ptr.is_null() {
                let state = &mut *state_ptr;
                if state.is_dragging {
                    ReleaseCapture();
                    state.is_dragging = false;

                    let mut pt: POINT = std::mem::zeroed();
                    GetCursorPos(&mut pt);

                    if !state.has_moved_during_drag {
                        // User gently tapped/petted the cat!
                        info!("Pet interaction: User clicked/petted Mimi");
                        state.current_frame_name = "purr_0".to_string();
                        state.pet_mood_timer = 20; // Show happy pet reaction for 20 ticks (~1.5s)
                        state.speech_bubble_text = if state.locale == SupportedLocale::TrTr {
                            "❤️ *mırrr mırrr*".into()
                        } else {
                            "❤️ *purr purr*".into()
                        };
                        state.speech_bubble_timer = 30;
                        InvalidateRect(hwnd, std::ptr::null(), 0);

                        if let Some(ref tx) = state.interaction_tx {
                            let _ = tx.send(InteractionType::Petting { intensity: 1.0 });
                        }
                    } else {
                        // Drag completed, drop pet
                        info!("Pet dropped at coordinates: {:?}", state.pet_pos);
                        state.current_frame_name = "sit_0".to_string();
                        InvalidateRect(hwnd, std::ptr::null(), 0);

                        if let Some(ref tx) = state.interaction_tx {
                            let _ = tx.send(InteractionType::DragEnd {
                                x: state.pet_pos.0 as f32,
                                y: state.pet_pos.1 as f32,
                            });
                        }
                    }
                }
            }
            0
        }
        WM_LBUTTONDBLCLK => {
            if !state_ptr.is_null() {
                let state = &mut *state_ptr;
                info!("Pet double-click: Launching Control Center dashboard");
                launch_control_center(&[]);
                if let Some(ref tx) = state.interaction_tx {
                    let _ = tx.send(InteractionType::DoubleClick);
                }
            }
            0
        }
        WM_RBUTTONUP => {
            // Right clicking on the pet displays direct desktop action menu!
            if !state_ptr.is_null() {
                let state = &mut *state_ptr;
                show_pet_context_menu(hwnd, state);
            }
            0
        }
        WM_TRAY_CALLBACK => {
            // Callback from taskbar system tray icon
            let event = lparam as u32;
            if !state_ptr.is_null() {
                let state = &mut *state_ptr;
                if event == WM_LBUTTONDBLCLK {
                    // Double click tray icon -> Open Control Center
                    handle_tray_action(hwnd, state, TrayAction::OpenControlCenter);
                } else if event == WM_RBUTTONUP || event == WM_CONTEXTMENU {
                    // Right click tray icon -> Show Context Menu
                    if let Some(ref tray) = state.tray {
                        if let Some(action) = tray.show_context_menu(hwnd) {
                            handle_tray_action(hwnd, state, action);
                        }
                    }
                }
            }
            0
        }
        WM_TIMER => {
            if !state_ptr.is_null() {
                let state = &mut *state_ptr;
                state.frame_counter = state.frame_counter.wrapping_add(1);

                // Speech bubble countdown
                if state.speech_bubble_timer > 0 {
                    state.speech_bubble_timer -= 1;
                    if state.speech_bubble_timer == 0 {
                        state.speech_bubble_text.clear();
                        InvalidateRect(hwnd, std::ptr::null(), 0);
                    }
                }

                // Temporary petting/action emotion countdown
                if state.pet_mood_timer > 0 {
                    state.pet_mood_timer -= 1;
                    if state.current_frame_name.starts_with("eat") {
                        let step = (state.frame_counter / 3) % 2;
                        let next = format!("eat_{}", step);
                        if state.current_frame_name != next {
                            state.current_frame_name = next;
                            InvalidateRect(hwnd, std::ptr::null(), 0);
                        }
                    } else if state.current_frame_name.starts_with("jump") {
                        let step = (state.frame_counter / 3) % 2;
                        let next = format!("jump_{}", step);
                        if state.current_frame_name != next {
                            state.current_frame_name = next;
                            InvalidateRect(hwnd, std::ptr::null(), 0);
                        }
                    } else if state.current_frame_name.starts_with("groom") {
                        let step = (state.frame_counter / 3) % 2;
                        let next = format!("groom_{}", step);
                        if state.current_frame_name != next {
                            state.current_frame_name = next;
                            InvalidateRect(hwnd, std::ptr::null(), 0);
                        }
                    } else if state.current_frame_name.starts_with("drink") {
                        let step = (state.frame_counter / 3) % 2;
                        let next = format!("drink_{}", step);
                        if state.current_frame_name != next {
                            state.current_frame_name = next;
                            InvalidateRect(hwnd, std::ptr::null(), 0);
                        }
                    } else if state.current_frame_name.starts_with("purr") {
                        let step = (state.frame_counter / 3) % 2;
                        let next = format!("purr_{}", step);
                        if state.current_frame_name != next {
                            state.current_frame_name = next;
                            InvalidateRect(hwnd, std::ptr::null(), 0);
                        }
                    }
                    if state.pet_mood_timer == 0 {
                        state.current_frame_name = match state.current_behavior {
                            BehaviorType::Sleep => "sleep_0".to_string(),
                            BehaviorType::Sit => "sit_0".to_string(),
                            BehaviorType::Purr => "purr_0".to_string(),
                            BehaviorType::Loaf => "loaf_0".to_string(),
                            _ => "idle_0".to_string(),
                        };
                        InvalidateRect(hwnd, std::ptr::null(), 0);
                    }
                } else if !state.is_dragging && !state.pet_paused {
                    // Smooth locomotion interpolation if target position is set
                    if let Some(target) = state.target_pos {
                        let dx = target.0 - state.pet_pos.0;
                        let dy = target.1 - state.pet_pos.1;
                        let dist_sq = dx * dx + dy * dy;

                        if dist_sq > 16 {
                            let speed = match state.current_behavior {
                                BehaviorType::Zoomies => 10,
                                BehaviorType::Run => 7,
                                _ => 4,
                            };
                            let step_x = (dx.clamp(-speed, speed)) as i32;
                            let step_y = (dy.clamp(-(speed / 2).max(2), (speed / 2).max(2))) as i32;
                            state.pet_pos.0 += step_x;
                            state.pet_pos.1 += step_y;
                            SetWindowPos(
                                hwnd,
                                std::ptr::null_mut(),
                                state.pet_pos.0,
                                state.pet_pos.1,
                                0,
                                0,
                                SWP_NOSIZE | SWP_NOZORDER,
                            );
                            if state.current_behavior != BehaviorType::Zoomies
                                && state.current_behavior != BehaviorType::Run
                            {
                                state.current_behavior = BehaviorType::Walk;
                            }
                        } else {
                            state.target_pos = None;
                            state.current_behavior = BehaviorType::Idle;
                        }
                    }

                    // Cycle animation frames based on active behavior
                    let next_frame = match state.current_behavior {
                        BehaviorType::Walk => {
                            let step = (state.frame_counter / 2) % 3;
                            match step {
                                0 => "walk_0",
                                1 => "walk_1",
                                _ => "walk_2",
                            }
                        }
                        BehaviorType::Sit => {
                            // Occasionally groom while sitting
                            if (state.frame_counter / 16) % 4 == 2 {
                                if ((state.frame_counter / 4) & 1) == 0 {
                                    "groom_0"
                                } else {
                                    "groom_1"
                                }
                            } else {
                                "sit_0"
                            }
                        }
                        BehaviorType::Sleep => {
                            if ((state.frame_counter / 8) & 1) == 0 {
                                "sleep_0"
                            } else {
                                "sleep_1"
                            }
                        }
                        BehaviorType::Play => {
                            if ((state.frame_counter / 2) & 1) == 0 {
                                "play_0"
                            } else {
                                "play_1"
                            }
                        }
                        BehaviorType::Stretch => {
                            if ((state.frame_counter / 4) & 1) == 0 {
                                "stretch_0"
                            } else {
                                "stretch_1"
                            }
                        }
                        BehaviorType::Curious => {
                            if ((state.frame_counter / 4) & 1) == 0 {
                                "curious_0"
                            } else {
                                "curious_1"
                            }
                        }
                        BehaviorType::Happy | BehaviorType::HighFive => {
                            if ((state.frame_counter / 3) & 1) == 0 {
                                "jump_0"
                            } else {
                                "jump_1"
                            }
                        }
                        BehaviorType::Purr => {
                            if ((state.frame_counter / 3) & 1) == 0 {
                                "purr_0"
                            } else {
                                "purr_1"
                            }
                        }
                        BehaviorType::Knead => {
                            if ((state.frame_counter / 3) & 1) == 0 {
                                "knead_0"
                            } else {
                                "knead_1"
                            }
                        }
                        BehaviorType::Zoomies => {
                            if ((state.frame_counter / 2) & 1) == 0 {
                                "zoomies_0"
                            } else {
                                "zoomies_1"
                            }
                        }
                        BehaviorType::Loaf => "loaf_0",
                        BehaviorType::Surprised => "surprised_0",
                        _ => {
                            if ((state.frame_counter / 4) & 1) == 0 {
                                "idle_0"
                            } else {
                                "idle_1"
                            }
                        }
                    };

                    if state.current_frame_name != next_frame {
                        state.current_frame_name = next_frame.to_string();
                        InvalidateRect(hwnd, std::ptr::null(), 0);
                    }
                }
            }
            0
        }
        WM_NCHITTEST => {
            if !state_ptr.is_null() {
                let state = &*state_ptr;
                if !state.pet_visible {
                    return windows_sys::Win32::UI::WindowsAndMessaging::HTTRANSPARENT as isize;
                }
                if state.is_dragging {
                    return windows_sys::Win32::UI::WindowsAndMessaging::HTCLIENT as isize;
                }
                let mut win_rect: RECT = std::mem::zeroed();
                GetWindowRect(hwnd, &mut win_rect);
                let cursor_x = (lparam & 0xFFFF) as i16 as i32;
                let cursor_y = ((lparam >> 16) & 0xFFFF) as i16 as i32;
                let rel_x = cursor_x - win_rect.left;
                let rel_y = cursor_y - win_rect.top;

                // Speech bubble clickable hit area
                if state.speech_bubble_timer > 0
                    && !state.speech_bubble_text.is_empty()
                    && (6..=PET_WINDOW_WIDTH - 6).contains(&rel_x)
                    && (4..=42).contains(&rel_y)
                {
                    return windows_sys::Win32::UI::WindowsAndMessaging::HTCLIENT as isize;
                }

                // Cat sprite relative area (offset 16, 42)
                let sprite_rel_x = rel_x - 16;
                let sprite_rel_y = rel_y - 42;
                if (0..128).contains(&sprite_rel_x) && (0..128).contains(&sprite_rel_y) {
                    let is_opaque = match state.art_style {
                        CompanionArtStyle::PixelArt => {
                            let sprite_x = (sprite_rel_x / 2).clamp(0, 63) as usize;
                            let sprite_y = (sprite_rel_y / 2).clamp(0, 63) as usize;
                            let frame = state.sprite_sheet.get_frame(&state.current_frame_name);
                            let px = frame.get_pixel(sprite_x, sprite_y);
                            px[3] > 0
                        }
                        CompanionArtStyle::Realistic => {
                            let is_chat_open = state
                                .chat_window
                                .as_ref()
                                .map(|c| c.is_visible())
                                .unwrap_or(false);
                            let frame = resolve_realistic_frame(
                                state.current_behavior,
                                &state.current_frame_name,
                                is_chat_open,
                            );
                            state.realistic_sheet.is_pixel_opaque(
                                frame,
                                128,
                                128,
                                sprite_rel_x as usize,
                                sprite_rel_y as usize,
                            )
                        }
                    };
                    if is_opaque {
                        return windows_sys::Win32::UI::WindowsAndMessaging::HTCLIENT as isize;
                    }
                }
                return windows_sys::Win32::UI::WindowsAndMessaging::HTTRANSPARENT as isize;
            }
            windows_sys::Win32::UI::WindowsAndMessaging::HTCLIENT as isize
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

/// Displays the desktop pet direct action menu when right clicked.
#[cfg(windows)]
unsafe fn show_pet_context_menu(
    hwnd: windows_sys::Win32::Foundation::HWND,
    state: &mut PetWindowState,
) {
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::Graphics::Gdi::InvalidateRect;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        AppendMenuW, CreatePopupMenu, DestroyMenu, GetCursorPos, PostMessageW, PostQuitMessage,
        SetForegroundWindow, TrackPopupMenu, HMENU, MF_SEPARATOR, MF_STRING, TPM_NONOTIFY,
        TPM_RETURNCMD, TPM_RIGHTBUTTON, WM_NULL,
    };

    let hmenu = CreatePopupMenu();
    if hmenu.is_null() {
        return;
    }

    let append_item = |menu: HMENU, id: usize, text: &str| {
        let wide: Vec<u16> = OsStr::new(text)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        AppendMenuW(menu, MF_STRING, id, wide.as_ptr());
    };

    let is_tr = state.locale == SupportedLocale::TrTr;

    // 1. 🐾 Kontrol Merkezi (Dashboard)
    append_item(
        hmenu,
        CMD_PET_DASHBOARD,
        if is_tr {
            "🐾 Kontrol Merkezi (Dashboard)"
        } else {
            "🐾 Control Center (Dashboard)"
        },
    );

    AppendMenuW(hmenu, MF_SEPARATOR, 0, std::ptr::null());

    // 2. 💬 Sohbet Et (Chat)
    append_item(
        hmenu,
        CMD_PET_CHAT,
        if is_tr {
            "💬 Sohbet Et"
        } else {
            "💬 Chat with Mimi"
        },
    );

    // 3. 🐟 Besle (Mama Ver)
    append_item(
        hmenu,
        CMD_PET_FEED,
        if is_tr {
            "🐟 Besle (Mama Ver)"
        } else {
            "🐟 Feed Treat"
        },
    );

    // 4. 💧 Su Ver (Water)
    append_item(
        hmenu,
        CMD_PET_WATER,
        if is_tr {
            "💧 Su Ver"
        } else {
            "💧 Give Water"
        },
    );

    // 5. 🪮 Tara (Groom)
    append_item(
        hmenu,
        CMD_PET_GROOM,
        if is_tr {
            "🪮 Tara (Tüy Bakımı)"
        } else {
            "🪮 Groom Fur"
        },
    );

    // 6. 🧶 Oyna (Play)
    append_item(
        hmenu,
        CMD_PET_PLAY,
        if is_tr {
            "🧶 Oyna (Yumak)"
        } else {
            "🧶 Play with Yarn"
        },
    );

    // 7. 🖐️ Çak Bir Beşlik! (High Five)
    append_item(
        hmenu,
        CMD_PET_HIGHFIVE,
        if is_tr {
            "🖐️ Çak Bir Beşlik!"
        } else {
            "🖐️ High Five!"
        },
    );

    // 8. 💤 Uyut / Uyandır (Sleep / Wake)
    let is_sleeping = state.current_behavior == BehaviorType::Sleep;
    let sleep_label = if is_sleeping {
        if is_tr {
            "⏰ Mimi'yi Uyandır"
        } else {
            "⏰ Wake Mimi Up"
        }
    } else if is_tr {
        "💤 Mimi'yi Uyut"
    } else {
        "💤 Put Mimi to Sleep"
    };
    append_item(hmenu, CMD_PET_SLEEP, sleep_label);

    AppendMenuW(hmenu, MF_SEPARATOR, 0, std::ptr::null());

    // 9. 👁️ Peti Gizle (Hide)
    append_item(
        hmenu,
        CMD_PET_HIDE,
        if is_tr {
            "👁️ Peti Gizle"
        } else {
            "👁️ Hide Pet"
        },
    );

    // 10. 🎨 Görünüm: Piksel / Gerçekçi
    append_item(
        hmenu,
        CMD_PET_TOGGLE_STYLE,
        if is_tr {
            "🎨 Görünüm: Piksel / Gerçekçi"
        } else {
            "🎨 Appearance: Pixel / Realistic"
        },
    );

    // 11. ⚙️ Ayarlar (Settings)
    append_item(
        hmenu,
        CMD_PET_SETTINGS,
        if is_tr {
            "⚙️ Ayarlar"
        } else {
            "⚙️ Settings"
        },
    );

    AppendMenuW(hmenu, MF_SEPARATOR, 0, std::ptr::null());

    // 12. ❌ Çıkış (Exit)
    append_item(
        hmenu,
        CMD_PET_EXIT,
        if is_tr { "❌ Çıkış" } else { "❌ Exit" },
    );

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
    PostMessageW(hwnd, WM_NULL, 0, 0);

    match selected {
        CMD_PET_DASHBOARD => {
            info!("Pet context menu: Opening Control Center dashboard");
            launch_control_center(&[]);
        }
        CMD_PET_CHAT => {
            info!("Pet context menu: Opening chat dashboard");
            launch_control_center(&["--tab", "chat"]);
        }
        CMD_PET_FEED => {
            info!("Pet context menu: Feeding Mimi");
            state.current_frame_name = "eat_0".to_string();
            state.pet_mood_timer = 25;
            state.speech_bubble_text = if is_tr {
                "🐟 Ham ham! Çok lezzetli!".into()
            } else {
                "🐟 Yum! Delicious treat!".into()
            };
            state.speech_bubble_timer = 35;
            InvalidateRect(hwnd, std::ptr::null(), 0);
            if let Some(ref tx) = state.interaction_tx {
                let _ = tx.send(InteractionType::Feed);
            }
        }
        CMD_PET_WATER => {
            info!("Pet context menu: Giving water to Mimi");
            state.current_frame_name = "drink_0".to_string();
            state.pet_mood_timer = 25;
            state.speech_bubble_text = if is_tr {
                "💧 Gluk gluk! Teşekkürler!".into()
            } else {
                "💧 Refreshing water! Thanks!".into()
            };
            state.speech_bubble_timer = 35;
            InvalidateRect(hwnd, std::ptr::null(), 0);
            if let Some(ref tx) = state.interaction_tx {
                let _ = tx.send(InteractionType::Water);
            }
        }
        CMD_PET_GROOM => {
            info!("Pet context menu: Grooming Mimi");
            state.current_frame_name = "groom_0".to_string();
            state.pet_mood_timer = 25;
            state.speech_bubble_text = if is_tr {
                "🪮 Mırrr... Pırıl pırıl oldum!".into()
            } else {
                "🪮 Purr... Groomed & shiny!".into()
            };
            state.speech_bubble_timer = 35;
            InvalidateRect(hwnd, std::ptr::null(), 0);
            if let Some(ref tx) = state.interaction_tx {
                let _ = tx.send(InteractionType::Groom);
            }
        }
        CMD_PET_PLAY => {
            info!("Pet context menu: Playing with Mimi");
            state.current_frame_name = "jump_0".to_string();
            state.pet_mood_timer = 20;
            state.speech_bubble_text = if is_tr {
                "🧶 Yaşasın! Oyun zamanı!".into()
            } else {
                "🧶 Yay! Playtime!".into()
            };
            state.speech_bubble_timer = 35;
            InvalidateRect(hwnd, std::ptr::null(), 0);
            if let Some(ref tx) = state.interaction_tx {
                let _ = tx.send(InteractionType::Play);
            }
        }
        CMD_PET_HIGHFIVE => {
            info!("Pet context menu: High five with Mimi!");
            state.current_frame_name = "jump_0".to_string();
            state.pet_mood_timer = 20;
            state.speech_bubble_text = if is_tr {
                "🖐️ Çak bir beşlik!".into()
            } else {
                "🖐️ High five!".into()
            };
            state.speech_bubble_timer = 35;
            InvalidateRect(hwnd, std::ptr::null(), 0);
            if let Some(ref tx) = state.interaction_tx {
                let _ = tx.send(InteractionType::HighFive);
            }
        }
        CMD_PET_SLEEP => {
            if state.current_behavior == BehaviorType::Sleep {
                info!("Pet context menu: Waking up Mimi");
                state.current_behavior = BehaviorType::Idle;
                state.current_frame_name = "idle_0".to_string();
                state.speech_bubble_text = if is_tr {
                    "☀️ Günaydın dostum!".into()
                } else {
                    "☀️ Good morning friend!".into()
                };
                state.speech_bubble_timer = 30;
            } else {
                info!("Pet context menu: Putting Mimi to sleep");
                state.current_behavior = BehaviorType::Sleep;
                state.current_frame_name = "sleep_0".to_string();
                state.speech_bubble_text = if is_tr {
                    "💤 Zzz... İyi uykular...".into()
                } else {
                    "💤 Zzz... Sweet dreams...".into()
                };
                state.speech_bubble_timer = 30;
            }
            InvalidateRect(hwnd, std::ptr::null(), 0);
            if let Some(ref tx) = state.interaction_tx {
                let _ = tx.send(InteractionType::SleepToggle);
            }
        }
        CMD_PET_HIDE => {
            info!("Pet context menu: Hiding pet to tray");
            state.pet_visible = false;
            if let Some(ref chat) = state.chat_window {
                chat.set_visible(false);
            }
            InvalidateRect(hwnd, std::ptr::null(), 0);
            if let Some(ref mut tray) = state.tray {
                tray.update_state(TrayState {
                    privacy_mode: state.privacy_mode,
                    pet_paused: state.pet_paused,
                    pet_visible: false,
                    art_style: state.art_style,
                    locale: state.locale,
                });
            }
        }
        CMD_PET_TOGGLE_STYLE => {
            state.art_style = match state.art_style {
                CompanionArtStyle::PixelArt => CompanionArtStyle::Realistic,
                CompanionArtStyle::Realistic => CompanionArtStyle::PixelArt,
            };
            info!(
                "Pet context menu: Toggled art style to {:?}",
                state.art_style
            );
            if let Some(ref mut tray) = state.tray {
                tray.update_state(TrayState {
                    privacy_mode: state.privacy_mode,
                    pet_paused: state.pet_paused,
                    pet_visible: state.pet_visible,
                    art_style: state.art_style,
                    locale: state.locale,
                });
            }
            if let Some(ref tx) = state.tray_action_tx {
                let _ = tx.send(TrayAction::ToggleArtStyle);
            }
            InvalidateRect(hwnd, std::ptr::null(), 0);
        }
        CMD_PET_SETTINGS => {
            info!("Pet context menu: Opening settings");
            launch_control_center(&["--settings"]);
        }
        CMD_PET_EXIT => {
            info!("Pet context menu: Exiting application");
            state.shutdown_flag.store(true, Ordering::SeqCst);
            PostQuitMessage(0);
        }
        _ => {}
    }
}

/// Dispatches selected tray/context actions (Open Control, Toggle Visibility, Privacy, Pause, Settings, Exit).
#[cfg(windows)]
fn handle_tray_action(
    hwnd: windows_sys::Win32::Foundation::HWND,
    state: &mut PetWindowState,
    action: TrayAction,
) {
    use windows_sys::Win32::Graphics::Gdi::InvalidateRect;

    info!("Executing tray action: {:?}", action);

    // Forward action to host receiver if registered
    if let Some(ref tx) = state.tray_action_tx {
        let _ = tx.send(action);
    }

    match action {
        TrayAction::OpenControlCenter => {
            // Launch or focus Control Center
            launch_control_center(&[]);
        }
        TrayAction::OpenSettings => {
            // Launch Control Center directly on the settings tab
            launch_control_center(&["--settings"]);
        }
        TrayAction::TogglePetVisibility => {
            state.pet_visible = !state.pet_visible;
            info!("Pet visibility toggled to: {}", state.pet_visible);
            unsafe {
                InvalidateRect(hwnd, std::ptr::null(), 0);
            }
            if let Some(ref mut tray) = state.tray {
                tray.update_state(TrayState {
                    privacy_mode: state.privacy_mode,
                    pet_paused: state.pet_paused,
                    pet_visible: state.pet_visible,
                    art_style: state.art_style,
                    locale: state.locale,
                });
            }
        }
        TrayAction::TogglePrivacyMode => {
            state.privacy_mode = !state.privacy_mode;
            info!("Privacy mode set to: {}", state.privacy_mode);
            if let Some(ref mut tray) = state.tray {
                tray.update_state(TrayState {
                    privacy_mode: state.privacy_mode,
                    pet_paused: state.pet_paused,
                    pet_visible: state.pet_visible,
                    art_style: state.art_style,
                    locale: state.locale,
                });
            }
        }
        TrayAction::TogglePausePet => {
            state.pet_paused = !state.pet_paused;
            info!("Pet pause state set to: {}", state.pet_paused);
            if let Some(ref mut tray) = state.tray {
                tray.update_state(TrayState {
                    privacy_mode: state.privacy_mode,
                    pet_paused: state.pet_paused,
                    pet_visible: state.pet_visible,
                    art_style: state.art_style,
                    locale: state.locale,
                });
            }
        }
        TrayAction::ToggleArtStyle => {
            state.art_style = match state.art_style {
                CompanionArtStyle::PixelArt => CompanionArtStyle::Realistic,
                CompanionArtStyle::Realistic => CompanionArtStyle::PixelArt,
            };
            info!("Art style toggled to: {:?}", state.art_style);
            unsafe {
                InvalidateRect(hwnd, std::ptr::null(), 0);
            }
            if let Some(ref mut tray) = state.tray {
                tray.update_state(TrayState {
                    privacy_mode: state.privacy_mode,
                    pet_paused: state.pet_paused,
                    pet_visible: state.pet_visible,
                    art_style: state.art_style,
                    locale: state.locale,
                });
            }
        }
        TrayAction::ExitApplication => {
            info!("Exit requested from tray menu. Terminating pet window and host.");
            state.shutdown_flag.store(true, Ordering::SeqCst);
            unsafe {
                windows_sys::Win32::UI::WindowsAndMessaging::PostQuitMessage(0);
            }
        }
    }
}

/// Spawns the OpenPet Control Center process asynchronously.
pub fn launch_control_center(args: &[&str]) {
    use std::process::Command;

    // Check adjacent binary in current exe's dir or standard target paths
    let current_exe = std::env::current_exe().unwrap_or_default();
    let parent_dir = current_exe.parent().unwrap_or(std::path::Path::new(""));

    let candidates = [
        parent_dir.join("openpet-control.exe"),
        parent_dir.join("openpet-control"),
        std::path::PathBuf::from("target/debug/openpet-control.exe"),
        std::path::PathBuf::from("target/release/openpet-control.exe"),
    ];

    for candidate in &candidates {
        if candidate.exists() {
            info!("Launching Control Center from: {:?}", candidate);
            let mut cmd = Command::new(candidate);
            cmd.args(args);
            if let Err(e) = cmd.spawn() {
                error!("Failed spawning Control Center: {}", e);
            }
            return;
        }
    }

    // Fallback: cargo run
    info!("Control Center binary not found on disk; invoking via cargo run");
    let mut cmd = Command::new("cargo");
    cmd.args(["run", "-p", "openpet-control", "--"]);
    cmd.args(args);
    let _ = cmd.spawn();
}

/// Spawns the Windows UI thread hosting the transparent pet window and system tray.
#[allow(clippy::too_many_arguments)]
pub fn spawn_desktop_pet_window(
    initial_pos: (i32, i32),
    initial_locale: SupportedLocale,
    initial_privacy: bool,
    initial_breed: openpet_types::CatBreed,
    initial_art_style: CompanionArtStyle,
    initial_always_on_top: bool,
    interaction_tx: mpsc::Sender<InteractionType>,
    tray_action_tx: mpsc::Sender<TrayAction>,
    chat_tx: mpsc::Sender<String>,
    cmd_rx: mpsc::Receiver<PetWindowCommand>,
    shutdown_flag: Arc<AtomicBool>,
) -> std::thread::JoinHandle<()> {
    std::thread::Builder::new()
        .name("openpet-ui-thread".to_string())
        .spawn(move || {
            #[cfg(windows)]
            run_pet_window_loop(
                initial_pos,
                initial_locale,
                initial_privacy,
                initial_breed,
                initial_art_style,
                initial_always_on_top,
                interaction_tx,
                tray_action_tx,
                chat_tx,
                cmd_rx,
                shutdown_flag,
            );

            #[cfg(not(windows))]
            {
                let _ = (
                    initial_pos,
                    initial_locale,
                    initial_privacy,
                    initial_breed,
                    initial_art_style,
                    initial_always_on_top,
                    interaction_tx,
                    tray_action_tx,
                    chat_tx,
                    cmd_rx,
                    shutdown_flag,
                );
            }
        })
        .expect("Failed spawning OpenPet Windows UI thread")
}

#[cfg(windows)]
#[allow(clippy::too_many_arguments)]
fn run_pet_window_loop(
    initial_pos: (i32, i32),
    initial_locale: SupportedLocale,
    initial_privacy: bool,
    initial_breed: openpet_types::CatBreed,
    initial_art_style: CompanionArtStyle,
    initial_always_on_top: bool,
    interaction_tx: mpsc::Sender<InteractionType>,
    tray_action_tx: mpsc::Sender<TrayAction>,
    chat_tx: mpsc::Sender<String>,
    cmd_rx: mpsc::Receiver<PetWindowCommand>,
    shutdown_flag: Arc<AtomicBool>,
) {
    use windows_sys::Win32::Graphics::Gdi::{CreateSolidBrush, DeleteObject, InvalidateRect};
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DispatchMessageW, GetSystemMetrics, LoadCursorW, LoadIconW, PeekMessageW,
        PostQuitMessage, RegisterClassExW, SetLayeredWindowAttributes, SetWindowLongPtrW,
        TranslateMessage, CS_DBLCLKS, CS_HREDRAW, CS_VREDRAW, GWLP_USERDATA, IDC_ARROW,
        IDI_APPLICATION, LWA_COLORKEY, MSG, PM_REMOVE, SM_CXSCREEN, SM_CYSCREEN, WNDCLASSEXW,
        WS_EX_LAYERED, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP, WS_VISIBLE,
    };

    unsafe {
        let class_name: Vec<u16> = OsStr::new("OpenPet_DesktopPetWindowClass")
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let hinstance = GetModuleHandleW(std::ptr::null());
        let brush = CreateSolidBrush(PET_CHROMA_KEY);

        let wnd_class = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW | CS_DBLCLKS,
            lpfnWndProc: Some(pet_window_wndproc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinstance,
            hIcon: LoadIconW(std::ptr::null_mut(), IDI_APPLICATION),
            hCursor: LoadCursorW(std::ptr::null_mut(), IDC_ARROW),
            hbrBackground: brush,
            lpszMenuName: std::ptr::null(),
            lpszClassName: class_name.as_ptr(),
            hIconSm: std::ptr::null_mut(),
        };

        RegisterClassExW(&wnd_class);

        let window_title: Vec<u16> = OsStr::new("OpenPet Desktop Companion")
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        // Calculate bottom-right positioning if not explicitly provided (respecting taskbar work area)
        let mut work_area: windows_sys::Win32::Foundation::RECT = std::mem::zeroed();
        let has_work_area = windows_sys::Win32::UI::WindowsAndMessaging::SystemParametersInfoW(
            windows_sys::Win32::UI::WindowsAndMessaging::SPI_GETWORKAREA,
            0,
            &mut work_area as *mut _ as *mut std::ffi::c_void,
            0,
        ) != 0;

        let (work_right, work_bottom) = if has_work_area && work_area.right > work_area.left {
            (work_area.right, work_area.bottom)
        } else {
            (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN))
        };

        let start_x = if initial_pos.0 <= 0 {
            (work_right - PET_WINDOW_WIDTH - 24).max(10)
        } else {
            initial_pos.0
        };
        let start_y = if initial_pos.1 <= 0 {
            (work_bottom - PET_WINDOW_HEIGHT - 16).max(10)
        } else {
            initial_pos.1
        };

        let mut ex_style = WS_EX_LAYERED | WS_EX_TOOLWINDOW;
        if initial_always_on_top {
            ex_style |= WS_EX_TOPMOST;
        }

        let hwnd = CreateWindowExW(
            ex_style,
            class_name.as_ptr(),
            window_title.as_ptr(),
            WS_POPUP | WS_VISIBLE,
            start_x,
            start_y,
            PET_WINDOW_WIDTH,
            PET_WINDOW_HEIGHT,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            hinstance,
            std::ptr::null_mut(),
        );

        if hwnd.is_null() {
            error!("Failed creating OpenPet Desktop Pet Window!");
            DeleteObject(brush);
            return;
        }

        // Apply layered window chroma key transparency
        SetLayeredWindowAttributes(hwnd, PET_CHROMA_KEY, 255, LWA_COLORKEY);

        // Initialize System Tray
        let tray = SystemTray::new(
            hwnd,
            TrayState {
                privacy_mode: initial_privacy,
                pet_paused: false,
                pet_visible: true,
                art_style: initial_art_style,
                locale: initial_locale,
            },
        );

        let sprite_sheet = MimiSpriteSheet::generate_for_breed(initial_breed);
        let realistic_sheet = RealisticCompanionSheet::new();
        let chat_window = FloatingChatWindow::new(hinstance, initial_locale, chat_tx);

        let mut state = Box::new(PetWindowState {
            sprite_sheet,
            realistic_sheet,
            art_style: initial_art_style,
            current_frame_name: "idle_0".to_string(),
            current_behavior: BehaviorType::Idle,
            frame_counter: 0,
            is_dragging: false,
            drag_start_cursor: (0, 0),
            drag_start_win: (0, 0),
            has_moved_during_drag: false,
            pet_pos: (start_x, start_y),
            target_pos: None,
            pet_visible: true,
            pet_paused: false,
            privacy_mode: initial_privacy,
            locale: initial_locale,
            pet_mood_timer: 0,
            speech_bubble_text: String::new(),
            speech_bubble_timer: 0,
            tray: Some(tray),
            chat_window: Some(chat_window),
            interaction_tx: Some(interaction_tx),
            tray_action_tx: Some(tray_action_tx),
            shutdown_flag: shutdown_flag.clone(),
        });

        SetWindowLongPtrW(hwnd, GWLP_USERDATA, &mut *state as *mut _ as isize);

        // Start animation timer (~15 FPS / 66ms interval)
        windows_sys::Win32::UI::WindowsAndMessaging::SetTimer(hwnd, 1, 66, None);

        info!(
            "Desktop pet window and system tray running at ({}, {})",
            initial_pos.0, initial_pos.1
        );

        // Message pump with non-blocking command drain
        let mut msg: MSG = std::mem::zeroed();
        while !shutdown_flag.load(Ordering::SeqCst) {
            // Process incoming commands from background host
            while let Ok(cmd) = cmd_rx.try_recv() {
                match cmd {
                    PetWindowCommand::SetBehavior(b) => {
                        state.current_behavior = b;
                        if state.pet_mood_timer == 0 {
                            state.current_frame_name = match b {
                                BehaviorType::Sleep => "sleep_0".to_string(),
                                BehaviorType::Sit => "sit_0".to_string(),
                                BehaviorType::Surprised => "surprised_0".to_string(),
                                BehaviorType::Purr => "purr_0".to_string(),
                                BehaviorType::Knead => "knead_0".to_string(),
                                BehaviorType::Zoomies => "zoomies_0".to_string(),
                                BehaviorType::Loaf => "loaf_0".to_string(),
                                _ => "idle_0".to_string(),
                            };
                        }
                        InvalidateRect(hwnd, std::ptr::null(), 0);
                    }
                    PetWindowCommand::SetTargetPosition { x, y } => {
                        state.target_pos = Some((x, y));
                    }
                    PetWindowCommand::SetVisibility(visible) => {
                        state.pet_visible = visible;
                        if !visible {
                            if let Some(ref chat) = state.chat_window {
                                chat.set_visible(false);
                            }
                        }
                        if let Some(ref mut t) = state.tray {
                            t.update_state(TrayState {
                                privacy_mode: state.privacy_mode,
                                pet_paused: state.pet_paused,
                                pet_visible: state.pet_visible,
                                art_style: state.art_style,
                                locale: state.locale,
                            });
                        }
                        InvalidateRect(hwnd, std::ptr::null(), 0);
                    }
                    PetWindowCommand::SetPrivacyMode(p) => {
                        state.privacy_mode = p;
                        if let Some(ref mut t) = state.tray {
                            t.update_state(TrayState {
                                privacy_mode: state.privacy_mode,
                                pet_paused: state.pet_paused,
                                pet_visible: state.pet_visible,
                                art_style: state.art_style,
                                locale: state.locale,
                            });
                        }
                    }
                    PetWindowCommand::SetPaused(paused) => {
                        state.pet_paused = paused;
                    }
                    PetWindowCommand::SetLocale(locale) => {
                        state.locale = locale;
                        if let Some(ref mut t) = state.tray {
                            t.update_state(TrayState {
                                privacy_mode: state.privacy_mode,
                                pet_paused: state.pet_paused,
                                pet_visible: state.pet_visible,
                                art_style: state.art_style,
                                locale: state.locale,
                            });
                        }
                    }
                    PetWindowCommand::ShowChat(visible) => {
                        if let Some(ref chat) = state.chat_window {
                            if visible {
                                chat.show_near_pet(state.pet_pos);
                            } else {
                                chat.set_visible(false);
                            }
                        }
                    }
                    PetWindowCommand::AddChatMessage { sender, text } => {
                        if let Some(ref chat) = state.chat_window {
                            chat.add_message(&sender, &text, false);
                        }
                    }
                    PetWindowCommand::TriggerAction(interaction) => {
                        let is_tr = state.locale == SupportedLocale::TrTr;
                        match interaction {
                            InteractionType::Feed => {
                                state.current_frame_name = "eat_0".to_string();
                                state.pet_mood_timer = 25;
                                state.speech_bubble_text = if is_tr {
                                    "🐟 Ham ham! Çok lezzetli!".into()
                                } else {
                                    "🐟 Yum! Delicious treat!".into()
                                };
                                state.speech_bubble_timer = 35;
                            }
                            InteractionType::Water => {
                                state.current_frame_name = "drink_0".to_string();
                                state.pet_mood_timer = 25;
                                state.speech_bubble_text = if is_tr {
                                    "💧 Gluk gluk! Teşekkürler!".into()
                                } else {
                                    "💧 Refreshing water! Thanks!".into()
                                };
                                state.speech_bubble_timer = 35;
                            }
                            InteractionType::Groom => {
                                state.current_frame_name = "groom_0".to_string();
                                state.pet_mood_timer = 25;
                                state.speech_bubble_text = if is_tr {
                                    "🪮 Mırrr... Pırıl pırıl oldum!".into()
                                } else {
                                    "🪮 Purr... Groomed & shiny!".into()
                                };
                                state.speech_bubble_timer = 35;
                            }
                            InteractionType::Petting { .. } => {
                                state.current_frame_name = "purr_0".to_string();
                                state.pet_mood_timer = 20;
                                state.speech_bubble_text = if is_tr {
                                    "❤️ *mırrr mırrr*".into()
                                } else {
                                    "❤️ *purr purr*".into()
                                };
                                state.speech_bubble_timer = 30;
                            }
                            InteractionType::Play => {
                                state.current_frame_name = "jump_0".to_string();
                                state.pet_mood_timer = 20;
                                state.speech_bubble_text = if is_tr {
                                    "🧶 Yaşasın! Oyun zamanı!".into()
                                } else {
                                    "🧶 Yay! Playtime!".into()
                                };
                                state.speech_bubble_timer = 35;
                            }
                            InteractionType::HighFive => {
                                state.current_frame_name = "jump_0".to_string();
                                state.pet_mood_timer = 20;
                                state.speech_bubble_text = if is_tr {
                                    "🖐️ Çak bir beşlik!".into()
                                } else {
                                    "🖐️ High five!".into()
                                };
                                state.speech_bubble_timer = 35;
                            }
                            _ => {}
                        }
                        InvalidateRect(hwnd, std::ptr::null(), 0);
                    }
                    PetWindowCommand::SetBreed(breed) => {
                        state.sprite_sheet = MimiSpriteSheet::generate_for_breed(breed);
                        InvalidateRect(hwnd, std::ptr::null(), 0);
                    }
                    PetWindowCommand::SetArtStyle(style) => {
                        state.art_style = style;
                        if let Some(ref mut tray) = state.tray {
                            tray.update_state(TrayState {
                                privacy_mode: state.privacy_mode,
                                pet_paused: state.pet_paused,
                                pet_visible: state.pet_visible,
                                art_style: state.art_style,
                                locale: state.locale,
                            });
                        }
                        InvalidateRect(hwnd, std::ptr::null(), 0);
                    }
                    PetWindowCommand::SetAlwaysOnTop(top) => {
                        use windows_sys::Win32::UI::WindowsAndMessaging::{
                            SetWindowPos, HWND_NOTOPMOST, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE,
                            SWP_NOSIZE,
                        };
                        let insert_after = if top { HWND_TOPMOST } else { HWND_NOTOPMOST };
                        SetWindowPos(
                            hwnd,
                            insert_after,
                            0,
                            0,
                            0,
                            0,
                            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                        );
                    }
                    PetWindowCommand::PlayAnimation {
                        name,
                        duration_ticks,
                    } => {
                        state.current_frame_name = name;
                        state.pet_mood_timer = duration_ticks;
                        InvalidateRect(hwnd, std::ptr::null(), 0);
                    }
                    PetWindowCommand::Close => {
                        shutdown_flag.store(true, Ordering::SeqCst);
                        PostQuitMessage(0);
                    }
                }
            }

            // Pump Windows messages
            while PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
                if msg.message == windows_sys::Win32::UI::WindowsAndMessaging::WM_QUIT {
                    shutdown_flag.store(true, Ordering::SeqCst);
                    break;
                }
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }

            std::thread::sleep(std::time::Duration::from_millis(16));
        }

        // Clean up tray and brush
        if let Some(mut t) = state.tray.take() {
            t.remove();
        }
        DeleteObject(brush);
        info!("OpenPet desktop pet window loop terminated cleanly.");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pet_window_command_variants() {
        let cmd = PetWindowCommand::SetBehavior(BehaviorType::Sleep);
        assert!(matches!(
            cmd,
            PetWindowCommand::SetBehavior(BehaviorType::Sleep)
        ));

        let pos_cmd = PetWindowCommand::SetTargetPosition { x: 300, y: 400 };
        assert!(matches!(
            pos_cmd,
            PetWindowCommand::SetTargetPosition { x: 300, y: 400 }
        ));

        let close_cmd = PetWindowCommand::Close;
        assert!(matches!(close_cmd, PetWindowCommand::Close));

        let pause_cmd = PetWindowCommand::SetPaused(true);
        assert!(matches!(pause_cmd, PetWindowCommand::SetPaused(true)));

        let vis_cmd = PetWindowCommand::SetVisibility(false);
        assert!(matches!(vis_cmd, PetWindowCommand::SetVisibility(false)));

        let priv_cmd = PetWindowCommand::SetPrivacyMode(true);
        assert!(matches!(priv_cmd, PetWindowCommand::SetPrivacyMode(true)));

        let loc_cmd = PetWindowCommand::SetLocale(SupportedLocale::TrTr);
        assert!(matches!(
            loc_cmd,
            PetWindowCommand::SetLocale(SupportedLocale::TrTr)
        ));

        let chat_cmd = PetWindowCommand::ShowChat(true);
        assert!(matches!(chat_cmd, PetWindowCommand::ShowChat(true)));

        let add_msg_cmd = PetWindowCommand::AddChatMessage {
            sender: "Mimi".to_string(),
            text: "Hello".to_string(),
        };
        assert!(matches!(
            add_msg_cmd,
            PetWindowCommand::AddChatMessage { .. }
        ));

        let action_cmd = PetWindowCommand::TriggerAction(InteractionType::Feed);
        assert!(matches!(
            action_cmd,
            PetWindowCommand::TriggerAction(InteractionType::Feed)
        ));

        let sleep_cmd = PetWindowCommand::TriggerAction(InteractionType::SleepToggle);
        assert!(matches!(
            sleep_cmd,
            PetWindowCommand::TriggerAction(InteractionType::SleepToggle)
        ));

        let anim_cmd = PetWindowCommand::PlayAnimation {
            name: "eat_0".to_string(),
            duration_ticks: 30,
        };
        assert!(matches!(anim_cmd, PetWindowCommand::PlayAnimation { .. }));

        let breed_cmd = PetWindowCommand::SetBreed(openpet_types::CatBreed::Calico);
        assert!(matches!(breed_cmd, PetWindowCommand::SetBreed(_)));

        let on_top_cmd = PetWindowCommand::SetAlwaysOnTop(true);
        assert!(matches!(on_top_cmd, PetWindowCommand::SetAlwaysOnTop(true)));

        let style_cmd = PetWindowCommand::SetArtStyle(CompanionArtStyle::Realistic);
        assert!(matches!(
            style_cmd,
            PetWindowCommand::SetArtStyle(CompanionArtStyle::Realistic)
        ));
    }

    #[test]
    fn test_resolve_realistic_frame_mapping() {
        // Sleep state
        assert_eq!(
            resolve_realistic_frame(BehaviorType::Sleep, "sleep_0", false),
            "sleep_0"
        );
        // Loaf maps to sleep pose
        assert_eq!(
            resolve_realistic_frame(BehaviorType::Loaf, "loaf_0", false),
            "sleep_0"
        );
        // Stretch
        assert_eq!(
            resolve_realistic_frame(BehaviorType::Stretch, "stretch_0", false),
            "stretch_0"
        );
        // Curious
        assert_eq!(
            resolve_realistic_frame(BehaviorType::Curious, "curious_0", false),
            "curious_0"
        );
        // Chat open takes precedence
        assert_eq!(
            resolve_realistic_frame(BehaviorType::Idle, "idle_0", true),
            "ask_0"
        );
        // High five
        assert_eq!(
            resolve_realistic_frame(BehaviorType::HighFive, "jump_0", false),
            "high_five_0"
        );
        // Idle / Sit / Happy default to sit_0
        assert_eq!(
            resolve_realistic_frame(BehaviorType::Idle, "idle_0", false),
            "sit_0"
        );
        assert_eq!(
            resolve_realistic_frame(BehaviorType::Sit, "sit_0", false),
            "sit_0"
        );
        assert_eq!(
            resolve_realistic_frame(BehaviorType::Happy, "happy_0", false),
            "sit_0"
        );
    }
}
