//! # Floating Rounded Chat Bubble Window
//!
//! Win32 popup layered window styled as a modern dark floating chat bubble
//! positioned near the desktop companion. Allows instant conversation,
//! memory queries, and emotional responses from Mimi the Cat.

use openpet_types::SupportedLocale;
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::sync::atomic::{AtomicIsize, Ordering};
use std::sync::mpsc;
use tracing::info;

pub const CHAT_WINDOW_WIDTH: i32 = 340;
pub const CHAT_WINDOW_HEIGHT: i32 = 420;
pub const CHAT_CORNER_RADIUS: i32 = 16;

pub const CMD_EDIT_INPUT: usize = 2001;
pub const CMD_BTN_SEND: usize = 2002;
pub const CMD_BTN_CLOSE: usize = 2003;

/// A single message entry displayed in the chat history bubble list.
#[derive(Debug, Clone)]
pub struct ChatMessageItem {
    pub sender: String,
    pub text: String,
    pub is_user: bool,
}

/// Computes message bubble height based on available vertical space and item count.
pub fn compute_bubble_height(available_height: i32, display_count: usize) -> i32 {
    let count = display_count.max(1) as i32;
    let total_gaps = (count - 1) * 8;
    ((available_height - total_gaps) / count).clamp(60, 75)
}

/// Calculates the docked screen position of the chat window relative to the pet.
pub fn compute_dock_position(
    pet_pos: (i32, i32),
    screen_w: i32,
    screen_h: i32,
    window_w: i32,
    window_h: i32,
) -> (i32, i32) {
    let dock_x = if pet_pos.0 + 130 + window_w <= screen_w - 20 {
        pet_pos.0 + 130
    } else {
        (pet_pos.0 - window_w - 10).max(10)
    };

    let max_y = (screen_h - window_h - 40).max(20);
    let dock_y = (pet_pos.1 - 100).clamp(20, max_y);
    (dock_x, dock_y)
}

/// Internal state owned by the floating chat window Win32 message procedure.
pub struct FloatingChatState {
    pub edit_hwnd: windows_sys::Win32::Foundation::HWND,
    pub send_btn_hwnd: windows_sys::Win32::Foundation::HWND,
    pub history: Vec<ChatMessageItem>,
    pub chat_tx: Option<mpsc::Sender<String>>,
    pub is_visible: bool,
    pub locale: SupportedLocale,
    pub dark_bg_brush: windows_sys::Win32::Graphics::Gdi::HBRUSH,
    pub input_bg_brush: windows_sys::Win32::Graphics::Gdi::HBRUSH,
    pub bubble_user_brush: windows_sys::Win32::Graphics::Gdi::HBRUSH,
    pub bubble_cat_brush: windows_sys::Win32::Graphics::Gdi::HBRUSH,
    pub font: windows_sys::Win32::Graphics::Gdi::HFONT,
    pub title_font: windows_sys::Win32::Graphics::Gdi::HFONT,
}

#[cfg(windows)]
static ORIGINAL_EDIT_PROC: AtomicIsize = AtomicIsize::new(0);

#[cfg(windows)]
unsafe extern "system" fn edit_subclass_proc(
    hwnd: windows_sys::Win32::Foundation::HWND,
    msg: u32,
    wparam: windows_sys::Win32::Foundation::WPARAM,
    lparam: windows_sys::Win32::Foundation::LPARAM,
) -> windows_sys::Win32::Foundation::LRESULT {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{VK_ESCAPE, VK_RETURN};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CallWindowProcW, GetParent, PostMessageW, WM_CHAR, WM_CLOSE, WM_COMMAND, WM_KEYDOWN,
    };

    // Intercept character generation for Return and Escape to suppress system warning beep
    if msg == WM_CHAR
        && (wparam == VK_RETURN as usize
            || wparam == 13
            || wparam == VK_ESCAPE as usize
            || wparam == 27)
    {
        return 0;
    }

    if msg == WM_KEYDOWN {
        if wparam == VK_RETURN as usize {
            let parent = GetParent(hwnd);
            PostMessageW(parent, WM_COMMAND, CMD_BTN_SEND, hwnd as isize);
            return 0;
        } else if wparam == VK_ESCAPE as usize {
            let parent = GetParent(hwnd);
            PostMessageW(parent, WM_CLOSE, 0, 0);
            return 0;
        }
    }

    let orig = ORIGINAL_EDIT_PROC.load(Ordering::SeqCst);
    if orig != 0 {
        let orig_fn: unsafe extern "system" fn(
            windows_sys::Win32::Foundation::HWND,
            u32,
            windows_sys::Win32::Foundation::WPARAM,
            windows_sys::Win32::Foundation::LPARAM,
        ) -> windows_sys::Win32::Foundation::LRESULT = std::mem::transmute(orig);
        CallWindowProcW(Some(orig_fn), hwnd, msg, wparam, lparam)
    } else {
        windows_sys::Win32::UI::WindowsAndMessaging::DefWindowProcW(hwnd, msg, wparam, lparam)
    }
}

