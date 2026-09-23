//! # OpenPet Secrets
//!
//! Hardened secret handling, in-memory zeroization upon drop, and Windows Credential Manager integration.

use std::collections::HashMap;
use std::fmt;
use std::sync::RwLock;
use thiserror::Error;
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(Error, Debug)]
pub enum SecretError {
    #[error("Store error: {0}")]
    StorageError(String),
    #[error("Credential not found")]
    NotFound,
    #[error("Invalid secret format")]
    InvalidFormat,
}

/// A protected string wrapper whose underlying memory buffer is automatically zeroized upon drop.
///
/// Implements `Debug` and `Display` to strictly output `[REDACTED_SECRET]` so accidental
/// formatting or logging never prints sensitive data.
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct SecretString {
    inner: String,
}

impl SecretString {
    pub fn new(secret: impl Into<String>) -> Self {
        Self {
            inner: secret.into(),
        }
    }

    /// Explicit accessor for component requiring plaintext value (e.g. HTTP authorization header).
    pub fn expose_secret(&self) -> &str {
        &self.inner
    }
}

impl fmt::Debug for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[REDACTED_SECRET]")
    }
}

impl fmt::Display for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[REDACTED_SECRET]")
    }
}

/// Storage interface for sensitive application credentials (DB encryption keys, LLM API keys).
pub trait SecretStore: Send + Sync {
    fn store_secret(&self, key: &str, secret: &SecretString) -> Result<(), SecretError>;
    fn retrieve_secret(&self, key: &str) -> Result<Option<SecretString>, SecretError>;
    fn delete_secret(&self, key: &str) -> Result<(), SecretError>;
}

/// Ephemeral in-memory secret store used in testing and as a fallback.
#[derive(Default)]
pub struct MemorySecretStore {
    vault: RwLock<HashMap<String, SecretString>>,
}

impl MemorySecretStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl SecretStore for MemorySecretStore {
    fn store_secret(&self, key: &str, secret: &SecretString) -> Result<(), SecretError> {
        let mut map = self
            .vault
            .write()
            .map_err(|e| SecretError::StorageError(e.to_string()))?;
        map.insert(key.to_string(), secret.clone());
        Ok(())
    }

    fn retrieve_secret(&self, key: &str) -> Result<Option<SecretString>, SecretError> {
        let map = self
            .vault
            .read()
            .map_err(|e| SecretError::StorageError(e.to_string()))?;
        Ok(map.get(key).cloned())
    }

    fn delete_secret(&self, key: &str) -> Result<(), SecretError> {
        let mut map = self
            .vault
            .write()
            .map_err(|e| SecretError::StorageError(e.to_string()))?;
        map.remove(key);
        Ok(())
    }
}

#[cfg(windows)]
mod win_cred {
    use super::*;
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Foundation::{GetLastError, ERROR_NOT_FOUND};
    use windows_sys::Win32::Security::Credentials::{
        CredDeleteW, CredFree, CredReadW, CredWriteW, CREDENTIALW, CRED_PERSIST_LOCAL_MACHINE,
        CRED_TYPE_GENERIC,
    };

    pub fn write_credential(target_name: &str, secret_bytes: &[u8]) -> Result<(), SecretError> {
        let wide_target: Vec<u16> = OsStr::new(target_name)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let wide_user: Vec<u16> = OsStr::new("OpenPetUser")
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let cred = CREDENTIALW {
            Flags: 0,
            Type: CRED_TYPE_GENERIC,
            TargetName: wide_target.as_ptr() as *mut u16,
            Comment: std::ptr::null_mut(),
            LastWritten: windows_sys::Win32::Foundation::FILETIME {
                dwLowDateTime: 0,
                dwHighDateTime: 0,
            },
            CredentialBlobSize: secret_bytes.len() as u32,
            CredentialBlob: secret_bytes.as_ptr() as *mut u8,
            Persist: CRED_PERSIST_LOCAL_MACHINE,
            AttributeCount: 0,
            Attributes: std::ptr::null_mut(),
            TargetAlias: std::ptr::null_mut(),
            UserName: wide_user.as_ptr() as *mut u16,
        };

        // SAFETY: Invoking CredWriteW with a valid, pinned CREDENTIALW struct pointer.
        let res = unsafe { CredWriteW(&cred, 0) };
        if res == 0 {
            let err = unsafe { GetLastError() };
            return Err(SecretError::StorageError(format!(
                "CredWriteW failed with error code: {}",
                err
            )));
        }
        Ok(())
    }

