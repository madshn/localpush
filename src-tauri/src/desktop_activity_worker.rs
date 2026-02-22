//! Background worker for the Desktop Activity source.
//!
//! Polls macOS IOKit HIDIdleTime every 30 seconds to track active desktop sessions.
//! When a session ends (3 minutes of inactivity), it enqueues the session data
//! to the delivery ledger for webhook delivery.

use std::sync::Arc;

use crate::iokit_idle;
use crate::source_manager::SourceManager;
use crate::sources::desktop_activity::DesktopActivityState;
use crate::traits::DeliveryLedgerTrait;

use std::sync::Mutex;

/// Poll interval for checking idle time
const POLL_INTERVAL_SECS: u64 = 30;

/// The source ID for desktop activity
const SOURCE_ID: &str = "desktop-activity";

/// Spawn the desktop activity background worker.
///
/// Returns the JoinHandle for the spawned task.
pub fn spawn_worker(
    source_manager: Arc<SourceManager>,
    ledger: Arc<dyn DeliveryLedgerTrait>,
) -> tauri::async_runtime::JoinHandle<()> {
    let activity_state = Arc::new(Mutex::new(DesktopActivityState::new()));

    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(POLL_INTERVAL_SECS));

        tracing::info!("Desktop activity worker started ({}s poll interval)", POLL_INTERVAL_SECS);

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

            // If a session just completed, enqueue it
            if let Some(session) = completed_session {
                let payload = serde_json::json!({
                    "type": "desktop_session",
                    "start_timestamp": session.start_timestamp,
                    "end_timestamp": session.end_timestamp,
                    "duration_minutes": session.duration_minutes,
                    "idle_threshold_seconds": session.idle_threshold_seconds,
                    "metadata": {
                        "source": "localpush",
                        "source_id": SOURCE_ID,
                        "generated_at": chrono::Utc::now().to_rfc3339(),
                    }
                });

                match ledger.enqueue(SOURCE_ID, payload) {
                    Ok(event_id) => {
                        tracing::info!(
                            event_id = %event_id,
                            duration_minutes = format!("{:.1}", session.duration_minutes),
                            "Desktop session enqueued for delivery"
                        );
                    }
                    Err(e) => {
                        tracing::error!(error = %e, "Failed to enqueue desktop session");
                    }
                }
            }
        }
    })
}
