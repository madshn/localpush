//! Desktop Activity Source — tracks active computer sessions via macOS IOKit idle time.
//!
//! Sessions are defined by keyboard/mouse activity. A session starts when
//! the user becomes active and ends after 3 minutes of inactivity.
//! No Accessibility permissions required.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;

use super::{PreviewField, Source, SourceError, SourcePreview};

/// Idle threshold: session ends after this many seconds of inactivity.
pub const IDLE_THRESHOLD_SECS: f64 = 180.0; // 3 minutes

/// Session state machine
#[derive(Debug, Clone, PartialEq)]
pub enum SessionState {
    /// No active session — user is idle
    Inactive,
    /// User is actively using the computer
    Active {
        start: DateTime<Utc>,
        last_active: DateTime<Utc>,
    },
}

/// A completed desktop session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletedSession {
    pub start_timestamp: i64,
    pub end_timestamp: i64,
    pub duration_minutes: f64,
    pub idle_threshold_seconds: f64,
}

/// Internal state for the desktop activity tracker
pub struct DesktopActivityState {
    pub state: SessionState,
    pub completed: Vec<CompletedSession>,
}

impl Default for DesktopActivityState {
    fn default() -> Self {
        Self::new()
    }
}

impl DesktopActivityState {
    pub fn new() -> Self {
        Self {
            state: SessionState::Inactive,
            completed: Vec::new(),
        }
    }

    /// Update state based on current idle time. Returns Some(session) if a session just completed.
    pub fn tick(&mut self, idle_seconds: f64) -> Option<CompletedSession> {
        let now = Utc::now();

        match &self.state {
            SessionState::Inactive => {
                if idle_seconds < IDLE_THRESHOLD_SECS {
                    // User became active — start new session
                    self.state = SessionState::Active {
                        start: now,
                        last_active: now,
                    };
                    tracing::info!("Desktop session started");
                }
                None
            }
            SessionState::Active { start, last_active } => {
                if idle_seconds >= IDLE_THRESHOLD_SECS {
                    // User went idle — finalize session
                    let session = CompletedSession {
                        start_timestamp: start.timestamp(),
                        end_timestamp: last_active.timestamp(),
                        duration_minutes: (*last_active - *start).num_seconds() as f64 / 60.0,
                        idle_threshold_seconds: IDLE_THRESHOLD_SECS,
                    };
                    tracing::info!(
                        duration_minutes = format!("{:.1}", session.duration_minutes),
                        "Desktop session ended"
                    );
                    self.completed.push(session.clone());
                    self.state = SessionState::Inactive;
                    Some(session)
                } else {
                    // Still active — update last_active
                    self.state = SessionState::Active {
                        start: *start,
                        last_active: now,
                    };
                    None
                }
            }
        }
    }

    /// Drain completed sessions (returns and clears them).
    pub fn drain_completed(&mut self) -> Vec<CompletedSession> {
        std::mem::take(&mut self.completed)
    }
}

/// Desktop Activity source — tracks computer usage sessions.
pub struct DesktopActivitySource {
    activity_state: Mutex<DesktopActivityState>,
}

impl Default for DesktopActivitySource {
    fn default() -> Self {
        Self::new()
    }
}

impl DesktopActivitySource {
    pub fn new() -> Self {
        Self {
            activity_state: Mutex::new(DesktopActivityState::new()),
        }
    }
}

impl Source for DesktopActivitySource {
    fn id(&self) -> &str {
        "desktop-activity"
    }

    fn name(&self) -> &str {
        "Desktop Activity"
    }

    fn watch_path(&self) -> Option<PathBuf> {
        None // Non-file source — uses polling worker instead
    }

    fn parse(&self) -> Result<serde_json::Value, SourceError> {
        let mut state = self.activity_state.lock().unwrap();
        let sessions = state.drain_completed();

        if sessions.is_empty() {
            return Ok(serde_json::json!({
                "type": "desktop_activity",
                "sessions": [],
                "metadata": {
                    "source": "localpush",
                    "source_id": "desktop-activity",
                    "generated_at": Utc::now().to_rfc3339(),
                }
            }));
        }

        Ok(serde_json::json!({
            "type": "desktop_activity",
            "sessions": sessions,
            "session_count": sessions.len(),
            "total_minutes": sessions.iter().map(|s| s.duration_minutes).sum::<f64>(),
            "metadata": {
                "source": "localpush",
                "source_id": "desktop-activity",
                "generated_at": Utc::now().to_rfc3339(),
            }
        }))
    }

