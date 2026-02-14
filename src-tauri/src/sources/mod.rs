use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

use crate::source_config::PropertyDef;

pub mod claude_stats;
pub mod claude_sessions;
pub mod apple_podcasts;
pub mod apple_notes;
pub mod apple_photos;

pub use claude_stats::ClaudeStatsSource;
pub use claude_sessions::ClaudeSessionsSource;
pub use apple_podcasts::ApplePodcastsSource;
pub use apple_notes::AppleNotesSource;
pub use apple_photos::ApplePhotosSource;

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

    /// Generate transparency preview showing what user will see
    fn preview(&self) -> Result<SourcePreview, SourceError>;

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
}
