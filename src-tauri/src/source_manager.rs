//! Source Manager - Registry and orchestrator for data sources
//!
//! The SourceManager maps file events to source parsing and ledger enqueue operations.
//! It maintains the registry of available sources, tracks which sources are enabled,
//! and coordinates the flow from file system events to webhook delivery.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::config::AppConfig;
use crate::source_config::SourceConfigStore;
use crate::sources::{Source, SourceError};
use crate::traits::{DeliveryLedgerTrait, FileWatcher, FileWatcherError, LedgerError};

/// Error types for SourceManager operations
#[derive(Debug, thiserror::Error)]
pub enum SourceManagerError {
    #[error("Source not found: {0}")]
    SourceNotFound(String),
    #[error("Unknown watched path: {0}")]
    UnknownPath(PathBuf),
    #[error("Source error: {0}")]
    SourceError(#[from] SourceError),
    #[error("Watcher error: {0}")]
    WatcherError(#[from] FileWatcherError),
    #[error("Ledger error: {0}")]
    LedgerError(#[from] LedgerError),
}

/// Information about a registered source
#[derive(Debug, Clone, serde::Serialize)]
pub struct SourceInfo {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub watch_path: Option<PathBuf>,
}

/// Metadata keys that should always be preserved in payloads (never filtered).
/// These include structural fields that provide context but aren't user-selectable data sections.
const METADATA_KEYS: &[&str] = &[
    "metadata",
    "source",
    "version",
    "generated_at",
    "file_path",
    "timestamp",
    "last_computed_date",
    "today",
    "yesterday",
    "summary",
];

/// Registry and orchestrator for data sources
pub struct SourceManager {
    sources: Mutex<HashMap<String, Arc<dyn Source>>>,
    enabled: Mutex<HashSet<String>>,
    path_to_source: Mutex<HashMap<PathBuf, String>>,
    ledger: Arc<dyn DeliveryLedgerTrait>,
    file_watcher: Arc<dyn FileWatcher>,
    config: Arc<AppConfig>,
}

impl SourceManager {
    /// Create a new SourceManager
    pub fn new(
        ledger: Arc<dyn DeliveryLedgerTrait>,
        file_watcher: Arc<dyn FileWatcher>,
        config: Arc<AppConfig>,
    ) -> Self {
        Self {
            sources: Mutex::new(HashMap::new()),
            enabled: Mutex::new(HashSet::new()),
            path_to_source: Mutex::new(HashMap::new()),
            ledger,
            file_watcher,
            config,
        }
    }

    /// Register a source in the registry
    pub fn register(&self, source: Arc<dyn Source>) {
        let id = source.id().to_string();
        if let Some(path) = source.watch_path() {
            self.path_to_source
                .lock()
                .unwrap()
                .insert(path, id.clone());
        }
        self.sources.lock().unwrap().insert(id, source);
    }

    /// Enable a source: start watching its path, persist to config
    pub fn enable(&self, source_id: &str) -> Result<(), SourceManagerError> {
        let sources = self.sources.lock().unwrap();
        let source = sources
            .get(source_id)
            .ok_or_else(|| SourceManagerError::SourceNotFound(source_id.to_string()))?;

        if let Some(path) = source.watch_path() {
            self.file_watcher.watch(path)?;
        }

        drop(sources);

        self.enabled
            .lock()
            .unwrap()
            .insert(source_id.to_string());

        let config_key = format!("source.{}.enabled", source_id);
        if let Err(e) = self.config.set(&config_key, "true") {
            tracing::warn!(key = %config_key, error = %e, "Failed to persist source enabled state");
        }

        tracing::info!("Enabled source: {}", source_id);
        Ok(())
    }

    /// Disable a source: stop watching, persist to config
    pub fn disable(&self, source_id: &str) -> Result<(), SourceManagerError> {
        let sources = self.sources.lock().unwrap();
        let source = sources
            .get(source_id)
            .ok_or_else(|| SourceManagerError::SourceNotFound(source_id.to_string()))?;

        if let Some(path) = source.watch_path() {
            self.file_watcher.unwatch(path)?;
        }

        drop(sources);

        self.enabled.lock().unwrap().remove(source_id);

        let config_key = format!("source.{}.enabled", source_id);
        if let Err(e) = self.config.set(&config_key, "false") {
            tracing::warn!(key = %config_key, error = %e, "Failed to persist source disabled state");
        }

        tracing::info!("Disabled source: {}", source_id);
        Ok(())
    }

    /// Check if a source is enabled
    pub fn is_enabled(&self, source_id: &str) -> bool {
        self.enabled.lock().unwrap().contains(source_id)
    }

    /// Filter payload based on enabled properties.
    /// Returns a filtered JSON value with only enabled properties, plus metadata keys.
    fn filter_payload(
        &self,
        source_id: &str,
        payload: serde_json::Value,
        source: &Arc<dyn Source>,
    ) -> Result<serde_json::Value, SourceManagerError> {
        let available_props = source.available_properties();

        // If source has no configurable properties, return payload as-is
        if available_props.is_empty() {
            return Ok(payload);
        }

        let config_store = SourceConfigStore::new(self.config.clone());
        let enabled_set = config_store.enabled_set(source_id, &available_props);

        // Parse payload as object
        let mut obj = match payload {
            serde_json::Value::Object(map) => map,
            other => return Ok(other), // Not an object, can't filter
        };

        // Remove keys that are not enabled AND not metadata
        obj.retain(|key, _| {
            // Always keep metadata keys
            if METADATA_KEYS.contains(&key.as_str()) {
                return true;
            }
            // Keep if enabled
            enabled_set.contains(key)
        });

        Ok(serde_json::Value::Object(obj))
    }

    /// Handle a file event: lookup source, parse, filter properties, enqueue to ledger
    pub fn handle_file_event(&self, path: &PathBuf) -> Result<(), SourceManagerError> {
        let source_id = {
            let path_map = self.path_to_source.lock().unwrap();
            path_map.get(path).cloned()
        };

        let source_id =
            source_id.ok_or_else(|| SourceManagerError::UnknownPath(path.clone()))?;

        // Only process if enabled
        if !self.is_enabled(&source_id) {
            tracing::debug!(source_id = %source_id, "Ignoring file event for disabled source");
            return Ok(());
        }

        let source = {
            let sources = self.sources.lock().unwrap();
            sources.get(&source_id).cloned()
        };

        let source =
            source.ok_or_else(|| SourceManagerError::SourceNotFound(source_id.clone()))?;

        tracing::debug!(source_id = %source_id, "Parsing source data for enqueue");
        let payload = source.parse()?;

        // Filter payload based on enabled properties
        let filtered_payload = self.filter_payload(&source_id, payload, &source)?;

        self.ledger.enqueue(&source_id, filtered_payload)?;
        tracing::info!(source_id = %source_id, "Enqueued delivery from source");
        Ok(())
    }

    /// Get a source by ID (for preview commands)
    pub fn get_source(&self, id: &str) -> Option<Arc<dyn Source>> {
        self.sources.lock().unwrap().get(id).cloned()
    }

    /// Parse and filter a source's payload based on enabled properties.
    /// Used by manual push commands.
    pub fn parse_and_filter(&self, source_id: &str) -> Result<serde_json::Value, SourceManagerError> {
        let source = self.get_source(source_id)
            .ok_or_else(|| SourceManagerError::SourceNotFound(source_id.to_string()))?;

        let payload = source.parse()?;
        self.filter_payload(source_id, payload, &source)
    }

    /// List all registered sources with their enabled state
    pub fn list_sources(&self) -> Vec<SourceInfo> {
        let sources = self.sources.lock().unwrap();
        let enabled = self.enabled.lock().unwrap();
        sources
            .iter()
            .map(|(id, source)| SourceInfo {
                id: id.clone(),
                name: source.name().to_string(),
                enabled: enabled.contains(id),
                watch_path: source.watch_path(),
            })
            .collect()
    }

    /// Restore enabled sources from config (call on startup)
    pub fn restore_enabled(&self) -> Vec<String> {
        let source_ids: Vec<String> = { self.sources.lock().unwrap().keys().cloned().collect() };
        let mut restored = Vec::new();
        for id in source_ids {
            let key = format!("source.{}.enabled", id);
            if self.config.get_bool(&key).unwrap_or(false) && self.enable(&id).is_ok() {
                restored.push(id);
            }
        }
        restored
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mocks::ManualFileWatcher;
    use crate::sources::ClaudeStatsSource;
    use crate::DeliveryLedger;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn test_manager() -> (SourceManager, Arc<ManualFileWatcher>) {
        let ledger = Arc::new(DeliveryLedger::open_in_memory().unwrap());
        let watcher = Arc::new(ManualFileWatcher::new());
        let config = Arc::new(AppConfig::open_in_memory().unwrap());
        let mgr = SourceManager::new(ledger, watcher.clone(), config);
        (mgr, watcher)
    }

    fn fake_stats_file() -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        write!(
            file,
            r#"{{
            "version": 2,
            "lastComputedDate": "2026-02-04",
            "dailyActivity": [],
            "dailyModelTokens": [],
            "modelUsage": {{}},
            "totalSessions": 10,
            "totalMessages": 100,
            "hourCounts": {{}}
        }}"#
        )
        .unwrap();
        file
    }

    #[test]
    fn test_register_source() {
        let (mgr, _) = test_manager();
        let source = Arc::new(ClaudeStatsSource::new_with_path("/tmp/fake.json"));
        mgr.register(source);
        assert!(mgr.get_source("claude-stats").is_some());
    }

    #[test]
    fn test_enable_starts_watching() {
        let (mgr, watcher) = test_manager();
        let source = Arc::new(ClaudeStatsSource::new_with_path("/tmp/fake.json"));
        mgr.register(source);
        mgr.enable("claude-stats").unwrap();
        assert!(mgr.is_enabled("claude-stats"));
        assert!(watcher
            .watched_paths()
            .contains(&PathBuf::from("/tmp/fake.json")));
    }

    #[test]
    fn test_disable_stops_watching() {
        let (mgr, watcher) = test_manager();
        let source = Arc::new(ClaudeStatsSource::new_with_path("/tmp/fake.json"));
        mgr.register(source);
        mgr.enable("claude-stats").unwrap();
        mgr.disable("claude-stats").unwrap();
        assert!(!mgr.is_enabled("claude-stats"));
        assert!(watcher.watched_paths().is_empty());
    }

    #[test]
    fn test_handle_file_event_enqueues() {
        let stats_file = fake_stats_file();
        let path = stats_file.path().to_path_buf();

        let ledger = Arc::new(DeliveryLedger::open_in_memory().unwrap());
        let watcher = Arc::new(ManualFileWatcher::new());
        let config = Arc::new(AppConfig::open_in_memory().unwrap());
        let mgr = SourceManager::new(ledger.clone(), watcher, config);

        let source = Arc::new(ClaudeStatsSource::new_with_path(&path));
        mgr.register(source);
        mgr.enable("claude-stats").unwrap();

        mgr.handle_file_event(&path).unwrap();

        let stats = ledger.get_stats().unwrap();
        assert_eq!(stats.pending, 1);
    }

    #[test]
    fn test_list_sources() {
        let (mgr, _) = test_manager();
        let source = Arc::new(ClaudeStatsSource::new_with_path("/tmp/fake.json"));
        mgr.register(source);

        let sources = mgr.list_sources();
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].id, "claude-stats");
        assert!(!sources[0].enabled);
    }

    #[test]
    fn test_enable_nonexistent_source_fails() {
        let (mgr, _) = test_manager();
        assert!(mgr.enable("nonexistent").is_err());
    }

    #[test]
    fn test_restore_enabled() {
        let ledger = Arc::new(DeliveryLedger::open_in_memory().unwrap());
        let watcher = Arc::new(ManualFileWatcher::new());
        let config = Arc::new(AppConfig::open_in_memory().unwrap());
        config
            .set("source.claude-stats.enabled", "true")
            .unwrap();

        let mgr = SourceManager::new(ledger, watcher, config);
        let source = Arc::new(ClaudeStatsSource::new_with_path("/tmp/fake.json"));
        mgr.register(source);

        let restored = mgr.restore_enabled();
        assert_eq!(restored, vec!["claude-stats"]);
        assert!(mgr.is_enabled("claude-stats"));
    }

    #[test]
    fn test_filter_payload_with_enabled_properties() {
        use crate::source_config::SourceConfigStore;
        use serde_json::json;

        let ledger = Arc::new(DeliveryLedger::open_in_memory().unwrap());
        let watcher = Arc::new(ManualFileWatcher::new());
        let config = Arc::new(AppConfig::open_in_memory().unwrap());
        let mgr = SourceManager::new(ledger, watcher, config.clone());
        let source: Arc<dyn Source> = Arc::new(ClaudeStatsSource::new_with_path("/tmp/fake.json"));
        mgr.register(source.clone());

        // Set specific properties enabled
        let store = SourceConfigStore::new(config);
        store.set_enabled("claude-stats", "daily_breakdown", true).unwrap();
        store.set_enabled("claude-stats", "model_totals", false).unwrap();

        // Mock payload with multiple sections
        let payload = json!({
            "metadata": {"source": "localpush"},
            "version": 2,
            "daily_breakdown": [{"date": "2024-01-01"}],
            "model_totals": [{"model": "opus"}],
            "summary": {"total_sessions": 10}
        });

        let filtered = mgr.filter_payload("claude-stats", payload, &source).unwrap();

        // Should keep metadata, version, and daily_breakdown (enabled)
        assert!(filtered.get("metadata").is_some(), "metadata should be preserved");
        assert!(filtered.get("version").is_some(), "version should be preserved");
        assert!(filtered.get("daily_breakdown").is_some(), "daily_breakdown is enabled");

        // Should remove model_totals (disabled)
        assert!(filtered.get("model_totals").is_none(), "model_totals is disabled");

        // summary is a metadata key, so it should be preserved even though not in available_properties
        assert!(filtered.get("summary").is_some(), "summary is metadata and should be preserved");
    }

    #[test]
    fn test_filter_payload_defaults_when_no_config() {
        use serde_json::json;

        let ledger = Arc::new(DeliveryLedger::open_in_memory().unwrap());
        let watcher = Arc::new(ManualFileWatcher::new());
        let config = Arc::new(AppConfig::open_in_memory().unwrap());
        let mgr = SourceManager::new(ledger, watcher, config);
        let source: Arc<dyn Source> = Arc::new(ClaudeStatsSource::new_with_path("/tmp/fake.json"));
        mgr.register(source.clone());

        // No explicit config → should use defaults from available_properties()
        let payload = json!({
            "metadata": {"source": "localpush"},
            "daily_breakdown": [],
            "model_totals": [],
            "cost_breakdown": [],
        });

        let filtered = mgr.filter_payload("claude-stats", payload, &source).unwrap();

        // daily_breakdown and model_totals default to enabled=true
        assert!(filtered.get("daily_breakdown").is_some());
        assert!(filtered.get("model_totals").is_some());

        // cost_breakdown defaults to enabled=false
        assert!(filtered.get("cost_breakdown").is_none());
    }

    #[test]
    fn test_filter_payload_all_disabled_keeps_metadata() {
        use crate::source_config::SourceConfigStore;
        use serde_json::json;

        let ledger = Arc::new(DeliveryLedger::open_in_memory().unwrap());
        let watcher = Arc::new(ManualFileWatcher::new());
        let config = Arc::new(AppConfig::open_in_memory().unwrap());
        let mgr = SourceManager::new(ledger, watcher, config.clone());
        let source: Arc<dyn Source> = Arc::new(ClaudeStatsSource::new_with_path("/tmp/fake.json"));
        mgr.register(source.clone());

        // Disable all properties
        let store = SourceConfigStore::new(config);
        let available = source.available_properties();
        for prop in &available {
            store.set_enabled("claude-stats", &prop.key, false).unwrap();
        }

        let payload = json!({
            "metadata": {"source": "localpush"},
            "version": 2,
            "daily_breakdown": [],
            "model_totals": [],
        });

        let filtered = mgr.filter_payload("claude-stats", payload, &source).unwrap();

        // Metadata should still be there
        assert!(filtered.get("metadata").is_some());
        assert!(filtered.get("version").is_some());

        // Data sections should be removed
        assert!(filtered.get("daily_breakdown").is_none());
        assert!(filtered.get("model_totals").is_none());
    }

    #[test]
    fn test_filter_payload_no_properties_returns_unchanged() {
        use serde_json::json;

        let ledger = Arc::new(DeliveryLedger::open_in_memory().unwrap());
        let watcher = Arc::new(ManualFileWatcher::new());
        let config = Arc::new(AppConfig::open_in_memory().unwrap());
        let mgr = SourceManager::new(ledger, watcher, config);

        // Create a mock source with no configurable properties
        use crate::sources::{Source, SourcePreview, SourceError};
        use std::path::PathBuf;

        struct NoPropertiesSource;
        impl Source for NoPropertiesSource {
            fn id(&self) -> &str { "test-source" }
            fn name(&self) -> &str { "Test" }
            fn watch_path(&self) -> Option<PathBuf> { None }
            fn parse(&self) -> Result<serde_json::Value, SourceError> {
                Ok(json!({"data": 1}))
            }
            fn preview(&self) -> Result<SourcePreview, SourceError> {
                unimplemented!()
            }
            // available_properties() returns empty vec (default)
        }

        let source = Arc::new(NoPropertiesSource) as Arc<dyn Source>;
        let payload = json!({"data": 1, "other": 2});

        let filtered = mgr.filter_payload("test-source", payload.clone(), &source).unwrap();

        // Should return unchanged since no properties are defined
        assert_eq!(filtered, payload);
    }

    #[test]
    fn test_parse_and_filter_integration() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        let mut stats_file = NamedTempFile::new().unwrap();
        write!(
            stats_file,
            r#"{{
            "version": 2,
            "lastComputedDate": "2026-02-04",
            "dailyActivity": [],
            "dailyModelTokens": [],
            "modelUsage": {{}},
            "totalSessions": 10,
            "totalMessages": 100,
            "hourCounts": {{}}
        }}"#
        )
        .unwrap();

        let ledger = Arc::new(DeliveryLedger::open_in_memory().unwrap());
        let watcher = Arc::new(ManualFileWatcher::new());
        let config = Arc::new(AppConfig::open_in_memory().unwrap());
        let mgr = SourceManager::new(ledger, watcher, config.clone());
        let source = Arc::new(ClaudeStatsSource::new_with_path(stats_file.path()));
        mgr.register(source);

        // Disable daily_breakdown
        let store = SourceConfigStore::new(config);
        store.set_enabled("claude-stats", "daily_breakdown", false).unwrap();

        let filtered = mgr.parse_and_filter("claude-stats").unwrap();

        // Should have metadata
        assert!(filtered.get("metadata").is_some());
        assert!(filtered.get("version").is_some());

        // Should NOT have daily_breakdown
        assert!(filtered.get("daily_breakdown").is_none(), "daily_breakdown should be filtered out");

        // Should have model_totals (default enabled)
        assert!(filtered.get("model_totals").is_some());
    }
}
