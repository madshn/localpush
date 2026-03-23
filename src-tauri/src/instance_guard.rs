use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

use rusqlite::Connection;
use tauri::{AppHandle, Manager};

#[derive(Debug, thiserror::Error)]
pub enum InstanceGuardError {
    #[error("Another LocalPush instance is already running")]
    AlreadyRunning,
    #[error("Failed to resolve app data directory")]
    MissingAppDataDir,
    #[error("Failed to create app data directory: {0}")]
    CreateDir(#[from] std::io::Error),
    #[error("Failed to open instance lock database: {0}")]
    OpenDb(#[source] rusqlite::Error),
    #[error("Failed to configure instance lock database: {0}")]
    ConfigureDb(#[source] rusqlite::Error),
    #[error("Failed to acquire instance lock: {0}")]
    Acquire(#[source] rusqlite::Error),
}

/// Keeps an exclusive SQLite transaction open for the app lifetime.
///
/// This prevents multiple LocalPush processes from running duplicate background
/// workers against the same ledger/config files.
pub struct InstanceGuard {
    _lock_conn: Mutex<Connection>,
    #[allow(dead_code)]
    lock_path: PathBuf,
}

impl InstanceGuard {
    pub fn acquire(app_handle: &AppHandle) -> Result<Self, InstanceGuardError> {
        let app_data_dir = app_handle
            .path()
            .app_data_dir()
            .map_err(|_| InstanceGuardError::MissingAppDataDir)?;
        std::fs::create_dir_all(&app_data_dir)?;

        let lock_path = app_data_dir.join("instance-lock.sqlite");
        let conn = Connection::open(&lock_path).map_err(InstanceGuardError::OpenDb)?;

        conn.busy_timeout(Duration::from_millis(0))
            .map_err(InstanceGuardError::ConfigureDb)?;
        conn.pragma_update(None, "journal_mode", "DELETE")
            .map_err(InstanceGuardError::ConfigureDb)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS app_instance_lock (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                acquired_at INTEGER NOT NULL
            );
            BEGIN EXCLUSIVE;
            INSERT OR REPLACE INTO app_instance_lock (id, acquired_at)
            VALUES (1, strftime('%s', 'now'));",
        )
        .map_err(|error| {
            if matches!(
                error,
                rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error {
                        code: rusqlite::ErrorCode::DatabaseBusy,
                        ..
                    },
                    _
                )
            ) {
                InstanceGuardError::AlreadyRunning
            } else {
                InstanceGuardError::Acquire(error)
            }
        })?;

        Ok(Self {
            _lock_conn: Mutex::new(conn),
            lock_path,
        })
    }
}
