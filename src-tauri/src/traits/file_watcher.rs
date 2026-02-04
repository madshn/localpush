//! File watching trait for monitoring local files

use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum FileWatcherError {
    #[error("Path not found: {0}")]
    PathNotFound(PathBuf),
    #[error("Permission denied: {0}")]
    PermissionDenied(PathBuf),
    #[error("Watch error: {0}")]
    WatchError(String),
}

/// Event emitted when a watched file changes
#[derive(Debug, Clone)]
pub struct FileEvent {
    pub path: PathBuf,
    pub kind: FileEventKind,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FileEventKind {
    Created,
    Modified,
    Deleted,
    Renamed { from: PathBuf },
}

/// Trait for file system watching
///
/// Production: FSEvents via `notify` crate
/// Testing: Manual event emission
#[cfg_attr(test, mockall::automock)]
pub trait FileWatcher: Send + Sync {
    /// Start watching a path
    fn watch(&self, path: PathBuf) -> Result<(), FileWatcherError>;

    /// Stop watching a path
    fn unwatch(&self, path: PathBuf) -> Result<(), FileWatcherError>;

    /// Get the list of currently watched paths
    fn watched_paths(&self) -> Vec<PathBuf>;
}