#[cfg(windows)]
unsafe extern "system" fn chat_window_wndproc(
    hwnd: windows_sys::Win32::Foundation::HWND,
    msg: u32,
    wparam: windows_sys::Win32::Foundation::WPARAM,
    lparam: windows_sys::Win32::Foundation::LPARAM,
) -> windows_sys::Win32::Foundation::LRESULT {
    use windows_sys::Win32::Foundation::{POINT, RECT};
    use windows_sys::Win32::Graphics::Gdi::{
        BeginPaint, BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject,
        DrawTextW, EndPaint, FillRect, GetStockObject, InvalidateRect, RoundRect, SelectObject,
        SetBkColor, SetBkMode, SetTextColor, DT_LEFT, DT_NOPREFIX, DT_WORDBREAK, NULL_PEN,
        PAINTSTRUCT, SRCCOPY, TRANSPARENT,
    };
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{ReleaseCapture, SetFocus};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        DefWindowProcW, GetCursorPos, GetWindowLongPtrW, GetWindowRect, GetWindowTextLengthW,
        GetWindowTextW, SendMessageW, SetWindowLongPtrW, SetWindowTextW, ShowWindow, GWLP_USERDATA,
        HTCAPTION, SW_HIDE, WM_CLOSE, WM_COMMAND, WM_CREATE, WM_CTLCOLORBTN, WM_CTLCOLOREDIT,
        WM_CTLCOLORSTATIC, WM_DESTROY, WM_ERASEBKGND, WM_LBUTTONDOWN, WM_NCLBUTTONDOWN, WM_PAINT,
    };

    let state_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut FloatingChatState;

    match msg {
        WM_CREATE => 0,
        WM_ERASEBKGND => 1,
        WM_PAINT => {
            if !state_ptr.is_null() {
                let state = &mut *state_ptr;
                let mut ps: PAINTSTRUCT = std::mem::zeroed();
                let hdc = BeginPaint(hwnd, &mut ps);

                let mem_dc = CreateCompatibleDC(hdc);
                // CRITICAL GDI FIX: Must pass screen hdc, not mem_dc (which defaults to 1-bpp monochrome)
                let mem_bmp = CreateCompatibleBitmap(hdc, CHAT_WINDOW_WIDTH, CHAT_WINDOW_HEIGHT);
                let old_bmp = SelectObject(mem_dc, mem_bmp);
                let null_pen = GetStockObject(NULL_PEN);
                let old_pen = SelectObject(mem_dc, null_pen);

                // 1. Fill base dark acrylic background (#1E1E2E)
                let bg_rect = RECT {
                    left: 0,
                    top: 0,
                    right: CHAT_WINDOW_WIDTH,
                    bottom: CHAT_WINDOW_HEIGHT,
                };
                FillRect(mem_dc, &bg_rect, state.dark_bg_brush);

                // 2. Draw Header Bar (#181825)
                let header_rect = RECT {
                    left: 0,
                    top: 0,
                    right: CHAT_WINDOW_WIDTH,
                    bottom: 36,
                };
                FillRect(mem_dc, &header_rect, state.input_bg_brush);

                // Header Title ("Mimi the Cat 🐾" or "Mimi ile Sohbet 🐾")
                SelectObject(mem_dc, state.title_font);
                SetBkMode(mem_dc, TRANSPARENT as i32);
                SetTextColor(mem_dc, 0x00F4D6CD); // Catppuccin Text #CDD6F4 (0x00BBGGRR)

                let title = if state.locale == SupportedLocale::TrTr {
                    "Mimi ile Sohbet 🐾"
                } else {
                    "Chat with Mimi 🐾"
                };
                let wide_title: Vec<u16> = OsStr::new(title)
                    .encode_wide()
                    .chain(std::iter::once(0))
                    .collect();
                let mut title_rect = RECT {
                    left: 14,
                    top: 8,
                    right: CHAT_WINDOW_WIDTH - 40,
                    bottom: 32,
                };
                DrawTextW(
                    mem_dc,
                    wide_title.as_ptr(),
                    -1,
                    &mut title_rect,
                    DT_LEFT | DT_NOPREFIX,
                );

                // Close Button ("✕")
                let close_sym: Vec<u16> = OsStr::new("✕")
                    .encode_wide()
                    .chain(std::iter::once(0))
                    .collect();
                let mut close_rect = RECT {
                    left: CHAT_WINDOW_WIDTH - 30,
                    top: 8,
                    right: CHAT_WINDOW_WIDTH - 10,
                    bottom: 30,
                };
                SetTextColor(mem_dc, 0x00A88BF3); // Pinkish red #F38BA8
                DrawTextW(
                    mem_dc,
                    close_sym.as_ptr(),
                    -1,
                    &mut close_rect,
                    DT_LEFT | DT_NOPREFIX,
                );

                // 3. Render Message History Bubbles
                SelectObject(mem_dc, state.font);
                let history_start_y = 44;
                let history_end_y = CHAT_WINDOW_HEIGHT - 60; // 360
                let available_height = history_end_y - history_start_y; // 316

                // Take last 4 messages to fit cleanly
                let count = state.history.len().min(4);
                let start_idx = state.history.len().saturating_sub(count);
                let messages_to_show = &state.history[start_idx..];

                // Dynamically allocate bubble height subtracting inter-bubble gaps
                let bubble_h = compute_bubble_height(available_height, messages_to_show.len());

                for (i, item) in messages_to_show.iter().enumerate() {
                    let top = history_start_y + (i as i32 * (bubble_h + 8));
                    let bottom = top + bubble_h;

                    if bottom > history_end_y {
                        break;
                    }

                    if item.is_user {
                        // User message: right aligned, slate dark bubble
                        let bubble_rect = RECT {
                            left: 50,
                            top,
                            right: CHAT_WINDOW_WIDTH - 14,
                            bottom,
                        };
                        SelectObject(mem_dc, state.bubble_user_brush);
                        RoundRect(
                            mem_dc,
                            bubble_rect.left,
                            bubble_rect.top,
                            bubble_rect.right,
                            bubble_rect.bottom,
                            12,
                            12,
                        );

                        // User label
                        SetTextColor(mem_dc, 0x00FAB489); // Light Blue #89B4FA
                        let user_lbl: Vec<u16> =
                            OsStr::new(if state.locale == SupportedLocale::TrTr {
                                "Sen:"
                            } else {
                                "You:"
                            })
                            .encode_wide()
                            .chain(std::iter::once(0))
                            .collect();
                        let mut lbl_rect = RECT {
                            left: bubble_rect.left + 8,
                            top: bubble_rect.top + 4,
                            right: bubble_rect.right - 8,
                            bottom: bubble_rect.top + 20,
                        };
                        DrawTextW(
                            mem_dc,
                            user_lbl.as_ptr(),
                            -1,
                            &mut lbl_rect,
                            DT_LEFT | DT_NOPREFIX,
                        );

                        // User text
                        SetTextColor(mem_dc, 0x00FFFFFF);
                        let wide_text: Vec<u16> = OsStr::new(&item.text)
                            .encode_wide()
                            .chain(std::iter::once(0))
                            .collect();
                        let mut text_rect = RECT {
                            left: bubble_rect.left + 8,
                            top: bubble_rect.top + 20,
                            right: bubble_rect.right - 8,
                            bottom: bubble_rect.bottom - 4,
                        };
                        DrawTextW(
                            mem_dc,
                            wide_text.as_ptr(),
                            -1,
                            &mut text_rect,
                            DT_LEFT | DT_WORDBREAK | DT_NOPREFIX | 0x00008000, // 0x8000 = DT_END_ELLIPSIS
                        );
                    } else {
                        // Mimi message: left aligned, warm mauve/gray bubble
                        let bubble_rect = RECT {
                            left: 14,
                            top,
                            right: CHAT_WINDOW_WIDTH - 50,
                            bottom,
                        };
                        SelectObject(mem_dc, state.bubble_cat_brush);
                        RoundRect(
                            mem_dc,
                            bubble_rect.left,
                            bubble_rect.top,
                            bubble_rect.right,
                            bubble_rect.bottom,
                            12,
                            12,
                        );

                        // Mimi label
                        SetTextColor(mem_dc, 0x0087B3FA); // Peach #FAB387
                        let mimi_lbl: Vec<u16> = OsStr::new("🐾 Mimi:")
                            .encode_wide()
                            .chain(std::iter::once(0))
                            .collect();
                        let mut lbl_rect = RECT {
                            left: bubble_rect.left + 8,
                            top: bubble_rect.top + 4,
                            right: bubble_rect.right - 8,
                            bottom: bubble_rect.top + 20,
                        };
                        DrawTextW(
                            mem_dc,
                            mimi_lbl.as_ptr(),
                            -1,
                            &mut lbl_rect,
                            DT_LEFT | DT_NOPREFIX,
                        );

                        // Mimi text
                        SetTextColor(mem_dc, 0x00F4D6CD);
                        let wide_text: Vec<u16> = OsStr::new(&item.text)
                            .encode_wide()
                            .chain(std::iter::once(0))
                            .collect();
                        let mut text_rect = RECT {
                            left: bubble_rect.left + 8,
                            top: bubble_rect.top + 20,
                            right: bubble_rect.right - 8,
                            bottom: bubble_rect.bottom - 4,
                        };
                        DrawTextW(
                            mem_dc,
                            wide_text.as_ptr(),
                            -1,
                            &mut text_rect,
                            DT_LEFT | DT_WORDBREAK | DT_NOPREFIX | 0x00008000, // 0x8000 = DT_END_ELLIPSIS
                        );
                    }
                }

                // 4. Draw Input Border Divider Bar
                let input_divider = RECT {
                    left: 0,
                    top: CHAT_WINDOW_HEIGHT - 54,
                    right: CHAT_WINDOW_WIDTH,
                    bottom: CHAT_WINDOW_HEIGHT,
                };
                FillRect(mem_dc, &input_divider, state.input_bg_brush);

                // Blit back buffer to screen DC
                BitBlt(
                    hdc,
                    0,
                    0,
                    CHAT_WINDOW_WIDTH,
                    CHAT_WINDOW_HEIGHT,
                    mem_dc,
                    0,
                    0,
                    SRCCOPY,
                );

                SelectObject(mem_dc, old_pen);
                SelectObject(mem_dc, old_bmp);
                DeleteObject(mem_bmp);
                DeleteDC(mem_dc);
                EndPaint(hwnd, &ps);
            }
            0
        }
        WM_LBUTTONDOWN => {
            if !state_ptr.is_null() {
                let mut pt: POINT = std::mem::zeroed();
                GetCursorPos(&mut pt);
                let mut win_rect: RECT = std::mem::zeroed();
                GetWindowRect(hwnd, &mut win_rect);

                let rel_x = pt.x - win_rect.left;
                let rel_y = pt.y - win_rect.top;

                // Close button clicked (top-right)
                if (CHAT_WINDOW_WIDTH - 36..CHAT_WINDOW_WIDTH).contains(&rel_x)
                    && (0..34).contains(&rel_y)
                {
                    ShowWindow(hwnd, SW_HIDE);
                    let state = &mut *state_ptr;
                    state.is_visible = false;
                    return 0;
                }

                // Header bar clicked: initiate native dragging with packed cursor position
                if (0..CHAT_WINDOW_WIDTH - 36).contains(&rel_x) && (0..36).contains(&rel_y) {
                    ReleaseCapture();
                    let lp = (((pt.y as i16 as u16 as u32) << 16) | (pt.x as i16 as u16 as u32))
                        as isize;
                    SendMessageW(hwnd, WM_NCLBUTTONDOWN, HTCAPTION as usize, lp);
                    return 0;
                }
            }
            0
        }
        WM_CTLCOLOREDIT | WM_CTLCOLORSTATIC | WM_CTLCOLORBTN => {
            if !state_ptr.is_null() {
                let state = &*state_ptr;
                let hdc = wparam as windows_sys::Win32::Graphics::Gdi::HDC;
                SetTextColor(hdc, 0x00F4D6CD);
                SetBkColor(hdc, 0x00443231);
                return state.bubble_user_brush as isize;
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_COMMAND => {
            let cmd_id = wparam & 0xFFFF;
            if cmd_id == CMD_BTN_SEND && !state_ptr.is_null() {
                let state = &mut *state_ptr;
                let len = GetWindowTextLengthW(state.edit_hwnd);
                if len > 0 {
                    let mut buf: Vec<u16> = vec![0; (len + 1) as usize];
                    GetWindowTextW(state.edit_hwnd, buf.as_mut_ptr(), len + 1);
                    let text = String::from_utf16_lossy(&buf[..len as usize]);
                    let trimmed = text.trim();
                    if !trimmed.is_empty() {
                        // 1. Add user message to history
                        state.history.push(ChatMessageItem {
                            sender: if state.locale == SupportedLocale::TrTr {
                                "Sen".to_string()
                            } else {
                                "You".to_string()
                            },
                            text: trimmed.to_string(),
                            is_user: true,
                        });

                        // 2. Clear input
                        let empty: Vec<u16> = vec![0];
                        SetWindowTextW(state.edit_hwnd, empty.as_ptr());

                        // 3. Dispatch to host channel
                        if let Some(ref tx) = state.chat_tx {
                            let _ = tx.send(trimmed.to_string());
                        }

                        // 4. Retain focus on edit field for smooth typing
                        SetFocus(state.edit_hwnd);

                        // 5. Redraw
                        InvalidateRect(hwnd, std::ptr::null(), 0);
                    }
                }
            }
            0
        }
        WM_CLOSE => {
            ShowWindow(hwnd, SW_HIDE);
            if !state_ptr.is_null() {
                let state = &mut *state_ptr;
                state.is_visible = false;
            }
            0
        }
        WM_DESTROY => {
            if !state_ptr.is_null() {
                let state = &mut *state_ptr;
                DeleteObject(state.dark_bg_brush);
                DeleteObject(state.input_bg_brush);
                DeleteObject(state.bubble_user_brush);
                DeleteObject(state.bubble_cat_brush);
                DeleteObject(state.font);
                DeleteObject(state.title_font);

                // Cleanly reclaim the boxed state heap allocation
                let _ = Box::from_raw(state_ptr);
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            }
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

/// Floating chat window controller managing lifecycle, snapping, and messages.
pub struct FloatingChatWindow {
    #[cfg(windows)]
    pub hwnd: windows_sys::Win32::Foundation::HWND,
}

impl Drop for FloatingChatWindow {
    fn drop(&mut self) {
        #[cfg(windows)]
        unsafe {
            use windows_sys::Win32::UI::WindowsAndMessaging::DestroyWindow;
            if !self.hwnd.is_null() {
                DestroyWindow(self.hwnd);
                self.hwnd = std::ptr::null_mut();
            }
        }
    }
}

impl FloatingChatWindow {
    /// Creates and configures the native floating chat bubble window.
    ///
    /// # Safety
    ///
    /// The caller must ensure that `hinstance` is a valid Win32 module handle
    /// owned by the active process.
    #[cfg(windows)]
    pub unsafe fn new(
        hinstance: windows_sys::Win32::Foundation::HMODULE,
        locale: SupportedLocale,
        chat_tx: mpsc::Sender<String>,
    ) -> Self {
        use windows_sys::Win32::Graphics::Gdi::{
            CreateFontW, CreateRoundRectRgn, CreateSolidBrush, SetWindowRgn, FW_BOLD, FW_SEMIBOLD,
        };
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            CreateWindowExW, LoadCursorW, RegisterClassExW, SetLayeredWindowAttributes,
            SetWindowLongPtrW, CS_HREDRAW, CS_VREDRAW, GWLP_USERDATA, GWLP_WNDPROC, IDC_ARROW,
            LWA_ALPHA, WNDCLASSEXW, WS_CHILD, WS_EX_LAYERED, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
            WS_POPUP, WS_TABSTOP, WS_VISIBLE,
        };

        unsafe {
            let class_name: Vec<u16> = OsStr::new("OpenPet_FloatingChatWindowClass")
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();

            let dark_bg_brush = CreateSolidBrush(0x002E1E1E); // Catppuccin Base #1E1E2E (0x00BBGGRR)
            let input_bg_brush = CreateSolidBrush(0x00251818); // Header/Footer #181825
            let bubble_user_brush = CreateSolidBrush(0x00443231); // Surface0 #313244
            let bubble_cat_brush = CreateSolidBrush(0x005A4745); // Surface1 #45475A

            let wnd_class = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(chat_window_wndproc),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: hinstance,
                hIcon: std::ptr::null_mut(),
                hCursor: LoadCursorW(std::ptr::null_mut(), IDC_ARROW),
                hbrBackground: std::ptr::null_mut(),
                lpszMenuName: std::ptr::null(),
                lpszClassName: class_name.as_ptr(),
                hIconSm: std::ptr::null_mut(),
            };
            RegisterClassExW(&wnd_class);

            let win_title: Vec<u16> = OsStr::new("OpenPet Chat Bubble")
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();

            let hwnd = CreateWindowExW(
                WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
                class_name.as_ptr(),
                win_title.as_ptr(),
                WS_POPUP,
                100,
                100,
                CHAT_WINDOW_WIDTH,
                CHAT_WINDOW_HEIGHT,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                hinstance,
                std::ptr::null_mut(),
            );

            // Apply 16px smooth rounded region matching full client area
            let rgn = CreateRoundRectRgn(
                0,
                0,
                CHAT_WINDOW_WIDTH + 1,
                CHAT_WINDOW_HEIGHT + 1,
                CHAT_CORNER_RADIUS,
                CHAT_CORNER_RADIUS,
            );
            SetWindowRgn(hwnd, rgn, 1);

            // Frosted / dark acrylic ~96% opacity
            SetLayeredWindowAttributes(hwnd, 0, 246, LWA_ALPHA);

            // Segoe UI font for text & title
            let font_name: Vec<u16> = OsStr::new("Segoe UI")
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();
            let font = CreateFontW(
                15,
                0,
                0,
                0,
                FW_SEMIBOLD as i32,
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
            let title_font = CreateFontW(
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

            // Child 1: Edit Input Box (single line edit control)
            let edit_class: Vec<u16> = OsStr::new("EDIT")
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();
            let edit_hwnd = CreateWindowExW(
                0,
                edit_class.as_ptr(),
                std::ptr::null(),
                WS_CHILD | WS_VISIBLE | WS_TABSTOP | 0x0080, // 0x0080 = ES_AUTOHSCROLL
                12,
                CHAT_WINDOW_HEIGHT - 44,
                CHAT_WINDOW_WIDTH - 96,
                30,
                hwnd,
                CMD_EDIT_INPUT as windows_sys::Win32::UI::WindowsAndMessaging::HMENU,
                hinstance,
                std::ptr::null_mut(),
            );

            // Set font on edit box
            windows_sys::Win32::UI::WindowsAndMessaging::SendMessageW(
                edit_hwnd,
                windows_sys::Win32::UI::WindowsAndMessaging::WM_SETFONT,
                font as usize,
                1,
            );

            // Subclass edit control for Enter key submission
            let old_proc = SetWindowLongPtrW(
                edit_hwnd,
                GWLP_WNDPROC,
                edit_subclass_proc as *const () as isize,
            );
            ORIGINAL_EDIT_PROC.store(old_proc, Ordering::SeqCst);

            // Child 2: Send Button
            let btn_class: Vec<u16> = OsStr::new("BUTTON")
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();
            let btn_label = if locale == SupportedLocale::TrTr {
                "Gönder"
            } else {
                "Send"
            };
            let btn_wide: Vec<u16> = OsStr::new(btn_label)
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();
            let send_btn_hwnd = CreateWindowExW(
                0,
                btn_class.as_ptr(),
                btn_wide.as_ptr(),
                WS_CHILD | WS_VISIBLE | WS_TABSTOP,
                CHAT_WINDOW_WIDTH - 76,
                CHAT_WINDOW_HEIGHT - 44,
                64,
                30,
                hwnd,
                CMD_BTN_SEND as windows_sys::Win32::UI::WindowsAndMessaging::HMENU,
                hinstance,
                std::ptr::null_mut(),
            );

            windows_sys::Win32::UI::WindowsAndMessaging::SendMessageW(
                send_btn_hwnd,
                windows_sys::Win32::UI::WindowsAndMessaging::WM_SETFONT,
                font as usize,
                1,
            );

            // Initial welcoming greeting
            let initial_greeting = if locale == SupportedLocale::TrTr {
                "Miyav! Nasılsın? Benimle konuşabilirsin 🐾"
            } else {
                "Meow! How are you? Feel free to chat with me 🐾"
            };

            let state = Box::new(FloatingChatState {
                edit_hwnd,
                send_btn_hwnd,
                history: vec![ChatMessageItem {
                    sender: "Mimi".to_string(),
                    text: initial_greeting.to_string(),
                    is_user: false,
                }],
                chat_tx: Some(chat_tx),
                is_visible: false,
                locale,
                dark_bg_brush,
                input_bg_brush,
                bubble_user_brush,
                bubble_cat_brush,
                font,
                title_font,
            });

            SetWindowLongPtrW(hwnd, GWLP_USERDATA, Box::into_raw(state) as isize);

            info!("Floating rounded chat bubble window created successfully.");
            Self { hwnd }
        }
    }

    #[cfg(not(windows))]
    pub fn new(
        _hinstance: isize,
        _locale: SupportedLocale,
        _chat_tx: mpsc::Sender<String>,
    ) -> Self {
        Self {}
    }

    /// Shows or hides the chat bubble.
    pub fn set_visible(&self, visible: bool) {
        #[cfg(windows)]
        unsafe {
            use windows_sys::Win32::Graphics::Gdi::InvalidateRect;
            use windows_sys::Win32::UI::Input::KeyboardAndMouse::SetFocus;
            use windows_sys::Win32::UI::WindowsAndMessaging::{
                GetWindowLongPtrW, SetForegroundWindow, ShowWindow, GWLP_USERDATA, SW_HIDE, SW_SHOW,
            };
            if self.hwnd.is_null() {
                return;
            }
            if visible {
                ShowWindow(self.hwnd, SW_SHOW);
                SetForegroundWindow(self.hwnd);
                let state_ptr =
                    GetWindowLongPtrW(self.hwnd, GWLP_USERDATA) as *mut FloatingChatState;
                if !state_ptr.is_null() {
                    let state = &mut *state_ptr;
                    state.is_visible = true;
                    SetFocus(state.edit_hwnd);
                    InvalidateRect(self.hwnd, std::ptr::null(), 0);
                }
            } else {
                ShowWindow(self.hwnd, SW_HIDE);
                let state_ptr =
                    GetWindowLongPtrW(self.hwnd, GWLP_USERDATA) as *mut FloatingChatState;
                if !state_ptr.is_null() {
                    let state = &mut *state_ptr;
                    state.is_visible = false;
                }
            }
        }
    }

    /// Returns whether the floating chat window is currently visible.
    pub fn is_visible(&self) -> bool {
        #[cfg(windows)]
        unsafe {
            use windows_sys::Win32::UI::WindowsAndMessaging::{GetWindowLongPtrW, GWLP_USERDATA};
            if self.hwnd.is_null() {
                return false;
            }
            let state_ptr = GetWindowLongPtrW(self.hwnd, GWLP_USERDATA) as *mut FloatingChatState;
            if !state_ptr.is_null() {
                (*state_ptr).is_visible
            } else {
                false
            }
        }
        #[cfg(not(windows))]
        false
    }

    /// Toggles visibility and snaps near the pet position if showing.
    pub fn toggle_near_pet(&self, pet_pos: (i32, i32)) {
        #[cfg(windows)]
        unsafe {
            use windows_sys::Win32::UI::WindowsAndMessaging::{GetWindowLongPtrW, GWLP_USERDATA};
            if self.hwnd.is_null() {
                return;
            }
            let state_ptr = GetWindowLongPtrW(self.hwnd, GWLP_USERDATA) as *mut FloatingChatState;
            if !state_ptr.is_null() {
                let is_vis = (*state_ptr).is_visible;
                if is_vis {
                    self.set_visible(false);
                } else {
                    self.show_near_pet(pet_pos);
                }
            }
        }
    }

    /// Snaps and displays the chat window docked adjacent to the desktop pet.
    pub fn show_near_pet(&self, pet_pos: (i32, i32)) {
        #[cfg(windows)]
        unsafe {
            use windows_sys::Win32::UI::WindowsAndMessaging::{
                GetSystemMetrics, SetWindowPos, SM_CXSCREEN, SM_CYSCREEN, SWP_NOACTIVATE,
                SWP_NOSIZE, SWP_NOZORDER,
            };

            if self.hwnd.is_null() {
                return;
            }

            let screen_w = GetSystemMetrics(SM_CXSCREEN);
            let screen_h = GetSystemMetrics(SM_CYSCREEN);
            let (dock_x, dock_y) = compute_dock_position(
                pet_pos,
                screen_w,
                screen_h,
                CHAT_WINDOW_WIDTH,
                CHAT_WINDOW_HEIGHT,
            );

            SetWindowPos(
                self.hwnd,
                std::ptr::null_mut(),
                dock_x,
                dock_y,
                0,
                0,
                SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
            );

            self.set_visible(true);
        }
    }

    /// Appends a new chat message to history and requests a repaint.
    pub fn add_message(&self, sender: &str, text: &str, is_user: bool) {
        #[cfg(windows)]
        unsafe {
            use windows_sys::Win32::Graphics::Gdi::InvalidateRect;
            use windows_sys::Win32::UI::WindowsAndMessaging::{GetWindowLongPtrW, GWLP_USERDATA};
            if self.hwnd.is_null() {
                return;
            }
            let state_ptr = GetWindowLongPtrW(self.hwnd, GWLP_USERDATA) as *mut FloatingChatState;
            if !state_ptr.is_null() {
                let state = &mut *state_ptr;
                state.history.push(ChatMessageItem {
                    sender: sender.to_string(),
                    text: text.to_string(),
                    is_user,
                });
                InvalidateRect(self.hwnd, std::ptr::null(), 0);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_message_item_creation() {
        let item = ChatMessageItem {
            sender: "Mimi".to_string(),
            text: "Meow! Hello friend!".to_string(),
            is_user: false,
        };
        assert_eq!(item.sender, "Mimi");
        assert_eq!(item.text, "Meow! Hello friend!");
        assert!(!item.is_user);
    }

    #[test]
    fn test_compute_bubble_height_dynamic_allocation() {
        let available_height = 316;

        // 1 message: capped at maximum 75
        let h1 = compute_bubble_height(available_height, 1);
        assert_eq!(h1, 75);

        // 2 messages: 316 - 8 = 308 / 2 = 154 -> capped at 75
        let h2 = compute_bubble_height(available_height, 2);
        assert_eq!(h2, 75);

        // 3 messages: 316 - 16 = 300 / 3 = 100 -> capped at 75
        let h3 = compute_bubble_height(available_height, 3);
        assert_eq!(h3, 75);

        // 4 messages: 316 - 24 = 292 / 4 = 73 -> exactly 73
        let h4 = compute_bubble_height(available_height, 4);
        assert_eq!(h4, 73);

        // 5 messages: 316 - 32 = 284 / 5 = 56 -> clamped to minimum 60
        let h5 = compute_bubble_height(available_height, 5);
        assert_eq!(h5, 60);

        // Edge case: 0 display count safely handles divide by zero
        let h0 = compute_bubble_height(available_height, 0);
        assert_eq!(h0, 75);
    }

    #[test]
    fn test_compute_dock_position_boundaries() {
        let screen_w = 1920;
        let screen_h = 1080;
        let win_w = CHAT_WINDOW_WIDTH; // 340
        let win_h = CHAT_WINDOW_HEIGHT; // 420

        // 1. Center of screen: docks to the right (+130)
        let (x, y) = compute_dock_position((500, 500), screen_w, screen_h, win_w, win_h);
        assert_eq!(x, 630);
        assert_eq!(y, 400);

        // 2. Near right edge: flips to the left of the pet
        let (x_right, _) = compute_dock_position((1800, 500), screen_w, screen_h, win_w, win_h);
        assert_eq!(x_right, 1800 - win_w - 10);

        // 3. Near top edge: clamped to at least 20
        let (_, y_top) = compute_dock_position((500, 50), screen_w, screen_h, win_w, win_h);
        assert_eq!(y_top, 20);

        // 4. Near bottom edge: clamped to screen_h - win_h - 40 = 620
        let (_, y_bottom) = compute_dock_position((500, 950), screen_w, screen_h, win_w, win_h);
        assert_eq!(y_bottom, 620);

        // 5. Headless / Zero resolution edge case: no panic, clamped safely
        let (x_zero, y_zero) = compute_dock_position((100, 100), 0, 0, win_w, win_h);
        assert_eq!(x_zero, 10);
        assert_eq!(y_zero, 20);
    }
}