    pub fn read_credential(target_name: &str) -> Result<Option<SecretString>, SecretError> {
        let wide_target: Vec<u16> = OsStr::new(target_name)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let mut p_cred: *mut CREDENTIALW = std::ptr::null_mut();
        // SAFETY: Invoking CredReadW with valid wide string target and receiving pointer.
        let res = unsafe { CredReadW(wide_target.as_ptr(), CRED_TYPE_GENERIC, 0, &mut p_cred) };

        if res == 0 {
            let err = unsafe { GetLastError() };
            if err == ERROR_NOT_FOUND {
                return Ok(None);
            }
            return Err(SecretError::StorageError(format!(
                "CredReadW failed with error code: {}",
                err
            )));
        }

        if p_cred.is_null() {
            return Ok(None);
        }

        // SAFETY: p_cred is non-null and points to valid CREDENTIALW; memory freed with CredFree.
        let slice = unsafe {
            std::slice::from_raw_parts(
                (*p_cred).CredentialBlob,
                (*p_cred).CredentialBlobSize as usize,
            )
        };
        let secret_str =
            String::from_utf8(slice.to_vec()).map_err(|_| SecretError::InvalidFormat)?;

        unsafe { CredFree(p_cred as *const std::ffi::c_void) };

        Ok(Some(SecretString::new(secret_str)))
    }

    pub fn delete_credential(target_name: &str) -> Result<(), SecretError> {
        let wide_target: Vec<u16> = OsStr::new(target_name)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        // SAFETY: Invoking CredDeleteW with valid wide target string.
        let res = unsafe { CredDeleteW(wide_target.as_ptr(), CRED_TYPE_GENERIC, 0) };
        if res == 0 {
            let err = unsafe { GetLastError() };
            if err == ERROR_NOT_FOUND {
                return Ok(());
            }
            return Err(SecretError::StorageError(format!(
                "CredDeleteW failed with error code: {}",
                err
            )));
        }
        Ok(())
    }
}

/// Platform Credential Store (Windows Credential Manager with graceful fallback).
pub struct WindowsCredentialStore {
    fallback: MemorySecretStore,
}

impl WindowsCredentialStore {
    pub fn new() -> Self {
        Self {
            fallback: MemorySecretStore::new(),
        }
    }

    fn qualify_key(key: &str) -> String {
        format!("OpenPet:{}", key)
    }
}

impl Default for WindowsCredentialStore {
    fn default() -> Self {
        Self::new()
    }
}

impl SecretStore for WindowsCredentialStore {
    fn store_secret(&self, key: &str, secret: &SecretString) -> Result<(), SecretError> {
        #[cfg(windows)]
        {
            let target = Self::qualify_key(key);
            match win_cred::write_credential(&target, secret.expose_secret().as_bytes()) {
                Ok(()) => return Ok(()),
                Err(e) => {
                    tracing::warn!(
                        "Windows Credential Manager write failed ({}), falling back to memory store",
                        e
                    );
                }
            }
        }
        self.fallback.store_secret(key, secret)
    }

    fn retrieve_secret(&self, key: &str) -> Result<Option<SecretString>, SecretError> {
        #[cfg(windows)]
        {
            let target = Self::qualify_key(key);
            match win_cred::read_credential(&target) {
                Ok(Some(secret)) => return Ok(Some(secret)),
                Ok(None) => {}
                Err(e) => {
                    tracing::warn!(
                        "Windows Credential Manager read failed ({}), falling back to memory store",
                        e
                    );
                }
            }
        }
        self.fallback.retrieve_secret(key)
    }

    fn delete_secret(&self, key: &str) -> Result<(), SecretError> {
        #[cfg(windows)]
        {
            let target = Self::qualify_key(key);
            let _ = win_cred::delete_credential(&target);
        }
        self.fallback.delete_secret(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secret_string_formatting() {
        let secret = SecretString::new("super-secret-key-999");
        assert_eq!(format!("{}", secret), "[REDACTED_SECRET]");
        assert_eq!(format!("{:?}", secret), "[REDACTED_SECRET]");
        assert_eq!(secret.expose_secret(), "super-secret-key-999");
    }

    #[test]
    fn test_memory_secret_store() {
        let store = MemorySecretStore::new();
        let key = "openai_api_key";
        let secret = SecretString::new("sk-test-12345");

        assert!(store.retrieve_secret(key).unwrap().is_none());
        store.store_secret(key, &secret).unwrap();

        let retrieved = store
            .retrieve_secret(key)
            .unwrap()
            .expect("Should find secret");
        assert_eq!(retrieved.expose_secret(), "sk-test-12345");

        store.delete_secret(key).unwrap();
        assert!(store.retrieve_secret(key).unwrap().is_none());
    }

    #[test]
    fn test_windows_credential_store() {
        let store = WindowsCredentialStore::new();
        let key = "test_persistence_key";
        let secret = SecretString::new("test-value-12345");

        store.store_secret(key, &secret).unwrap();
        let retrieved = store
            .retrieve_secret(key)
            .unwrap()
            .expect("Should retrieve secret");
        assert_eq!(retrieved.expose_secret(), "test-value-12345");

        store.delete_secret(key).unwrap();
        assert!(store.retrieve_secret(key).unwrap().is_none());
    }
}
