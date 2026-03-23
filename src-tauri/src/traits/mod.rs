//! Trait definitions for dependency injection
//!
//! All external dependencies are abstracted behind traits to enable testing.

use std::sync::{Arc, Mutex};

mod credential_store;
mod delivery_ledger;
mod file_watcher;
mod target;
mod webhook_client;

pub use credential_store::{CredentialError, CredentialStore};
pub use delivery_ledger::{
    DeliveryEntry, DeliveryLedgerTrait, DeliveryStatus, LedgerError, LedgerStats, SourceStatusCount,
};
pub use file_watcher::{FileEvent, FileEventKind, FileWatcher, FileWatcherError};
pub use target::{Target, TargetEndpoint, TargetError, TargetInfo};
pub use webhook_client::{WebhookAuth, WebhookClient, WebhookError, WebhookResponse};

/// Shared event handler type used by file watchers
pub type EventHandler = Arc<Mutex<Option<Arc<dyn Fn(FileEvent) + Send + Sync>>>>;
