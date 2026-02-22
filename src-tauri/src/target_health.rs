//! Per-target health tracking with automatic degradation.
//!
//! Tracks delivery failures per target and transitions targets between
//! Healthy and Degraded states. Auth/token errors degrade immediately;
//! transient errors degrade after 3 consecutive failures.

use std::collections::HashMap;
use std::sync::Mutex;
use serde::Serialize;
use crate::traits::TargetError;

/// Health state for a single target.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum TargetHealthState {
    Healthy,
    Degraded {
        reason: String,
        degraded_at: i64,
    },
}

/// Internal tracking data per target.
#[derive(Debug)]
struct TargetHealthEntry {
    state: TargetHealthState,
    consecutive_failures: u32,
}

impl Default for TargetHealthEntry {
    fn default() -> Self {
        Self {
            state: TargetHealthState::Healthy,
            consecutive_failures: 0,
        }
    }
}

/// Info returned to callers about a degraded target.
#[derive(Debug, Clone, Serialize)]
pub struct DegradationInfo {
    pub target_id: String,
    pub reason: String,
    pub degraded_at: i64,
}

/// Tracks health state per target, with automatic degradation on failures.
#[derive(Default)]
pub struct TargetHealthTracker {
    entries: Mutex<HashMap<String, TargetHealthEntry>>,
}

