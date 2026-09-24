//! # Windows Desktop Pet Window
//!
//! Native Win32 borderless, transparent, draggable desktop pet window.
//! Handles real-time animations, mouse interactions (petting, dragging, context menu),
//! and coordinates with the behavior engine.

use crate::tray::{SystemTray, TrayAction, TrayState, WM_TRAY_CALLBACK};
use openpet_render::mimi::MimiSpriteSheet;
use openpet_types::{BehaviorType, InteractionType, SupportedLocale};
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use tracing::{error, info};

pub const PET_WINDOW_WIDTH: i32 = 128;
pub const PET_WINDOW_HEIGHT: i32 = 128;
pub const PET_CHROMA_KEY: u32 = 0x00FF00FF; // Magenta chroma key

/// Commands that can be dispatched to the active pet window from host threads.
#[derive(Debug)]
pub enum PetWindowCommand {
    SetBehavior(BehaviorType),
    SetTargetPosition { x: i32, y: i32 },
    SetVisibility(bool),
    SetPrivacyMode(bool),
    SetPaused(bool),
    SetLocale(SupportedLocale),
    Close,
}

/// Internal state owned by the pet window Win32 message procedure.
pub struct PetWindowState {
    pub sprite_sheet: MimiSpriteSheet,
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
    pub tray: Option<SystemTray>,
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
                    let (bgra, w, h) = state.sprite_sheet.render_frame_bgra_scaled(
                        &state.current_frame_name,
                        2,
                        Some(PET_CHROMA_KEY),
                    );

                    let mut bmi: BITMAPINFO = std::mem::zeroed();
                    bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
                    bmi.bmiHeader.biWidth = w as i32;
                    bmi.bmiHeader.biHeight = -(h as i32); // Top-down
                    bmi.bmiHeader.biPlanes = 1;
                    bmi.bmiHeader.biBitCount = 32;
                    bmi.bmiHeader.biCompression = BI_RGB;

                    SetDIBitsToDevice(
                        mem_dc,
                        0,
                        0,
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
                        state.current_frame_name = "pet_0".to_string();
                        state.pet_mood_timer = 15; // Show happy pet reaction for 15 ticks (~1.5s)
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
        WM_RBUTTONUP => {
            // Right clicking on the pet displays the context menu!
            if !state_ptr.is_null() {
                let state = &mut *state_ptr;
                if let Some(ref tray) = state.tray {
                    if let Some(action) = tray.show_context_menu(hwnd) {
                        handle_tray_action(hwnd, state, action);
                    }
                }
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

                // Temporary petting emotion countdown
                if state.pet_mood_timer > 0 {
                    state.pet_mood_timer -= 1;
                    if state.pet_mood_timer == 0 {
                        state.current_frame_name = "idle_0".to_string();
                        InvalidateRect(hwnd, std::ptr::null(), 0);
                    }
                } else if !state.is_dragging && !state.pet_paused {
                    // Smooth locomotion interpolation if target position is set
                    if let Some(target) = state.target_pos {
                        let dx = target.0 - state.pet_pos.0;
                        let dy = target.1 - state.pet_pos.1;
                        let dist_sq = dx * dx + dy * dy;

                        if dist_sq > 16 {
                            let step_x = (dx.clamp(-4, 4)) as i32;
                            let step_y = (dy.clamp(-2, 2)) as i32;
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
                            state.current_behavior = BehaviorType::Walk;
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
                        BehaviorType::Sit => "sit_0",
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

                if (0..PET_WINDOW_WIDTH).contains(&rel_x) && (0..PET_WINDOW_HEIGHT).contains(&rel_y)
                {
                    let sprite_x = (rel_x / 2).clamp(0, 63) as usize;
                    let sprite_y = (rel_y / 2).clamp(0, 63) as usize;
                    let frame = state.sprite_sheet.get_frame(&state.current_frame_name);
                    let px = frame.get_pixel(sprite_x, sprite_y);
                    if px[3] > 0 {
                        return windows_sys::Win32::UI::WindowsAndMessaging::HTCLIENT as isize;
                    } else {
                        return windows_sys::Win32::UI::WindowsAndMessaging::HTTRANSPARENT as isize;
                    }
                }
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
pub fn spawn_desktop_pet_window(
    initial_pos: (i32, i32),
    initial_locale: SupportedLocale,
    initial_privacy: bool,
    interaction_tx: mpsc::Sender<InteractionType>,
    tray_action_tx: mpsc::Sender<TrayAction>,
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
                interaction_tx,
                tray_action_tx,
                cmd_rx,
                shutdown_flag,
            );

            #[cfg(not(windows))]
            {
                let _ = (
                    initial_pos,
                    initial_locale,
                    initial_privacy,
                    interaction_tx,
                    tray_action_tx,
                    cmd_rx,
                    shutdown_flag,
                );
            }
        })
        .expect("Failed spawning OpenPet Windows UI thread")
}

#[cfg(windows)]
fn run_pet_window_loop(
    initial_pos: (i32, i32),
    initial_locale: SupportedLocale,
    initial_privacy: bool,
    interaction_tx: mpsc::Sender<InteractionType>,
    tray_action_tx: mpsc::Sender<TrayAction>,
    cmd_rx: mpsc::Receiver<PetWindowCommand>,
    shutdown_flag: Arc<AtomicBool>,
) {
    use windows_sys::Win32::Graphics::Gdi::{CreateSolidBrush, DeleteObject, InvalidateRect};
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DispatchMessageW, GetSystemMetrics, LoadCursorW, LoadIconW, PeekMessageW,
        PostQuitMessage, RegisterClassExW, SetLayeredWindowAttributes, SetWindowLongPtrW,
        TranslateMessage, CS_HREDRAW, CS_VREDRAW, GWLP_USERDATA, IDC_ARROW, IDI_APPLICATION,
        LWA_COLORKEY, MSG, PM_REMOVE, SM_CXSCREEN, SM_CYSCREEN, WNDCLASSEXW, WS_EX_LAYERED,
        WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP, WS_VISIBLE,
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
            style: CS_HREDRAW | CS_VREDRAW,
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

        let hwnd = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
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
                locale: initial_locale,
            },
        );

        let sprite_sheet = MimiSpriteSheet::generate();

        let mut state = Box::new(PetWindowState {
            sprite_sheet,
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
            tray: Some(tray),
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
                        InvalidateRect(hwnd, std::ptr::null(), 0);
                    }
                    PetWindowCommand::SetTargetPosition { x, y } => {
                        state.target_pos = Some((x, y));
                    }
                    PetWindowCommand::SetVisibility(visible) => {
                        state.pet_visible = visible;
                        if let Some(ref mut t) = state.tray {
                            t.update_state(TrayState {
                                privacy_mode: state.privacy_mode,
                                pet_paused: state.pet_paused,
                                pet_visible: state.pet_visible,
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
                                locale: state.locale,
                            });
                        }
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
    }
}
