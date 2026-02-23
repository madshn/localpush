//! macOS Keychain implementation with consolidated vault.
//!
//! All credentials are stored in a single Keychain entry as a JSON map,
//! so startup requires only ONE Keychain password prompt instead of N.
//! Pre-vault individual entries are migrated on first access (fallback read).

use std::collections::HashMap;
use std::sync::Mutex;
use keyring::Entry;
use crate::traits::{CredentialStore, CredentialError};

const SERVICE_NAME: &str = "com.localpush.app";
const VAULT_KEY: &str = "__vault__";

pub struct KeychainCredentialStore {
    cache: Mutex<HashMap<String, String>>,
}

impl KeychainCredentialStore {
    pub fn new() -> Self {
        let mut cache = HashMap::new();

        // Load consolidated vault from a single Keychain entry (one prompt)
        if let Ok(entry) = Entry::new(SERVICE_NAME, VAULT_KEY) {
            match entry.get_password() {
                Ok(json) => {
                    if let Ok(map) = serde_json::from_str::<HashMap<String, String>>(&json) {
                        tracing::info!(count = map.len(), "Loaded credential vault from Keychain");
                        cache = map;
                    }
                }
                Err(keyring::Error::NoEntry) => {
                    tracing::debug!("No credential vault yet — will migrate individual entries");
                }
                Err(e) => {
                    tracing::warn!(error = %e, "Failed to load credential vault");
                }
            }
        }

        Self {
            cache: Mutex::new(cache),
        }
    }

    /// Persist the in-memory cache to the single Keychain vault entry.
    fn save_vault(&self) -> Result<(), CredentialError> {
        let cache = self.cache.lock().unwrap();
        let json = serde_json::to_string(&*cache)
            .map_err(|e| CredentialError::StorageError(e.to_string()))?;

        let entry = Entry::new(SERVICE_NAME, VAULT_KEY)
            .map_err(|e| CredentialError::StorageError(e.to_string()))?;
        entry.set_password(&json)
            .map_err(|e| CredentialError::StorageError(e.to_string()))?;

        Ok(())
    }
}

impl Default for KeychainCredentialStore {
    fn default() -> Self {
        Self::new()
    }
}

impl CredentialStore for KeychainCredentialStore {
    fn store(&self, key: &str, value: &str) -> Result<(), CredentialError> {
        {
            let mut cache = self.cache.lock().unwrap();
            cache.insert(key.to_string(), value.to_string());
        }
        self.save_vault()?;
        tracing::debug!("Stored credential: {}", key);
        Ok(())
    }

    fn retrieve(&self, key: &str) -> Result<Option<String>, CredentialError> {
        // Check vault cache first (no Keychain prompt)
        {
            let cache = self.cache.lock().unwrap();
            if let Some(value) = cache.get(key) {
                return Ok(Some(value.clone()));
            }
        }

        // Fallback: try pre-vault individual Keychain entry (may prompt)
        let entry = Entry::new(SERVICE_NAME, key)
            .map_err(|e| CredentialError::StorageError(e.to_string()))?;

        match entry.get_password() {
            Ok(value) => {
                // Migrate to vault cache (flushed by flush_vault() after startup)
                let mut cache = self.cache.lock().unwrap();
                cache.insert(key.to_string(), value.clone());
                tracing::info!(key = %key, "Migrated credential to vault");
                Ok(Some(value))
            }
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(CredentialError::StorageError(e.to_string())),
        }
    }

    fn delete(&self, key: &str) -> Result<bool, CredentialError> {
        let existed = {
            let mut cache = self.cache.lock().unwrap();
            cache.remove(key).is_some()
        };
        if existed {
            self.save_vault()?;
        }
        // Also remove any pre-vault individual entry
        if let Ok(entry) = Entry::new(SERVICE_NAME, key) {
            let _ = entry.delete_credential();
        }
        tracing::debug!("Deleted credential: {}", key);
        Ok(existed)
    }

    fn exists(&self, key: &str) -> Result<bool, CredentialError> {
        {
            let cache = self.cache.lock().unwrap();
            if cache.contains_key(key) {
                return Ok(true);
            }
        }
        // Fallback: check pre-vault individual entry
        let entry = Entry::new(SERVICE_NAME, key)
            .map_err(|e| CredentialError::StorageError(e.to_string()))?;
        match entry.get_password() {
            Ok(_) => Ok(true),
            Err(keyring::Error::NoEntry) => Ok(false),
            Err(e) => Err(CredentialError::StorageError(e.to_string())),
        }
    }

    fn flush_vault(&self) {
        match self.save_vault() {
            Ok(()) => {
                let count = self.cache.lock().unwrap().len();
                tracing::info!(count = count, "Flushed credential vault to Keychain");
            }
            Err(e) => {
                tracing::warn!(error = %e, "Failed to flush credential vault");
            }
        }
    }
}
