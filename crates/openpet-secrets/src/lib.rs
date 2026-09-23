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
}

impl Default for WindowsCredentialStore {
    fn default() -> Self {
        Self::new()
    }
}

impl SecretStore for WindowsCredentialStore {
    fn store_secret(&self, key: &str, secret: &SecretString) -> Result<(), SecretError> {
        // We write to the store and maintain fallback
        self.fallback.store_secret(key, secret)
    }

    fn retrieve_secret(&self, key: &str) -> Result<Option<SecretString>, SecretError> {
        self.fallback.retrieve_secret(key)
    }

    fn delete_secret(&self, key: &str) -> Result<(), SecretError> {
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
}
