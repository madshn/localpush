//! Source Manager - Registry and orchestrator for data sources
//!
//! The SourceManager maps file events to source parsing and ledger enqueue operations.
//! It maintains the registry of available sources, tracks which sources are enabled,
//! and coordinates the flow from file system events to webhook delivery.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::bindings::BindingStore;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeliverySkipReason {
    NoData,
    Unchanged,
}

impl DeliverySkipReason {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::NoData => "no_data",
            Self::Unchanged => "unchanged",
        }
    }
}

#[derive(Debug)]
pub enum PreparedPayload {
    Deliver(serde_json::Value),
    Skip(DeliverySkipReason),
}

/// Metadata keys that should always be preserved in payloads (never filtered).
/// These include structural fields that provide context but aren't user-selectable data sections.
const METADATA_KEYS: &[&str] = &[
    "metadata",
    "meta",
    "source",
    "source_family",
    "source_type",
    "version",
    "schema_version",
    "semantics",
    "generated_at",
    "file_path",
    "timestamp",
    "last_computed_date",
    "today",
    "yesterday",
    "summary",
];

/// How long to buffer file events before flushing (seconds)
const COALESCE_WINDOW_SECS: i64 = 90;

/// Stagger offset between target deliveries (seconds)
const STAGGER_OFFSET_SECS: i64 = 10;

const DELIVERY_FINGERPRINT_KEY_PREFIX: &str = "delivery_fingerprint.";

/// Registry and orchestrator for data sources
pub struct SourceManager {
    sources: Mutex<HashMap<String, Arc<dyn Source>>>,
    enabled: Mutex<HashSet<String>>,
    path_to_source: Mutex<HashMap<PathBuf, String>>,
    /// Sources whose watch paths should use prefix (directory) matching
    recursive_sources: Mutex<HashSet<String>>,
    ledger: Arc<dyn DeliveryLedgerTrait>,
    file_watcher: Arc<dyn FileWatcher>,
    config: Arc<AppConfig>,
    binding_store: Arc<BindingStore>,
    /// Coalescing state: source_id → timestamp of last file event (epoch seconds)
    pending_events: Mutex<HashMap<String, i64>>,
}

impl SourceManager {
    /// Create a new SourceManager
    pub fn new(
        ledger: Arc<dyn DeliveryLedgerTrait>,
        file_watcher: Arc<dyn FileWatcher>,
        config: Arc<AppConfig>,
        binding_store: Arc<BindingStore>,
    ) -> Self {
        Self {
            sources: Mutex::new(HashMap::new()),
            enabled: Mutex::new(HashSet::new()),
            path_to_source: Mutex::new(HashMap::new()),
            recursive_sources: Mutex::new(HashSet::new()),
            ledger,
            file_watcher,
            config,
            binding_store,
            pending_events: Mutex::new(HashMap::new()),
        }
    }

