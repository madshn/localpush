//! File-based credential store for development builds
//!
//! Avoids macOS Keychain prompts during development. The binary changes every
//! compile in dev mode, so macOS prompts for password on every keychain access.
//! This stores credentials in a plain JSON file instead.
//!
//! WARNING: Not secure. Only used when `debug_assertions` is enabled.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use crate::traits::{CredentialError, CredentialStore};

pub struct DevFileCredentialStore {
    path: PathBuf,
    cache: Mutex<HashMap<String, String>>,
}

impl DevFileCredentialStore {
    pub fn new(path: PathBuf) -> Self {
        let cache = if path.exists() {
            let content = std::fs::read_to_string(&path).unwrap_or_default();
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            HashMap::new()
        };
        Self {
            path,
            cache: Mutex::new(cache),
        }
    }

    fn flush(&self) -> Result<(), CredentialError> {
        let cache = self.cache.lock().unwrap();
        let content = serde_json::to_string_pretty(&*cache)
            .map_err(|e| CredentialError::StorageError(e.to_string()))?;
        std::fs::write(&self.path, content)
            .map_err(|e| CredentialError::StorageError(e.to_string()))?;
        Ok(())
    }
}

impl CredentialStore for DevFileCredentialStore {
    fn store(&self, key: &str, value: &str) -> Result<(), CredentialError> {
        self.cache
            .lock()
            .unwrap()
            .insert(key.to_string(), value.to_string());
        self.flush()
    }

    fn retrieve(&self, key: &str) -> Result<Option<String>, CredentialError> {
        Ok(self.cache.lock().unwrap().get(key).cloned())
    }

    fn delete(&self, key: &str) -> Result<bool, CredentialError> {
        let removed = self.cache.lock().unwrap().remove(key).is_some();
        if removed {
            self.flush()?;
        }
        Ok(removed)
    }

    fn exists(&self, key: &str) -> Result<bool, CredentialError> {
        Ok(self.cache.lock().unwrap().contains_key(key))
    }
}
