//! Application state management

use std::sync::Arc;
use tauri::{AppHandle, Manager};

use crate::config::AppConfig;
use crate::source_manager::SourceManager;
use crate::target_manager::TargetManager;
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
    pub target_manager: Arc<TargetManager>,
    pub config: Arc<AppConfig>,
}

impl AppState {
    /// Create a new AppState with production implementations
    pub fn new_production(app: &AppHandle) -> Result<Self, Box<dyn std::error::Error>> {
        tracing::info!("Initializing AppState");

        let app_data_dir = app.path().app_data_dir()?;
        std::fs::create_dir_all(&app_data_dir)?;

        let db_path = app_data_dir.join("ledger.sqlite");
        tracing::info!(path = %db_path.display(), "Opening delivery ledger");
        let ledger = Arc::new(DeliveryLedger::open(&db_path)?);

        let config_path = app_data_dir.join("config.sqlite");
        tracing::info!(path = %config_path.display(), "Opening config database");
        let config_conn = rusqlite::Connection::open(&config_path)?;
        AppConfig::init_table(&config_conn)?;
        let config = Arc::new(AppConfig::from_connection(config_conn));

        // Set default webhook if not configured
        if config.get("webhook_url").ok().flatten().is_none() {
            tracing::info!("Setting default webhook URL");
            let _ = config.set("webhook_url", "https://flow.rightaim.ai/webhook/localpush-ingest");
            let _ = config.set("webhook_auth_json", r#"{"type":"none"}"#);
        }

        tracing::info!("Keychain credential store initialized");
        let credentials = Arc::new(KeychainCredentialStore::new());

        tracing::info!("FSEvents file watcher initialized");
        let file_watcher = Arc::new(FsEventsWatcher::new()?);

        tracing::info!("Webhook client initialized");
        let webhook_client = Arc::new(ReqwestWebhookClient::new()?);

        let source_manager = Arc::new(SourceManager::new(
            ledger.clone(),
            file_watcher.clone(),
            config.clone(),
        ));

        // Initialize target manager
        let target_manager = Arc::new(TargetManager::new(config.clone()));

        // Register sources
        use crate::sources::{ClaudeStatsSource, ClaudeSessionsSource, ApplePodcastsSource, AppleNotesSource, ApplePhotosSource};

        match ClaudeStatsSource::new() {
            Ok(source) => {
                tracing::info!("Registered ClaudeStatsSource");
                source_manager.register(Arc::new(source));
            }
            Err(e) => tracing::warn!("Could not initialize Claude stats source: {}", e),
        }

        // Register Claude Sessions source
        match ClaudeSessionsSource::new() {
            Ok(source) => {
                tracing::info!("Registered ClaudeSessionsSource");
                source_manager.register(Arc::new(source));
            }
            Err(e) => tracing::warn!("Could not initialize Claude sessions source: {}", e),
        }

        // Register Apple sources (graceful — may fail due to permissions)
        match ApplePodcastsSource::new() {
            Ok(source) => {
                tracing::info!("Registered ApplePodcastsSource");
                source_manager.register(Arc::new(source));
            }
            Err(e) => tracing::warn!("Apple Podcasts source unavailable: {}", e),
        }
        match AppleNotesSource::new() {
            Ok(source) => {
                tracing::info!("Registered AppleNotesSource");
                source_manager.register(Arc::new(source));
            }
            Err(e) => tracing::warn!("Apple Notes source unavailable: {}", e),
        }
        match ApplePhotosSource::new() {
            Ok(source) => {
                tracing::info!("Registered ApplePhotosSource");
                source_manager.register(Arc::new(source));
            }
            Err(e) => tracing::warn!("Apple Photos source unavailable: {}", e),
        }

        // Restore enabled sources from config
        let restored = source_manager.restore_enabled();
        tracing::info!(restored_count = restored.len(), "Restored enabled sources");

        // Auto-enable Claude stats on first launch
        if restored.is_empty() && config.get("source.claude-stats.enabled").ok().flatten().is_none() {
            tracing::info!("First launch: auto-enabling Claude Code stats source");
            let _ = source_manager.enable("claude-stats");
        }

        tracing::info!("AppState initialization complete");

        Ok(Self {
            credentials,
            file_watcher,
            webhook_client,
            ledger,
            source_manager,
            target_manager,
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

        let target_manager = Arc::new(TargetManager::new(config.clone()));

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
            target_manager,
            config,
        }
    }
}
