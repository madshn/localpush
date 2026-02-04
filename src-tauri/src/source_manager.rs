//! Source Manager - Registry and orchestrator for data sources
//!
//! The SourceManager maps file events to source parsing and ledger enqueue operations.
//! It maintains the registry of available sources, tracks which sources are enabled,
//! and coordinates the flow from file system events to webhook delivery.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::config::AppConfig;
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
        self.config
            .set(&format!("source.{}.enabled", source_id), "true")
            .ok();
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
        self.config
            .set(&format!("source.{}.enabled", source_id), "false")
            .ok();
        tracing::info!("Disabled source: {}", source_id);
        Ok(())
    }

    /// Check if a source is enabled
    pub fn is_enabled(&self, source_id: &str) -> bool {
        self.enabled.lock().unwrap().contains(source_id)
    }

    /// Handle a file event: lookup source, parse, enqueue to ledger
    pub fn handle_file_event(&self, path: &PathBuf) -> Result<(), SourceManagerError> {
        let source_id = {
            let path_map = self.path_to_source.lock().unwrap();
            path_map.get(path).cloned()
        };

        let source_id =
            source_id.ok_or_else(|| SourceManagerError::UnknownPath(path.clone()))?;

        // Only process if enabled
        if !self.is_enabled(&source_id) {
            return Ok(());
        }

        let source = {
            let sources = self.sources.lock().unwrap();
            sources.get(&source_id).cloned()
        };

        let source =
            source.ok_or_else(|| SourceManagerError::SourceNotFound(source_id.clone()))?;

        let payload = source.parse()?;
        self.ledger.enqueue(&source_id, payload)?;
        tracing::info!("Enqueued delivery from source: {}", source_id);
        Ok(())
    }

    /// Get a source by ID (for preview commands)
    pub fn get_source(&self, id: &str) -> Option<Arc<dyn Source>> {
        self.sources.lock().unwrap().get(id).cloned()
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
}