    /// Register a source in the registry
    pub fn register(&self, source: Arc<dyn Source>) {
        let id = source.id().to_string();
        if let Some(path) = source.watch_path() {
            self.path_to_source.lock().unwrap().insert(path, id.clone());
        }
        if source.watch_recursive() {
            self.recursive_sources.lock().unwrap().insert(id.clone());
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
            if source.watch_recursive() {
                self.file_watcher.watch_recursive(path)?;
            } else {
                self.file_watcher.watch(path)?;
            }
        }

        drop(sources);

        self.enabled.lock().unwrap().insert(source_id.to_string());

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

    fn parse_filter_with_source(
        &self,
        source_id: &str,
    ) -> Result<(Arc<dyn Source>, serde_json::Value), SourceManagerError> {
        let source = self
            .get_source(source_id)
            .ok_or_else(|| SourceManagerError::SourceNotFound(source_id.to_string()))?;

        let payload = source.parse()?;
        let filtered = self.filter_payload(source_id, payload, &source)?;
        Ok((source, filtered))
    }

    fn fingerprint_key(source_id: &str) -> String {
        format!("{DELIVERY_FINGERPRINT_KEY_PREFIX}{source_id}")
    }

    fn fingerprint_for_payload(
        source: &Arc<dyn Source>,
        payload: &serde_json::Value,
    ) -> Result<String, SourceManagerError> {
        serde_json::to_string(&source.fingerprint_payload(payload))
            .map_err(|e| SourceManagerError::SourceError(SourceError::JsonError(e)))
    }

    fn last_payload_fingerprint(
        &self,
        source_id: &str,
    ) -> Result<Option<String>, SourceManagerError> {
        self.config
            .get(&Self::fingerprint_key(source_id))
            .map_err(SourceManagerError::from)
    }

    pub fn remember_payload_fingerprint(
        &self,
        source_id: &str,
        payload: &serde_json::Value,
    ) -> Result<(), SourceManagerError> {
        let source = self
            .get_source(source_id)
            .ok_or_else(|| SourceManagerError::SourceNotFound(source_id.to_string()))?;
        let fingerprint = Self::fingerprint_for_payload(&source, payload)?;
        self.config
            .set(&Self::fingerprint_key(source_id), &fingerprint)
            .map_err(SourceManagerError::from)
    }

    /// Handle a file event: resolve source, record for coalescing.
    ///
    /// Instead of immediately parsing and enqueuing, this records the event timestamp.
    /// A background coalescing worker calls `flush_expired()` to process buffered events
    /// after the coalesce window (90s) expires.
    pub fn handle_file_event(&self, path: &PathBuf) -> Result<(), SourceManagerError> {
        let source_id = {
            let path_map = self.path_to_source.lock().unwrap();
            // Try exact match first, then prefix match for directory-backed sources
            path_map.get(path).cloned().or_else(|| {
                let recursive = self.recursive_sources.lock().unwrap();
                path_map
                    .iter()
                    .find(|(watch_path, sid)| {
                        recursive.contains(*sid) && path.starts_with(watch_path)
                    })
                    .map(|(_, sid)| sid.clone())
            })
        };

        let source_id = source_id.ok_or_else(|| SourceManagerError::UnknownPath(path.clone()))?;

        // Only process if enabled
        if !self.is_enabled(&source_id) {
            tracing::debug!(source_id = %source_id, "Ignoring file event for disabled source");
            return Ok(());
        }

        // Record event for coalescing (resets the 90s window)
        let now = chrono::Utc::now().timestamp();
        self.pending_events
            .lock()
            .unwrap()
            .insert(source_id.clone(), now);

        tracing::debug!(source_id = %source_id, "File event recorded for coalescing (90s window)");
        Ok(())
    }

    /// Flush a specific source: parse once, resolve bindings, enqueue with staggered offsets.
    ///
    /// For N on_change bindings, creates N ledger entries with available_at staggered 10s apart.
    /// If bindings exist but none are on-change, skip enqueueing and let scheduled delivery own it.
    /// If no bindings exist, skip enqueueing.
    pub fn flush_source(&self, source_id: &str) -> Result<usize, SourceManagerError> {
        // Remove from pending events
        self.pending_events.lock().unwrap().remove(source_id);

        // Only process if still enabled (may have been disabled during coalesce window)
        if !self.is_enabled(source_id) {
            tracing::debug!(source_id = %source_id, "Skipping flush for disabled source");
            return Ok(0);
        }

        let filtered_payload = match self.prepare_payload_for_delivery(source_id)? {
            PreparedPayload::Deliver(payload) => payload,
            PreparedPayload::Skip(reason) => {
                tracing::debug!(
                    source_id = %source_id,
                    reason = reason.as_str(),
                    "Skipping source flush with no new deliverable payload"
                );
                return Ok(0);
            }
        };

        let bindings = self.binding_store.get_for_source(source_id);
        if bindings.is_empty() {
            tracing::debug!(source_id = %source_id, "Skipping source flush with no bindings");
            return Ok(0);
        }

        let on_change_bindings: Vec<_> = bindings
            .into_iter()
            .filter(|b| b.delivery_mode == "on_change")
            .collect();

        let now = chrono::Utc::now().timestamp();

        if on_change_bindings.is_empty() {
            tracing::debug!(
                source_id = %source_id,
                "Skipping source flush with no on_change bindings"
            );
            return Ok(0);
        }

        // Enqueue one targeted entry per binding with staggered available_at
        let mut enqueued = 0;
        for (i, binding) in on_change_bindings.iter().enumerate() {
            let available_at = now + (i as i64 * STAGGER_OFFSET_SECS);
            self.ledger.enqueue_targeted_at(
                source_id,
                filtered_payload.clone(),
                &binding.endpoint_id,
                available_at,
            )?;
            enqueued += 1;
            tracing::debug!(
                source_id = %source_id,
                endpoint_id = %binding.endpoint_id,
                stagger_offset = i as i64 * STAGGER_OFFSET_SECS,
                "Enqueued staggered delivery"
            );
        }

        self.remember_payload_fingerprint(source_id, &filtered_payload)?;

        tracing::info!(
            source_id = %source_id,
            targets = enqueued,
            "Flushed coalesced event with staggered delivery"
        );
        Ok(enqueued)
    }

    /// Flush a source only to `on_change` bindings, with no legacy fallback.
    /// Used by non-file sources that generate events internally.
    pub fn flush_source_on_change(&self, source_id: &str) -> Result<usize, SourceManagerError> {
        if !self.is_enabled(source_id) {
            tracing::debug!(source_id = %source_id, "Skipping on_change flush for disabled source");
            return Ok(0);
        }

        let on_change_bindings: Vec<_> = self
            .binding_store
            .get_for_source(source_id)
            .into_iter()
            .filter(|b| b.delivery_mode == "on_change")
            .collect();

        if on_change_bindings.is_empty() {
            return Ok(0);
        }

        let filtered_payload = match self.prepare_payload_for_delivery(source_id)? {
            PreparedPayload::Deliver(payload) => payload,
            PreparedPayload::Skip(reason) => {
                tracing::debug!(
                    source_id = %source_id,
                    reason = reason.as_str(),
                    "Skipping on_change flush with no new deliverable payload"
                );
                return Ok(0);
            }
        };

        let now = chrono::Utc::now().timestamp();
        let mut enqueued = 0;
        for (i, binding) in on_change_bindings.iter().enumerate() {
            let available_at = now + (i as i64 * STAGGER_OFFSET_SECS);
            self.ledger.enqueue_targeted_at(
                source_id,
                filtered_payload.clone(),
                &binding.endpoint_id,
                available_at,
            )?;
            enqueued += 1;
        }

        self.remember_payload_fingerprint(source_id, &filtered_payload)?;

        tracing::info!(
            source_id = %source_id,
            targets = enqueued,
            "Flushed source to on_change bindings"
        );
        Ok(enqueued)
    }

    /// Flush all sources whose coalesce window has expired (>90s since last event).
    ///
    /// Called periodically by the coalescing background worker.
    /// Returns the number of sources flushed.
    pub fn flush_expired(&self) -> usize {
        let now = chrono::Utc::now().timestamp();
        let expired: Vec<String> = {
            let pending = self.pending_events.lock().unwrap();
            pending
                .iter()
                .filter(|(_, &timestamp)| now - timestamp >= COALESCE_WINDOW_SECS)
                .map(|(source_id, _)| source_id.clone())
                .collect()
        };

        let mut flushed = 0;
        for source_id in expired {
            match self.flush_source(&source_id) {
                Ok(count) => {
                    if count > 0 {
                        flushed += 1;
                    }
                }
                Err(e) => {
                    tracing::warn!(source_id = %source_id, error = %e, "Failed to flush coalesced source");
                    // Remove from pending to avoid infinite retry
                    self.pending_events.lock().unwrap().remove(&source_id);
                }
            }
        }

        flushed
    }

    /// Check if a source has a pending coalesced event (for testing).
    pub fn has_pending_event(&self, source_id: &str) -> bool {
        self.pending_events.lock().unwrap().contains_key(source_id)
    }

    /// Get a source by ID (for preview commands)
    pub fn get_source(&self, id: &str) -> Option<Arc<dyn Source>> {
        self.sources.lock().unwrap().get(id).cloned()
    }

    /// Parse and filter a source's payload based on enabled properties.
    /// Used by manual push commands.
    pub fn parse_and_filter(
        &self,
        source_id: &str,
    ) -> Result<serde_json::Value, SourceManagerError> {
        let (_, filtered) = self.parse_filter_with_source(source_id)?;
        Ok(filtered)
    }

    pub fn prepare_payload_for_delivery(
        &self,
        source_id: &str,
    ) -> Result<PreparedPayload, SourceManagerError> {
        let (source, filtered) = self.parse_filter_with_source(source_id)?;

        if !source.has_meaningful_payload(&filtered) {
            return Ok(PreparedPayload::Skip(DeliverySkipReason::NoData));
        }

        let fingerprint = Self::fingerprint_for_payload(&source, &filtered)?;
        if self.last_payload_fingerprint(source_id)? == Some(fingerprint) {
            return Ok(PreparedPayload::Skip(DeliverySkipReason::Unchanged));
        }

        Ok(PreparedPayload::Deliver(filtered))
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
    use serde_json::json;
    use tempfile::TempDir;

    fn test_manager() -> (SourceManager, Arc<ManualFileWatcher>) {
        let ledger = Arc::new(DeliveryLedger::open_in_memory().unwrap());
        let watcher = Arc::new(ManualFileWatcher::new());
        let config = Arc::new(AppConfig::open_in_memory().unwrap());
        let binding_store = Arc::new(crate::bindings::BindingStore::new(config.clone()));
        let mgr = SourceManager::new(ledger, watcher.clone(), config, binding_store);
        (mgr, watcher)
    }

    /// Create a temporary projects directory that ClaudeStatsSource can parse successfully.
    /// Returns the TempDir (caller must keep alive for the duration of the test).
    fn fake_projects_dir() -> TempDir {
        TempDir::new().unwrap()
    }

    struct SignalSource {
        path: PathBuf,
    }

    impl Source for SignalSource {
        fn id(&self) -> &str {
            "signal-source"
        }

        fn name(&self) -> &str {
            "Signal Source"
        }

        fn watch_path(&self) -> Option<PathBuf> {
            Some(self.path.clone())
        }

        fn parse(&self) -> Result<serde_json::Value, SourceError> {
            Ok(json!({
                "count": 1,
                "metadata": {
                    "source": "localpush",
                    "source_id": "signal-source",
                }
            }))
        }

        fn preview(&self) -> Result<crate::sources::SourcePreview, SourceError> {
            unimplemented!()
        }
    }

    struct EmptySource {
        path: PathBuf,
    }

    impl Source for EmptySource {
        fn id(&self) -> &str {
            "empty-source"
        }

        fn name(&self) -> &str {
            "Empty Source"
        }

        fn watch_path(&self) -> Option<PathBuf> {
            Some(self.path.clone())
        }

        fn parse(&self) -> Result<serde_json::Value, SourceError> {
            Ok(json!({
                "sessions": [],
                "metadata": {
                    "source": "localpush",
                    "source_id": "empty-source",
                }
            }))
        }

        fn preview(&self) -> Result<crate::sources::SourcePreview, SourceError> {
            unimplemented!()
        }

        fn has_meaningful_payload(&self, payload: &serde_json::Value) -> bool {
            payload["sessions"]
                .as_array()
                .map(|sessions| !sessions.is_empty())
                .unwrap_or(false)
        }
    }

    struct TimestampedSignalSource {
        path: PathBuf,
        counter: Mutex<u64>,
    }

    impl Source for TimestampedSignalSource {
        fn id(&self) -> &str {
            "timestamped-signal"
        }

        fn name(&self) -> &str {
            "Timestamped Signal"
        }

        fn watch_path(&self) -> Option<PathBuf> {
            Some(self.path.clone())
        }

        fn parse(&self) -> Result<serde_json::Value, SourceError> {
            let mut counter = self.counter.lock().unwrap();
            *counter += 1;
            Ok(json!({
                "count": 1,
                "metadata": {
                    "source": "localpush",
                    "source_id": "timestamped-signal",
                    "generated_at": format!("tick-{}", counter),
                }
            }))
        }

        fn preview(&self) -> Result<crate::sources::SourcePreview, SourceError> {
            unimplemented!()
        }
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
    fn test_handle_file_event_coalesces() {
        let projects_dir = fake_projects_dir();
        let path = projects_dir.path().to_path_buf();

        let ledger = Arc::new(DeliveryLedger::open_in_memory().unwrap());
        let watcher = Arc::new(ManualFileWatcher::new());
        let config = Arc::new(AppConfig::open_in_memory().unwrap());
        let binding_store = Arc::new(crate::bindings::BindingStore::new(config.clone()));
        let mgr = SourceManager::new(ledger.clone(), watcher, config, binding_store);

        let source = Arc::new(ClaudeStatsSource::new_with_path(&path));
        mgr.register(source);
        mgr.enable("claude-stats").unwrap();

        mgr.handle_file_event(&path).unwrap();

        // Event is buffered, not immediately enqueued
        let stats = ledger.get_stats().unwrap();
        assert_eq!(stats.pending, 0, "coalescing should buffer events");
        assert!(
            mgr.has_pending_event("claude-stats"),
            "source should have pending coalesce event"
        );
    }

    #[test]
    fn test_flush_source_enqueues() {
        let projects_dir = fake_projects_dir();
        let path = projects_dir.path().to_path_buf();

        let ledger = Arc::new(DeliveryLedger::open_in_memory().unwrap());
        let watcher = Arc::new(ManualFileWatcher::new());
        let config = Arc::new(AppConfig::open_in_memory().unwrap());
        let binding_store = Arc::new(crate::bindings::BindingStore::new(config.clone()));
        let mgr = SourceManager::new(ledger.clone(), watcher, config, binding_store);

        let source = Arc::new(SignalSource { path: path.clone() });
        mgr.register(source);
        mgr.enable("signal-source").unwrap();

        // Record event, then flush immediately
        mgr.handle_file_event(&path).unwrap();
        let count = mgr.flush_source("signal-source").unwrap();

        // No bindings → falls back to single untargeted enqueue
        assert_eq!(count, 1);
        let stats = ledger.get_stats().unwrap();
        assert_eq!(stats.pending, 1);
        assert!(
            !mgr.has_pending_event("signal-source"),
            "flush should clear pending event"
        );
    }

    #[test]
    fn test_flush_source_with_bindings_staggers() {
        let projects_dir = fake_projects_dir();
        let path = projects_dir.path().to_path_buf();

        let ledger = Arc::new(DeliveryLedger::open_in_memory().unwrap());
        let watcher = Arc::new(ManualFileWatcher::new());
        let config = Arc::new(AppConfig::open_in_memory().unwrap());
        let binding_store = Arc::new(crate::bindings::BindingStore::new(config.clone()));
        let mgr = SourceManager::new(ledger.clone(), watcher, config, binding_store.clone());

        let source = Arc::new(SignalSource { path: path.clone() });
        mgr.register(source);
        mgr.enable("signal-source").unwrap();

        // Create two bindings for this source
        let binding1 = crate::bindings::SourceBinding {
            source_id: "signal-source".to_string(),
            target_id: "n8n-1".to_string(),
            endpoint_id: "ep1".to_string(),
            endpoint_url: "https://example.com/wh1".to_string(),
            endpoint_name: "Workflow 1".to_string(),
            active: true,
            delivery_mode: "on_change".to_string(),
            schedule_time: None,
            schedule_day: None,
            headers_json: None,
            auth_credential_key: None,
            last_scheduled_at: None,
            created_at: chrono::Utc::now().timestamp(),
        };
        let mut binding2 = binding1.clone();
        binding2.target_id = "ntfy-1".to_string();
        binding2.endpoint_id = "ep2".to_string();
        binding2.endpoint_url = "https://example.com/wh2".to_string();
        binding2.endpoint_name = "Workflow 2".to_string();

        binding_store.save(&binding1).unwrap();
        binding_store.save(&binding2).unwrap();

        // Flush (no need for handle_file_event first — flush_source works independently)
        let count = mgr.flush_source("signal-source").unwrap();
        assert_eq!(count, 2, "should create one entry per binding");

        let stats = ledger.get_stats().unwrap();
        assert_eq!(stats.pending, 2);
    }

    #[test]
    fn test_coalesce_resets_on_new_events() {
        let projects_dir = fake_projects_dir();
        let path = projects_dir.path().to_path_buf();

        let ledger = Arc::new(DeliveryLedger::open_in_memory().unwrap());
        let watcher = Arc::new(ManualFileWatcher::new());
        let config = Arc::new(AppConfig::open_in_memory().unwrap());
        let binding_store = Arc::new(crate::bindings::BindingStore::new(config.clone()));
        let mgr = SourceManager::new(ledger.clone(), watcher, config, binding_store);

        let source = Arc::new(SignalSource { path: path.clone() });
        mgr.register(source);
        mgr.enable("signal-source").unwrap();

        // Fire multiple events
        mgr.handle_file_event(&path).unwrap();
        mgr.handle_file_event(&path).unwrap();
        mgr.handle_file_event(&path).unwrap();

        // Should still be just one pending event (latest timestamp)
        assert!(mgr.has_pending_event("signal-source"));
        let stats = ledger.get_stats().unwrap();
        assert_eq!(
            stats.pending, 0,
            "multiple events should not create multiple enqueues"
        );
    }

    #[test]
    fn test_flush_expired_respects_window() {
        let projects_dir = fake_projects_dir();
        let path = projects_dir.path().to_path_buf();

        let ledger = Arc::new(DeliveryLedger::open_in_memory().unwrap());
        let watcher = Arc::new(ManualFileWatcher::new());
        let config = Arc::new(AppConfig::open_in_memory().unwrap());
        let binding_store = Arc::new(crate::bindings::BindingStore::new(config.clone()));
        let mgr = SourceManager::new(ledger.clone(), watcher, config, binding_store);

        let source = Arc::new(SignalSource { path: path.clone() });
        mgr.register(source);
        mgr.enable("signal-source").unwrap();

        // Record event with current timestamp
        mgr.handle_file_event(&path).unwrap();

        // flush_expired should NOT flush (event is fresh, within 90s window)
        let flushed = mgr.flush_expired();
        assert_eq!(flushed, 0, "fresh events should not be flushed");
        assert!(
            mgr.has_pending_event("signal-source"),
            "event should still be pending"
        );

        // Manually backdate the event to simulate 90s passing
        {
            let mut pending = mgr.pending_events.lock().unwrap();
            let old_ts = chrono::Utc::now().timestamp() - 91;
            pending.insert("signal-source".to_string(), old_ts);
        }

        // Now flush_expired should flush
        let flushed = mgr.flush_expired();
        assert_eq!(flushed, 1, "expired events should be flushed");
        assert!(
            !mgr.has_pending_event("signal-source"),
            "event should be cleared after flush"
        );

        let stats = ledger.get_stats().unwrap();
        assert_eq!(stats.pending, 1);
    }

    #[test]
    fn test_prepare_payload_skips_when_source_reports_no_data() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().to_path_buf();

        let ledger = Arc::new(DeliveryLedger::open_in_memory().unwrap());
        let watcher = Arc::new(ManualFileWatcher::new());
        let config = Arc::new(AppConfig::open_in_memory().unwrap());
        let binding_store = Arc::new(crate::bindings::BindingStore::new(config.clone()));
        let mgr = SourceManager::new(ledger, watcher, config, binding_store);

        mgr.register(Arc::new(EmptySource { path }));

        match mgr.prepare_payload_for_delivery("empty-source").unwrap() {
            PreparedPayload::Skip(DeliverySkipReason::NoData) => {}
            other => panic!("expected no-data skip, got {other:?}"),
        }
    }

    #[test]
    fn test_prepare_payload_skips_when_only_freshness_timestamp_changed() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().to_path_buf();

        let ledger = Arc::new(DeliveryLedger::open_in_memory().unwrap());
        let watcher = Arc::new(ManualFileWatcher::new());
        let config = Arc::new(AppConfig::open_in_memory().unwrap());
        let binding_store = Arc::new(crate::bindings::BindingStore::new(config.clone()));
        let mgr = SourceManager::new(ledger, watcher, config, binding_store);

        mgr.register(Arc::new(TimestampedSignalSource {
            path,
            counter: Mutex::new(0),
        }));

        let first = match mgr
            .prepare_payload_for_delivery("timestamped-signal")
            .unwrap()
        {
            PreparedPayload::Deliver(payload) => payload,
            other => panic!("expected deliverable payload, got {other:?}"),
        };
        mgr.remember_payload_fingerprint("timestamped-signal", &first)
            .unwrap();

        match mgr
            .prepare_payload_for_delivery("timestamped-signal")
            .unwrap()
        {
            PreparedPayload::Skip(DeliverySkipReason::Unchanged) => {}
            other => panic!("expected unchanged skip, got {other:?}"),
        }
    }

    #[test]
    fn test_disabled_source_not_coalesced() {
        let projects_dir = fake_projects_dir();
        let path = projects_dir.path().to_path_buf();

        let ledger = Arc::new(DeliveryLedger::open_in_memory().unwrap());
        let watcher = Arc::new(ManualFileWatcher::new());
        let config = Arc::new(AppConfig::open_in_memory().unwrap());
        let binding_store = Arc::new(crate::bindings::BindingStore::new(config.clone()));
        let mgr = SourceManager::new(ledger.clone(), watcher, config, binding_store);

        let source = Arc::new(ClaudeStatsSource::new_with_path(&path));
        mgr.register(source);
        // Do NOT enable

        mgr.handle_file_event(&path).unwrap();

        assert!(
            !mgr.has_pending_event("claude-stats"),
            "disabled sources should not coalesce"
        );
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
        config.set("source.claude-stats.enabled", "true").unwrap();

        let binding_store = Arc::new(crate::bindings::BindingStore::new(config.clone()));
        let mgr = SourceManager::new(ledger, watcher, config, binding_store);
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
        let binding_store = Arc::new(crate::bindings::BindingStore::new(config.clone()));
        let mgr = SourceManager::new(ledger, watcher, config.clone(), binding_store);
        let source: Arc<dyn Source> = Arc::new(ClaudeStatsSource::new_with_path("/tmp/fake.json"));
        mgr.register(source.clone());

        // Set specific properties enabled
        let store = SourceConfigStore::new(config);
        store
            .set_enabled("claude-stats", "daily_breakdown", true)
            .unwrap();
        store
            .set_enabled("claude-stats", "model_totals", false)
            .unwrap();

        // Mock payload with multiple sections
        let payload = json!({
            "metadata": {"source": "localpush"},
            "version": 2,
            "daily_breakdown": [{"date": "2024-01-01"}],
            "model_totals": [{"model": "opus"}],
            "summary": {"total_sessions": 10}
        });

        let filtered = mgr
            .filter_payload("claude-stats", payload, &source)
            .unwrap();

        // Should keep metadata, version, and daily_breakdown (enabled)
        assert!(
            filtered.get("metadata").is_some(),
            "metadata should be preserved"
        );
        assert!(
            filtered.get("version").is_some(),
            "version should be preserved"
        );
        assert!(
            filtered.get("daily_breakdown").is_some(),
            "daily_breakdown is enabled"
        );

        // Should remove model_totals (disabled)
        assert!(
            filtered.get("model_totals").is_none(),
            "model_totals is disabled"
        );

        // summary is a metadata key, so it should be preserved even though not in available_properties
        assert!(
            filtered.get("summary").is_some(),
            "summary is metadata and should be preserved"
        );
    }

    #[test]
    fn test_filter_payload_defaults_when_no_config() {
        use serde_json::json;

        let ledger = Arc::new(DeliveryLedger::open_in_memory().unwrap());
        let watcher = Arc::new(ManualFileWatcher::new());
        let config = Arc::new(AppConfig::open_in_memory().unwrap());
        let binding_store = Arc::new(crate::bindings::BindingStore::new(config.clone()));
        let mgr = SourceManager::new(ledger, watcher, config, binding_store);
        let source: Arc<dyn Source> = Arc::new(ClaudeStatsSource::new_with_path("/tmp/fake.json"));
        mgr.register(source.clone());

        // No explicit config → should use defaults from available_properties()
        let payload = json!({
            "metadata": {"source": "localpush"},
            "daily_breakdown": [],
            "model_totals": [],
            "cost_breakdown": [],
        });

        let filtered = mgr
            .filter_payload("claude-stats", payload, &source)
            .unwrap();

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
        let binding_store = Arc::new(crate::bindings::BindingStore::new(config.clone()));
        let mgr = SourceManager::new(ledger, watcher, config.clone(), binding_store);
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

        let filtered = mgr
            .filter_payload("claude-stats", payload, &source)
            .unwrap();

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
        let binding_store = Arc::new(crate::bindings::BindingStore::new(config.clone()));
        let mgr = SourceManager::new(ledger, watcher, config, binding_store);

        // Create a mock source with no configurable properties
        use crate::sources::{Source, SourceError, SourcePreview};
        use std::path::PathBuf;

        struct NoPropertiesSource;
        impl Source for NoPropertiesSource {
            fn id(&self) -> &str {
                "test-source"
            }
            fn name(&self) -> &str {
                "Test"
            }
            fn watch_path(&self) -> Option<PathBuf> {
                None
            }
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

        let filtered = mgr
            .filter_payload("test-source", payload.clone(), &source)
            .unwrap();

        // Should return unchanged since no properties are defined
        assert_eq!(filtered, payload);
    }

    #[test]
    fn test_parse_and_filter_integration() {
        let projects_dir = fake_projects_dir();

        let ledger = Arc::new(DeliveryLedger::open_in_memory().unwrap());
        let watcher = Arc::new(ManualFileWatcher::new());
        let config = Arc::new(AppConfig::open_in_memory().unwrap());
        let binding_store = Arc::new(crate::bindings::BindingStore::new(config.clone()));
        let mgr = SourceManager::new(ledger, watcher, config.clone(), binding_store);
        let source = Arc::new(ClaudeStatsSource::new_with_path(projects_dir.path()));
        mgr.register(source);

        // Disable daily_breakdown
        let store = SourceConfigStore::new(config);
        store
            .set_enabled("claude-stats", "daily_breakdown", false)
            .unwrap();

        let filtered = mgr.parse_and_filter("claude-stats").unwrap();

        // Should have metadata
        assert!(filtered.get("metadata").is_some());
        assert!(filtered.get("version").is_some());

        // Should NOT have daily_breakdown
        assert!(
            filtered.get("daily_breakdown").is_none(),
            "daily_breakdown should be filtered out"
        );

        // Should have model_totals (default enabled)
        assert!(filtered.get("model_totals").is_some());
    }
}