    fn preview(&self) -> Result<SourcePreview, SourceError> {
        let state = self.activity_state.lock().unwrap();

        let mut fields = Vec::new();

        // Current session status
        match &state.state {
            SessionState::Inactive => {
                fields.push(PreviewField {
                    label: "Status".to_string(),
                    value: "Inactive (no active session)".to_string(),
                    sensitive: false,
                });
            }
            SessionState::Active { start, last_active } => {
                let duration = (*last_active - *start).num_minutes();
                fields.push(PreviewField {
                    label: "Status".to_string(),
                    value: format!("Active session ({} min)", duration),
                    sensitive: false,
                });
                fields.push(PreviewField {
                    label: "Session Start".to_string(),
                    value: start.format("%H:%M").to_string(),
                    sensitive: false,
                });
            }
        }

        // Recent completed sessions
        let recent: Vec<_> = state.completed.iter().rev().take(5).collect();
        if !recent.is_empty() {
            fields.push(PreviewField {
                label: "Recent Sessions".to_string(),
                value: format!("{} completed", state.completed.len()),
                sensitive: false,
            });
            for session in recent {
                let start = DateTime::from_timestamp(session.start_timestamp, 0)
                    .map(|dt| dt.format("%H:%M").to_string())
                    .unwrap_or_else(|| "?".to_string());
                let end = DateTime::from_timestamp(session.end_timestamp, 0)
                    .map(|dt| dt.format("%H:%M").to_string())
                    .unwrap_or_else(|| "?".to_string());
                fields.push(PreviewField {
                    label: "Session".to_string(),
                    value: format!("{} - {} ({:.0} min)", start, end, session.duration_minutes),
                    sensitive: false,
                });
            }
        }

        Ok(SourcePreview {
            title: "Desktop Activity".to_string(),
            summary: "Tracks active computer sessions via keyboard/mouse activity".to_string(),
            fields,
            last_updated: Some(Utc::now()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_trait_impl() {
        let source = DesktopActivitySource::new();
        assert_eq!(source.id(), "desktop-activity");
        assert_eq!(source.name(), "Desktop Activity");
        assert!(source.watch_path().is_none(), "desktop-activity is a non-file source");
    }

    #[test]
    fn test_inactive_to_active_transition() {
        let mut state = DesktopActivityState::new();
        assert_eq!(state.state, SessionState::Inactive);

        // User is active (idle < threshold)
        let session = state.tick(5.0);
        assert!(session.is_none(), "no session completed on activation");
        assert!(matches!(state.state, SessionState::Active { .. }));
    }

    #[test]
    fn test_active_stays_active() {
        let mut state = DesktopActivityState::new();

        // Become active
        state.tick(5.0);

        // Still active
        let session = state.tick(10.0);
        assert!(session.is_none());
        assert!(matches!(state.state, SessionState::Active { .. }));
    }

    #[test]
    fn test_active_to_idle_completes_session() {
        let mut state = DesktopActivityState::new();

        // Become active
        state.tick(1.0);
        assert!(matches!(state.state, SessionState::Active { .. }));

        // Go idle (>= threshold)
        let session = state.tick(IDLE_THRESHOLD_SECS);
        assert!(session.is_some(), "session should complete when idle threshold reached");

        let session = session.unwrap();
        assert!(session.duration_minutes >= 0.0);
        assert_eq!(session.idle_threshold_seconds, IDLE_THRESHOLD_SECS);
        assert_eq!(state.state, SessionState::Inactive);
    }

    #[test]
    fn test_inactive_stays_inactive_when_idle() {
        let mut state = DesktopActivityState::new();

        // Already idle, stays idle
        let session = state.tick(300.0);
        assert!(session.is_none());
        assert_eq!(state.state, SessionState::Inactive);
    }

    #[test]
    fn test_multiple_sessions() {
        let mut state = DesktopActivityState::new();

        // Session 1
        state.tick(1.0); // active
        state.tick(IDLE_THRESHOLD_SECS); // idle → complete

        // Session 2
        state.tick(1.0); // active again
        state.tick(IDLE_THRESHOLD_SECS); // idle → complete

        assert_eq!(state.completed.len(), 2);
    }

    #[test]
    fn test_drain_completed_clears() {
        let mut state = DesktopActivityState::new();

        state.tick(1.0);
        state.tick(IDLE_THRESHOLD_SECS);

        let drained = state.drain_completed();
        assert_eq!(drained.len(), 1);
        assert!(state.completed.is_empty(), "drain should clear completed sessions");
    }

    #[test]
    fn test_parse_empty_sessions() {
        let source = DesktopActivitySource::new();
        let payload = source.parse().unwrap();

        assert_eq!(payload["type"], "desktop_activity");
        assert!(payload["sessions"].as_array().unwrap().is_empty());
        assert!(payload["metadata"]["source"].as_str() == Some("localpush"));
    }

    #[test]
    fn test_preview_inactive() {
        let source = DesktopActivitySource::new();
        let preview = source.preview().unwrap();

        assert_eq!(preview.title, "Desktop Activity");
        assert!(!preview.fields.is_empty());
        assert!(preview.fields[0].value.contains("Inactive"));
    }
}
