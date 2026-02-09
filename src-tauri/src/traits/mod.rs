//! Trait definitions for dependency injection
//!
//! All external dependencies are abstracted behind traits to enable testing.

use std::sync::{Arc, Mutex};

mod credential_store;
mod file_watcher;
mod webhook_client;
mod delivery_ledger;
mod target;

pub use credential_store::{CredentialStore, CredentialError};
pub use file_watcher::{FileWatcher, FileWatcherError, FileEvent, FileEventKind};
pub use webhook_client::{WebhookClient, WebhookError, WebhookResponse, WebhookAuth};
pub use delivery_ledger::{DeliveryLedgerTrait, DeliveryEntry, DeliveryStatus, LedgerError, LedgerStats};
pub use target::{Target, TargetError, TargetInfo, TargetEndpoint};

/// Shared event handler type used by file watchers
pub type EventHandler = Arc<Mutex<Option<Arc<dyn Fn(FileEvent) + Send + Sync>>>>;
