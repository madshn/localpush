//! Trait definitions for dependency injection
//!
//! All external dependencies are abstracted behind traits to enable testing.

mod credential_store;
mod file_watcher;
mod webhook_client;
mod delivery_ledger;

pub use credential_store::{CredentialStore, CredentialError};
pub use file_watcher::{FileWatcher, FileWatcherError, FileEvent};
pub use webhook_client::{WebhookClient, WebhookError, WebhookResponse, WebhookAuth};
pub use delivery_ledger::{DeliveryLedgerTrait, DeliveryEntry, DeliveryStatus};
