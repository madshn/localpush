//! macOS Keychain implementation

use keyring::Entry;
use crate::traits::{CredentialStore, CredentialError};

const SERVICE_NAME: &str = "com.localpush.app";

pub struct KeychainCredentialStore;

impl KeychainCredentialStore {
    pub fn new() -> Self {
        Self
    }
}

impl CredentialStore for KeychainCredentialStore {
    fn store(&self, key: &str, value: &str) -> Result<(), CredentialError> {
        let entry = Entry::new(SERVICE_NAME, key)
            .map_err(|e| CredentialError::StorageError(e.to_string()))?;

        entry.set_password(value)
            .map_err(|e| CredentialError::StorageError(e.to_string()))?;

        tracing::debug!("Stored credential: {}", key);
        Ok(())
    }

    fn retrieve(&self, key: &str) -> Result<Option<String>, CredentialError> {
        let entry = Entry::new(SERVICE_NAME, key)
            .map_err(|e| CredentialError::StorageError(e.to_string()))?;

        match entry.get_password() {
            Ok(value) => Ok(Some(value)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(CredentialError::StorageError(e.to_string())),
        }
    }

    fn delete(&self, key: &str) -> Result<bool, CredentialError> {
        let entry = Entry::new(SERVICE_NAME, key)
            .map_err(|e| CredentialError::StorageError(e.to_string()))?;

        match entry.delete_credential() {
            Ok(()) => {
                tracing::debug!("Deleted credential: {}", key);
                Ok(true)
            }
            Err(keyring::Error::NoEntry) => Ok(false),
            Err(e) => Err(CredentialError::StorageError(e.to_string())),
        }
    }

    fn exists(&self, key: &str) -> Result<bool, CredentialError> {
        let entry = Entry::new(SERVICE_NAME, key)
            .map_err(|e| CredentialError::StorageError(e.to_string()))?;

        match entry.get_password() {
            Ok(_) => Ok(true),
            Err(keyring::Error::NoEntry) => Ok(false),
            Err(e) => Err(CredentialError::StorageError(e.to_string())),
        }
    }
}
