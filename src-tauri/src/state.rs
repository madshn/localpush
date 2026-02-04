//! Application state management

use std::sync::Arc;
use tauri::AppHandle;

use crate::traits::{CredentialStore, FileWatcher, WebhookClient, DeliveryLedgerTrait};
use crate::production::{KeychainCredentialStore, FsEventsWatcher, ReqwestWebhookClient};
use crate::ledger::DeliveryLedger;

/// Application state containing all dependencies
pub struct AppState {
    pub credentials: Arc<dyn CredentialStore>,
    pub file_watcher: Arc<dyn FileWatcher>,
    pub webhook_client: Arc<dyn WebhookClient>,
    pub ledger: Arc<dyn DeliveryLedgerTrait>,
}

impl AppState {
    /// Create a new AppState with production implementations
    pub fn new_production(app: &AppHandle) -> Result<Self, Box<dyn std::error::Error>> {
        let app_data_dir = app.path().app_data_dir()?;
        std::fs::create_dir_all(&app_data_dir)?;

        let db_path = app_data_dir.join("ledger.sqlite");
        let ledger = DeliveryLedger::open(&db_path)?;

        Ok(Self {
            credentials: Arc::new(KeychainCredentialStore::new()),
            file_watcher: Arc::new(FsEventsWatcher::new()?),
            webhook_client: Arc::new(ReqwestWebhookClient::new()),
            ledger: Arc::new(ledger),
        })
    }

    /// Create a new AppState with test implementations
    #[cfg(test)]
    pub fn new_test() -> Self {
        use crate::mocks::{MockCredentialStore, MockFileWatcher, MockWebhookClient, InMemoryLedger};

        Self {
            credentials: Arc::new(MockCredentialStore::new()),
            file_watcher: Arc::new(MockFileWatcher::new()),
            webhook_client: Arc::new(MockWebhookClient::new()),
            ledger: Arc::new(InMemoryLedger::new()),
        }
    }
}
