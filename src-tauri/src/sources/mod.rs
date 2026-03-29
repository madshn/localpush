use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;
use thiserror::Error;

use crate::source_config::PropertyDef;

pub mod apple_notes;
pub mod apple_photos;
pub mod apple_podcasts;
pub mod cic_task_output;
pub mod claude_sessions;
pub mod claude_sessions_collector;
pub mod claude_stats;
pub mod codex_sessions;
pub mod codex_stats;
pub mod desktop_activity;

pub use apple_notes::AppleNotesSource;
pub use apple_photos::ApplePhotosSource;
pub use apple_podcasts::ApplePodcastsSource;
pub use cic_task_output::CicTaskOutputSource;
pub use claude_sessions::ClaudeSessionsSource;
pub use claude_stats::ClaudeStatsSource;
pub use codex_sessions::CodexSessionsSource;
pub use codex_stats::CodexStatsSource;
pub use desktop_activity::DesktopActivitySource;

/// Errors that can occur when parsing or accessing sources
#[derive(Debug, Error)]
pub enum SourceError {
    #[error("File not found: {0}")]
    FileNotFound(PathBuf),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Permission denied: {0}. Grant Full Disk Access in System Settings > Privacy & Security > Full Disk Access > LocalPush")]
    PermissionDenied(String),
}

/// A field in the source preview
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviewField {
    pub label: String,
    pub value: String,
    pub sensitive: bool, // Should be masked in transparency preview
}

/// Human-readable preview of what will be sent to webhooks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourcePreview {
    pub title: String,
    pub summary: String,
    pub fields: Vec<PreviewField>,
    pub last_updated: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PostDeliveryAction {
    None,
    FlushNext,
}

fn default_fingerprint_payload(payload: &serde_json::Value) -> serde_json::Value {
    let mut normalized = payload.clone();

    if let Some(obj) = normalized.as_object_mut() {
        obj.remove("timestamp");

        if let Some(meta) = obj.get_mut("metadata").and_then(|m| m.as_object_mut()) {
            meta.remove("generated_at");
        }

        if let Some(meta) = obj.get_mut("meta").and_then(|m| m.as_object_mut()) {
            meta.remove("generated_at");
        }
    }

    normalized
}

fn metadata_modified_millis(metadata: &fs::Metadata) -> u128 {
    metadata
        .modified()
        .ok()
        .and_then(|ts| ts.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}

pub(crate) fn file_change_hint(path: &Path) -> Result<Option<String>, SourceError> {
    let metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(SourceError::IoError(err)),
    };

    Ok(Some(format!(
        "file:{}:{}:{}",
        path.display(),
        metadata.len(),
        metadata_modified_millis(&metadata)
    )))
}

pub(crate) fn recursive_path_change_hint(
    root: &Path,
    extension: Option<&str>,
) -> Result<Option<String>, SourceError> {
    fn visit(
        dir: &Path,
        extension: Option<&str>,
        count: &mut u64,
        total_size: &mut u64,
        latest_modified: &mut u128,
        latest_path: &mut String,
    ) -> Result<(), SourceError> {
        let entries = fs::read_dir(dir)?;
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            let file_type = entry.file_type()?;
            if file_type.is_dir() {
                visit(
                    &path,
                    extension,
                    count,
                    total_size,
                    latest_modified,
                    latest_path,
                )?;
                continue;
            }

            if !file_type.is_file() {
                continue;
            }

            if extension
                .is_some_and(|wanted| path.extension().and_then(|ext| ext.to_str()) != Some(wanted))
            {
                continue;
            }

            let metadata = entry.metadata()?;
            let modified = metadata_modified_millis(&metadata);
            *count += 1;
            *total_size += metadata.len();
            if modified >= *latest_modified {
                *latest_modified = modified;
                *latest_path = path.display().to_string();
            }
        }

        Ok(())
    }

    if !root.exists() {
        return Ok(None);
    }

    let mut count = 0_u64;
    let mut total_size = 0_u64;
    let mut latest_modified = 0_u128;
    let mut latest_path = String::new();
    visit(
        root,
        extension,
        &mut count,
        &mut total_size,
        &mut latest_modified,
        &mut latest_path,
    )?;

    Ok(Some(format!(
        "tree:{}:{}:{}:{}:{}",
        root.display(),
        extension.unwrap_or("*"),
        count,
        total_size,
        if latest_path.is_empty() {
            latest_modified.to_string()
        } else {
            format!("{latest_modified}:{latest_path}")
        }
    )))
}

/// Trait that all sources must implement
pub trait Source: Send + Sync {
    /// Unique identifier for this source (e.g., "claude-stats")
    fn id(&self) -> &str;

    /// Human-readable name (e.g., "Claude Code Statistics")
    fn name(&self) -> &str;

    /// Path to watch for changes (if file-based)
    fn watch_path(&self) -> Option<PathBuf>;

    /// Parse current data and return payload for webhook delivery
    fn parse(&self) -> Result<serde_json::Value, SourceError>;

    /// Prepare a payload for actual delivery.
    ///
    /// Defaults to `parse()`, but sources can override this to claim
    /// source-specific work items only when LocalPush is actually enqueueing.
    fn prepare_for_delivery(&self) -> Result<serde_json::Value, SourceError> {
        self.parse()
    }

    /// Generate transparency preview showing what user will see
    fn preview(&self) -> Result<SourcePreview, SourceError>;

    /// Return a cheap metadata-based hint used to skip unchanged scheduled parses.
    ///
    /// Sources should keep this conservative: return `Some` only when the hint is
    /// cheap to compute and guaranteed to change whenever the delivered payload
    /// could change for reasons other than freshness timestamps.
    fn delivery_change_hint(&self) -> Result<Option<String>, SourceError> {
        Ok(None)
    }

    /// Whether the watch path should be watched recursively.
    /// Override to return true for directory-backed sources (e.g., Claude Sessions).
    fn watch_recursive(&self) -> bool {
        false
    }

    /// List of configurable properties for this source.
    /// Default implementation returns empty (no configurable properties).
    fn available_properties(&self) -> Vec<PropertyDef> {
        vec![]
    }

    /// Whether a specific filesystem event is relevant to this source.
    fn should_process_event(&self, _path: &Path) -> bool {
        true
    }

    /// Record source-specific bookkeeping after LocalPush enqueues a delivery.
    fn on_delivery_queued(
        &self,
        _event_id: &str,
        _payload: &serde_json::Value,
    ) -> Result<(), SourceError> {
        Ok(())
    }

    /// Allow a source to rewrite delivery headers for one queued event.
    fn rewrite_delivery_headers(
        &self,
        _event_id: &str,
        _headers: &mut Vec<(String, String)>,
    ) -> Result<(), SourceError> {
        Ok(())
    }

    /// Allow a source to react after one queued delivery succeeds.
    fn on_delivery_success(
        &self,
        _event_id: &str,
        _payload: &serde_json::Value,
    ) -> Result<PostDeliveryAction, SourceError> {
        Ok(PostDeliveryAction::None)
    }

    /// Whether the filtered payload contains enough signal to be worth delivering.
    ///
    /// Default: `true` — specific sources should override this to suppress
    /// empty snapshots or zero-activity aggregates.
    fn has_meaningful_payload(&self, _payload: &serde_json::Value) -> bool {
        true
    }

    /// Normalize the payload used for change detection between pushes.
    ///
    /// Default removes volatile freshness timestamps so scheduled deliveries only
    /// fire when the substantive content changes.
    fn fingerprint_payload(&self, payload: &serde_json::Value) -> serde_json::Value {
        default_fingerprint_payload(payload)
    }
}