impl TargetHealthTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Report a delivery failure for a target. Returns true if this failure
    /// caused a state transition to Degraded (caller should pause deliveries).
    pub fn report_failure(&self, target_id: &str, error: &TargetError) -> bool {
        let now = chrono::Utc::now().timestamp();
        let mut entries = self.entries.lock().unwrap();
        let entry = entries.entry(target_id.to_string()).or_default();

        // Already degraded — no transition
        if matches!(entry.state, TargetHealthState::Degraded { .. }) {
            return false;
        }

        entry.consecutive_failures += 1;

        let should_degrade = match error {
            // Auth errors → immediate degradation
            TargetError::TokenExpired | TargetError::AuthFailed(_) => true,
            // Transient errors → degrade after 3 consecutive failures
            TargetError::ConnectionFailed(_) | TargetError::DeliveryError(_) => {
                entry.consecutive_failures >= 3
            }
            // Config errors and not-connected don't trigger degradation
            TargetError::InvalidConfig(_) | TargetError::NotConnected => false,
        };

        if should_degrade {
            let reason = format!("{}", error);
            tracing::warn!(
                target_id = %target_id,
                reason = %reason,
                consecutive_failures = entry.consecutive_failures,
                "Target degraded"
            );
            entry.state = TargetHealthState::Degraded {
                reason,
                degraded_at: now,
            };
            true
        } else {
            false
        }
    }

    /// Report a successful delivery — resets consecutive failure count.
    pub fn report_success(&self, target_id: &str) {
        let mut entries = self.entries.lock().unwrap();
        if let Some(entry) = entries.get_mut(target_id) {
            entry.consecutive_failures = 0;
            // Note: success does NOT transition from Degraded → Healthy.
            // Only explicit reconnect does that.
        }
    }

    /// Check if a target is degraded. Returns degradation info if so.
    pub fn is_degraded(&self, target_id: &str) -> Option<DegradationInfo> {
        let entries = self.entries.lock().unwrap();
        entries.get(target_id).and_then(|entry| {
            if let TargetHealthState::Degraded { reason, degraded_at } = &entry.state {
                Some(DegradationInfo {
                    target_id: target_id.to_string(),
                    reason: reason.clone(),
                    degraded_at: *degraded_at,
                })
            } else {
                None
            }
        })
    }

    /// Mark a target as healthy after successful reconnection.
    pub fn mark_reconnected(&self, target_id: &str) {
        let mut entries = self.entries.lock().unwrap();
        if let Some(entry) = entries.get_mut(target_id) {
            tracing::info!(target_id = %target_id, "Target reconnected — marking healthy");
            entry.state = TargetHealthState::Healthy;
            entry.consecutive_failures = 0;
        }
    }

    /// Get all currently degraded targets.
    pub fn get_all_degraded(&self) -> Vec<DegradationInfo> {
        let entries = self.entries.lock().unwrap();
        entries.iter().filter_map(|(target_id, entry)| {
            if let TargetHealthState::Degraded { reason, degraded_at } = &entry.state {
                Some(DegradationInfo {
                    target_id: target_id.clone(),
                    reason: reason.clone(),
                    degraded_at: *degraded_at,
                })
            } else {
                None
            }
        }).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_healthy_by_default() {
        let tracker = TargetHealthTracker::new();
        assert!(tracker.is_degraded("t1").is_none());
    }

    #[test]
    fn test_auth_error_immediate_degradation() {
        let tracker = TargetHealthTracker::new();
        let transitioned = tracker.report_failure("t1", &TargetError::TokenExpired);
        assert!(transitioned);
        assert!(tracker.is_degraded("t1").is_some());
        assert_eq!(tracker.is_degraded("t1").unwrap().reason, "Token expired");
    }

    #[test]
    fn test_auth_failed_immediate_degradation() {
        let tracker = TargetHealthTracker::new();
        let transitioned = tracker.report_failure("t1", &TargetError::AuthFailed("bad creds".into()));
        assert!(transitioned);
        assert!(tracker.is_degraded("t1").is_some());
    }

    #[test]
    fn test_transient_error_degrades_after_three() {
        let tracker = TargetHealthTracker::new();
        let err = TargetError::ConnectionFailed("refused".into());

        assert!(!tracker.report_failure("t1", &err)); // 1st
        assert!(tracker.is_degraded("t1").is_none());

        assert!(!tracker.report_failure("t1", &err)); // 2nd
        assert!(tracker.is_degraded("t1").is_none());

        assert!(tracker.report_failure("t1", &err));  // 3rd → degraded
        assert!(tracker.is_degraded("t1").is_some());
    }

    #[test]
    fn test_success_resets_consecutive_failures() {
        let tracker = TargetHealthTracker::new();
        let err = TargetError::ConnectionFailed("refused".into());

        tracker.report_failure("t1", &err); // 1st
        tracker.report_failure("t1", &err); // 2nd
        tracker.report_success("t1");       // resets

        // Next failure starts from 1 again
        assert!(!tracker.report_failure("t1", &err)); // 1st (reset)
        assert!(tracker.is_degraded("t1").is_none());
    }

    #[test]
    fn test_reconnect_restores_health() {
        let tracker = TargetHealthTracker::new();
        tracker.report_failure("t1", &TargetError::TokenExpired);
        assert!(tracker.is_degraded("t1").is_some());

        tracker.mark_reconnected("t1");
        assert!(tracker.is_degraded("t1").is_none());
    }

    #[test]
    fn test_already_degraded_no_double_transition() {
        let tracker = TargetHealthTracker::new();
        assert!(tracker.report_failure("t1", &TargetError::TokenExpired));
        // Second failure on already-degraded target should NOT trigger transition
        assert!(!tracker.report_failure("t1", &TargetError::TokenExpired));
    }

    #[test]
    fn test_get_all_degraded() {
        let tracker = TargetHealthTracker::new();
        tracker.report_failure("t1", &TargetError::TokenExpired);
        tracker.report_failure("t2", &TargetError::AuthFailed("bad".into()));
        // t3 is healthy
        let err = TargetError::ConnectionFailed("refused".into());
        tracker.report_failure("t3", &err); // only 1 failure

        let degraded = tracker.get_all_degraded();
        assert_eq!(degraded.len(), 2);
    }

    #[test]
    fn test_config_error_does_not_degrade() {
        let tracker = TargetHealthTracker::new();
        let err = TargetError::InvalidConfig("bad url".into());
        assert!(!tracker.report_failure("t1", &err));
        assert!(!tracker.report_failure("t1", &err));
        assert!(!tracker.report_failure("t1", &err));
        assert!(tracker.is_degraded("t1").is_none());
    }

    #[test]
    fn test_independent_targets() {
        let tracker = TargetHealthTracker::new();
        tracker.report_failure("t1", &TargetError::TokenExpired);
        assert!(tracker.is_degraded("t1").is_some());
        assert!(tracker.is_degraded("t2").is_none()); // t2 unaffected
    }
}
