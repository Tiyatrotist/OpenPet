use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SingletonError {
    #[error("Another instance of OpenPet is already running")]
    AlreadyRunning,
    #[error("Operating system error while acquiring singleton mutex: {0}")]
    OsError(u32),
}

/// Guards against multiple concurrent instances of the OpenPet host running simultaneously.
pub struct SingleInstanceGuard {
    #[cfg(windows)]
    handle: windows_sys::Win32::Foundation::HANDLE,
}

impl SingleInstanceGuard {
    /// Attempts to acquire the global user-scoped single instance named mutex.
    pub fn acquire(name: &str) -> Result<Self, SingletonError> {
        #[cfg(windows)]
        {
            use windows_sys::Win32::Foundation::{
                GetLastError, ERROR_ALREADY_EXISTS, INVALID_HANDLE_VALUE,
            };
            use windows_sys::Win32::System::Threading::CreateMutexW;

            let wide_name: Vec<u16> = OsStr::new(name)
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();

            // SAFETY:
            // Calling CreateMutexW with valid null-terminated wide string buffer.
            // Returned HANDLE is checked against NULL and INVALID_HANDLE_VALUE before use.
            let handle = unsafe { CreateMutexW(std::ptr::null(), 1, wide_name.as_ptr()) };

            if handle.is_null() || handle == INVALID_HANDLE_VALUE {
                let err = unsafe { GetLastError() };
                return Err(SingletonError::OsError(err));
            }

            let last_error = unsafe { GetLastError() };
            if last_error == ERROR_ALREADY_EXISTS {
                unsafe { windows_sys::Win32::Foundation::CloseHandle(handle) };
                return Err(SingletonError::AlreadyRunning);
            }

            Ok(Self { handle })
        }

        #[cfg(not(windows))]
        {
            let _ = name;
            Ok(Self {})
        }
    }
}

impl Drop for SingleInstanceGuard {
    fn drop(&mut self) {
        #[cfg(windows)]
        {
            if !self.handle.is_null()
                && self.handle != windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE
            {
                // SAFETY:
                // CloseHandle is invoked on the acquired mutex handle to release system resource.
                unsafe {
                    windows_sys::Win32::Foundation::CloseHandle(self.handle);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_instance_acquisition() {
        let unique_name = format!("OpenPet_Test_Mutex_{}", uuid::Uuid::new_v4());
        let guard1 =
            SingleInstanceGuard::acquire(&unique_name).expect("First acquire must succeed");

        // Second acquisition of the exact same mutex should report AlreadyRunning
        let guard2 = SingleInstanceGuard::acquire(&unique_name);
        assert!(matches!(guard2, Err(SingletonError::AlreadyRunning)));

        drop(guard1);

        // After drop, acquiring again should succeed
        let guard3 = SingleInstanceGuard::acquire(&unique_name);
        assert!(guard3.is_ok());
    }
}
