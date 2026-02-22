//! Application state management

use std::sync::Arc;
use tauri::{AppHandle, Manager};

use crate::bindings::BindingStore;
use crate::config::AppConfig;
use crate::source_manager::SourceManager;
use crate::target_health::TargetHealthTracker;
use crate::target_manager::TargetManager;
use crate::traits::{CredentialStore, FileWatcher, WebhookClient, DeliveryLedgerTrait};
#[cfg(not(debug_assertions))]
use crate::production::KeychainCredentialStore;
use crate::production::{FsEventsWatcher, ReqwestWebhookClient};
use crate::ledger::DeliveryLedger;

/// Application state containing all dependencies
pub struct AppState {
    pub credentials: Arc<dyn CredentialStore>,
    pub file_watcher: Arc<dyn FileWatcher>,
    pub webhook_client: Arc<dyn WebhookClient>,
    pub ledger: Arc<dyn DeliveryLedgerTrait>,
    pub source_manager: Arc<SourceManager>,
    pub target_manager: Arc<TargetManager>,
    pub binding_store: Arc<BindingStore>,
    pub config: Arc<AppConfig>,
    pub health_tracker: Arc<TargetHealthTracker>,
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

        #[cfg(debug_assertions)]
        let credentials: Arc<dyn CredentialStore> = {
            let cred_path = app_data_dir.join("dev-credentials.json");
            tracing::info!(path = %cred_path.display(), "DEV MODE: file-based credential store (no Keychain prompts)");
            Arc::new(crate::production::DevFileCredentialStore::new(cred_path))
        };
        #[cfg(not(debug_assertions))]
        let credentials: Arc<dyn CredentialStore> = {
            tracing::info!("Keychain credential store initialized");
            Arc::new(KeychainCredentialStore::new())
        };

        tracing::info!("FSEvents file watcher initialized");
        let file_watcher = Arc::new(FsEventsWatcher::new()?);

        tracing::info!("Webhook client initialized");
        let webhook_client = Arc::new(ReqwestWebhookClient::new()?);

        // Initialize target manager, binding store, and health tracker
        let target_manager = Arc::new(TargetManager::new(config.clone()));
        let binding_store = Arc::new(BindingStore::new(config.clone()));

        let source_manager = Arc::new(SourceManager::new(
            ledger.clone(),
            file_watcher.clone(),
            config.clone(),
            binding_store.clone(),
        ));
        let health_tracker = Arc::new(TargetHealthTracker::new());

