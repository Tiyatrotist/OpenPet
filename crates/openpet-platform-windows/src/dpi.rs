//! # Windows DPI Awareness
//!
//! Configures Per-Monitor V2 DPI awareness to ensure crystal-clear sprite rendering
//! across multi-monitor setups with mixed scaling (100% to 250%).

pub fn enable_per_monitor_dpi_v2() -> bool {
    #[cfg(windows)]
    {
        type SetProcessDpiAwarenessContextFn = unsafe extern "system" fn(isize) -> i32;

        // SAFETY:
        // Dynamically resolving SetProcessDpiAwarenessContext from user32.dll with valid null-terminated strings.
        // If the entrypoint is present (Windows 10 1703+), invokes it with DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2 (-4).
        unsafe {
            use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleA, GetProcAddress};
            let user32 = GetModuleHandleA(c"user32.dll".as_ptr() as *const u8);
            if !user32.is_null() {
                let func = GetProcAddress(
                    user32,
                    c"SetProcessDpiAwarenessContext".as_ptr() as *const u8,
                );
                if let Some(set_dpi) = func {
                    let set_dpi_fn: SetProcessDpiAwarenessContextFn = std::mem::transmute(set_dpi);
                    return set_dpi_fn(-4) != 0;
                }
            }
            false
        }
    }

    #[cfg(not(windows))]
    {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dpi_call_does_not_panic() {
        let _ = enable_per_monitor_dpi_v2();
    }
}
