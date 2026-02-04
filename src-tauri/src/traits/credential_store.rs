//! Credential storage trait for secure secret management

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CredentialError {
    #[error("Credential not found")]
    NotFound,
    #[error("Access denied")]
    AccessDenied,
    #[error("Storage error: {0}")]
    StorageError(String),
}

/// Trait for secure credential storage
///
/// Production: macOS Keychain via `keyring` crate
/// Testing: In-memory HashMap
#[cfg_attr(test, mockall::automock)]
pub trait CredentialStore: Send + Sync {
    /// Store a credential
    fn store(&self, key: &str, value: &str) -> Result<(), CredentialError>;

    /// Retrieve a credential
    fn retrieve(&self, key: &str) -> Result<Option<String>, CredentialError>;

    /// Delete a credential
    fn delete(&self, key: &str) -> Result<bool, CredentialError>;

    /// Check if a credential exists
    fn exists(&self, key: &str) -> Result<bool, CredentialError>;
}