        // Restore persisted targets from config
        let target_entries = config.get_by_prefix("target.").unwrap_or_default();
        let mut target_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
        for (key, _) in &target_entries {
            // Keys are like "target.n8n-abc123.url" — extract the target ID
            let parts: Vec<&str> = key.splitn(3, '.').collect();
            if parts.len() >= 2 {
                target_ids.insert(parts[1].to_string());
            }
        }
        for tid in &target_ids {
            let target_type = config.get(&format!("target.{}.type", tid)).ok().flatten();
            let target_url = config.get(&format!("target.{}.url", tid)).ok().flatten();
            if let (Some(ttype), Some(url)) = (target_type, target_url) {
                match ttype.as_str() {
                    "n8n" => {
                        let cred_key = format!("n8n:{}", tid);
                        let cred_result = credentials.retrieve(&cred_key);
                        tracing::debug!(target_id = %tid, cred_key = %cred_key, result = ?cred_result, "n8n credential lookup");
                        match cred_result {
                            Ok(Some(api_key)) if !api_key.is_empty() => {
                                let target = crate::targets::N8nTarget::new(tid.clone(), url, api_key);
                                target_manager.register(Arc::new(target));
                                tracing::info!(target_id = %tid, "Restored n8n target");
                            }
                            Ok(Some(_)) => tracing::warn!(target_id = %tid, "n8n API key is empty in keychain"),
                            Ok(None) => tracing::warn!(target_id = %tid, "n8n API key not found in keychain — target skipped"),
                            Err(e) => tracing::warn!(target_id = %tid, error = %e, "Failed to retrieve n8n API key from keychain"),
                        }
                    }
                    "ntfy" => {
                        let mut target = crate::targets::NtfyTarget::new(tid.clone(), url);
                        if let Some(topic) = config.get(&format!("target.{}.topic", tid)).ok().flatten() {
                            target = target.with_topic(topic);
                        }
                        if let Ok(Some(token)) = credentials.retrieve(&format!("ntfy:{}", tid)) {
                            if !token.is_empty() {
                                target = target.with_auth(token);
                            }
                        }
                        target_manager.register(Arc::new(target));
                        tracing::info!(target_id = %tid, "Restored ntfy target");
                    }
                    "make" => {
                        let cred_key = format!("make:{}", tid);
                        let cred_result = credentials.retrieve(&cred_key);
                        tracing::debug!(target_id = %tid, cred_key = %cred_key, result = ?cred_result, "Make.com credential lookup");
                        match cred_result {
                            Ok(Some(api_key)) if !api_key.is_empty() => {
                                let team_id = config.get(&format!("target.{}.team_id", tid)).ok().flatten();
                                let target = crate::targets::MakeTarget::new(tid.clone(), url, api_key, team_id);
                                target_manager.register(Arc::new(target));
                                tracing::info!(target_id = %tid, "Restored Make.com target");
                            }
                            Ok(Some(_)) => tracing::warn!(target_id = %tid, "Make.com API key is empty in keychain"),
                            Ok(None) => tracing::warn!(target_id = %tid, "Make.com API key not found in keychain — target skipped"),
                            Err(e) => tracing::warn!(target_id = %tid, error = %e, "Failed to retrieve Make.com API key from keychain"),
                        }
                    }
                    "zapier" => {
                        let name = config.get(&format!("target.{}.name", tid)).ok().flatten().unwrap_or_else(|| "Zapier Webhook".to_string());
                        match crate::targets::ZapierTarget::new(tid.clone(), name, url) {
                            Ok(target) => {
                                target_manager.register(Arc::new(target));
                                tracing::info!(target_id = %tid, "Restored Zapier target");
                            }
                            Err(e) => tracing::warn!(target_id = %tid, error = %e, "Failed to restore Zapier target"),
                        }
                    }
                    "google-sheets" => {
                        let cred_key = format!("google-sheets:{}", tid);
                        match credentials.retrieve(&cred_key) {
                            Ok(Some(tokens_json)) => {
                                match serde_json::from_str::<crate::targets::google_sheets::GoogleTokens>(&tokens_json) {
                                    Ok(tokens) => {
                                        let email = config.get(&format!("target.{}.email", tid))
                                            .ok().flatten().unwrap_or_default();
                                        let target = crate::targets::GoogleSheetsTarget::new(
                                            tid.clone(), email, tokens,
                                        );
                                        target_manager.register(Arc::new(target));
                                        tracing::info!(target_id = %tid, "Restored Google Sheets target");
                                    }
                                    Err(e) => tracing::warn!(target_id = %tid, error = %e, "Failed to parse Google Sheets tokens"),
                                }
                            }
                            Ok(None) => tracing::warn!(target_id = %tid, "Google Sheets tokens not found — target skipped"),
                            Err(e) => tracing::warn!(target_id = %tid, error = %e, "Failed to retrieve Google Sheets tokens"),
                        }
                    }
                    "custom" => {
                        let name = config.get(&format!("target.{}.name", tid))
                            .ok().flatten().unwrap_or_else(|| "Custom Webhook".to_string());
                        let auth_type_str = config.get(&format!("target.{}.auth_type", tid))
                            .ok().flatten().unwrap_or_else(|| "none".to_string());

                        // Reconstruct auth from config + credentials
                        let auth = match auth_type_str.as_str() {
                            "none" => crate::targets::AuthType::None,
                            "bearer" => {
                                match credentials.retrieve(&format!("custom:{}:token", tid)) {
                                    Ok(Some(token)) if !token.is_empty() => {
                                        crate::targets::AuthType::Bearer { token }
                                    }
                                    _ => {
                                        tracing::warn!(target_id = %tid, "Bearer token not found for custom target");
                                        continue;
                                    }
                                }
                            }
                            "header" => {
                                let header_name = config.get(&format!("target.{}.auth_header_name", tid))
                                    .ok().flatten();
                                let header_value = credentials.retrieve(&format!("custom:{}:header_value", tid))
                                    .ok().flatten();

                                match (header_name, header_value) {
                                    (Some(name), Some(value)) if !value.is_empty() => {
                                        crate::targets::AuthType::Header { name, value }
                                    }
                                    _ => {
                                        tracing::warn!(target_id = %tid, "Header auth incomplete for custom target");
                                        continue;
                                    }
                                }
                            }
                            "basic" => {
                                let username = config.get(&format!("target.{}.auth_username", tid))
                                    .ok().flatten();
                                let password = credentials.retrieve(&format!("custom:{}:password", tid))
                                    .ok().flatten();

                                match (username, password) {
                                    (Some(username), Some(password)) if !password.is_empty() => {
                                        crate::targets::AuthType::Basic { username, password }
                                    }
                                    _ => {
                                        tracing::warn!(target_id = %tid, "Basic auth incomplete for custom target");
                                        continue;
                                    }
                                }
                            }
                            _ => {
                                tracing::warn!(target_id = %tid, auth_type = %auth_type_str, "Unknown auth type for custom target");
                                continue;
                            }
                        };

                        match crate::targets::CustomTarget::new(tid.clone(), name, url, auth) {
                            Ok(target) => {
                                target_manager.register(Arc::new(target));
                                tracing::info!(target_id = %tid, "Restored custom target");
                            }
                            Err(e) => tracing::warn!(target_id = %tid, error = %e, "Failed to restore custom target"),
                        }
                    }
                    _ => tracing::warn!(target_id = %tid, target_type = %ttype, "Unknown target type"),
                }
            }
        }

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
            binding_store,
            config,
            health_tracker,
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

        let target_manager = Arc::new(TargetManager::new(config.clone()));
        let binding_store = Arc::new(BindingStore::new(config.clone()));
        let health_tracker = Arc::new(TargetHealthTracker::new());

        let source_manager = Arc::new(SourceManager::new(
            ledger.clone(),
            file_watcher.clone(),
            config.clone(),
            binding_store.clone(),
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
            target_manager,
            binding_store,
            config,
            health_tracker,
        }
    }
}
