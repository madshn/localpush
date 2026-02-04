//! Application state management

use std::sync::Arc;
use tauri::{AppHandle, Manager};

use crate::config::AppConfig;
use crate::source_manager::SourceManager;
use crate::traits::{CredentialStore, FileWatcher, WebhookClient, DeliveryLedgerTrait};
use crate::production::{KeychainCredentialStore, FsEventsWatcher, ReqwestWebhookClient};
use crate::ledger::DeliveryLedger;

/// Application state containing all dependencies
pub struct AppState {
    pub credentials: Arc<dyn CredentialStore>,
    pub file_watcher: Arc<dyn FileWatcher>,
    pub webhook_client: Arc<dyn WebhookClient>,
    pub ledger: Arc<dyn DeliveryLedgerTrait>,
    pub source_manager: Arc<SourceManager>,
    pub config: Arc<AppConfig>,
}

impl AppState {
    /// Create a new AppState with production implementations
    pub fn new_production(app: &AppHandle) -> Result<Self, Box<dyn std::error::Error>> {
        let app_data_dir = app.path().app_data_dir()?;
        std::fs::create_dir_all(&app_data_dir)?;

        let db_path = app_data_dir.join("ledger.sqlite");
        let ledger = Arc::new(DeliveryLedger::open(&db_path)?);

        let config_path = app_data_dir.join("config.sqlite");
        let config_conn = rusqlite::Connection::open(&config_path)?;
        AppConfig::init_table(&config_conn)?;
        let config = Arc::new(AppConfig::from_connection(config_conn));

        let credentials = Arc::new(KeychainCredentialStore::new());
        let file_watcher = Arc::new(FsEventsWatcher::new()?);
        let webhook_client = Arc::new(ReqwestWebhookClient::new()?);

        let source_manager = Arc::new(SourceManager::new(
            ledger.clone(),
            file_watcher.clone(),
            config.clone(),
        ));

        // Register ClaudeStatsSource
        use crate::sources::ClaudeStatsSource;
        match ClaudeStatsSource::new() {
            Ok(source) => source_manager.register(Arc::new(source)),
            Err(e) => tracing::warn!("Could not initialize Claude stats source: {}", e),
        }

        // Restore enabled sources from config
        source_manager.restore_enabled();

        Ok(Self {
            credentials,
            file_watcher,
            webhook_client,
            ledger,
            source_manager,
            config,
        })
    }

    /// Create a new AppState with test implementations
    #[cfg(test)]
    pub fn new_test() -> Self {
        use crate::mocks::{InMemoryCredentialStore, ManualFileWatcher, RecordedWebhookClient};
        use crate::DeliveryLedger;
        use crate::sources::ClaudeStatsSource;

        let credentials = Arc::new(InMemoryCredentialStore::new());
        let file_watcher = Arc::new(ManualFileWatcher::new());
        let webhook_client = Arc::new(RecordedWebhookClient::new());
        let ledger = Arc::new(DeliveryLedger::open_in_memory().unwrap());
        let config = Arc::new(AppConfig::open_in_memory().unwrap());

        let source_manager = Arc::new(SourceManager::new(
            ledger.clone(),
            file_watcher.clone(),
            config.clone(),
        ));

        // Register test source
        match ClaudeStatsSource::new() {
            Ok(source) => source_manager.register(Arc::new(source)),
            Err(_) => {
                // In tests, use a custom path
                source_manager.register(Arc::new(ClaudeStatsSource::new_with_path("/tmp/fake-stats.json")))
            }
        }

        Self {
            credentials,
            file_watcher,
            webhook_client,
            ledger,
            source_manager,
            config,
        }
    }
}
