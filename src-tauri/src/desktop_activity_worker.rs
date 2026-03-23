//! Background worker for the Desktop Activity source.
//!
//! Polls macOS IOKit HIDIdleTime every 30 seconds to track active desktop sessions.
//! When a session ends (3 minutes of inactivity), it enqueues the session data
//! to the delivery ledger for webhook delivery.

use std::sync::Arc;

use crate::iokit_idle;
use crate::source_manager::SourceManager;
use crate::sources::desktop_activity::SharedDesktopActivityState;

/// Poll interval for checking idle time
const POLL_INTERVAL_SECS: u64 = 30;

/// The source ID for desktop activity
const SOURCE_ID: &str = "desktop-activity";

/// Spawn the desktop activity background worker.
///
/// Returns the JoinHandle for the spawned task.
pub fn spawn_worker(
    source_manager: Arc<SourceManager>,
    activity_state: SharedDesktopActivityState,
) -> tauri::async_runtime::JoinHandle<()> {
    tauri::async_runtime::spawn(async move {
        let mut interval =
            tokio::time::interval(std::time::Duration::from_secs(POLL_INTERVAL_SECS));

        tracing::info!(
            "Desktop activity worker started ({}s poll interval)",
            POLL_INTERVAL_SECS
        );

        loop {
            interval.tick().await;

            // Only process if the source is enabled
            if !source_manager.is_enabled(SOURCE_ID) {
                continue;
            }

            // Read idle time from IOKit
            let idle_seconds = match iokit_idle::get_idle_seconds() {
                Ok(seconds) => seconds,
                Err(e) => {
                    tracing::debug!(error = %e, "Failed to read idle time (expected in headless/CI)");
                    continue;
                }
            };

            // Update state machine
            let completed_session = {
                let mut state = activity_state.lock().unwrap();
                state.tick(idle_seconds)
            };

            // If a session just completed, flush it through the normal source-manager path.
            if let Some(session) = completed_session {
                match source_manager.flush_source_on_change(SOURCE_ID) {
                    Ok(0) => {
                        tracing::debug!(
                            duration_minutes = format!("{:.1}", session.duration_minutes),
                            "Desktop session buffered for a future scheduled/manual push"
                        );
                    }
                    Ok(count) => {
                        tracing::info!(
                            deliveries = count,
                            duration_minutes = format!("{:.1}", session.duration_minutes),
                            "Desktop session flushed to on_change bindings"
                        );
                    }
                    Err(e) => {
                        tracing::error!(error = %e, "Failed to flush desktop session");
                    }
                }
            }
        }
    })
}
