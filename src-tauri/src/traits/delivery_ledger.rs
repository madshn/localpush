//! Delivery ledger trait for guaranteed delivery

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LedgerError {
    #[error("Database error: {0}")]
    DatabaseError(String),
    #[error("Entry not found: {0}")]
    NotFound(String),
    #[error("Invalid state transition")]
    InvalidStateTransition,
}

/// Status of a delivery entry
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryStatus {
    Pending,
    InFlight,
    Delivered,
    Failed,
    Dlq, // Dead Letter Queue
    TargetPaused, // Target is degraded — delivery queued until reconnect
}

impl DeliveryStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            DeliveryStatus::Pending => "pending",
            DeliveryStatus::InFlight => "in_flight",
            DeliveryStatus::Delivered => "delivered",
            DeliveryStatus::Failed => "failed",
            DeliveryStatus::Dlq => "dlq",
            DeliveryStatus::TargetPaused => "target_paused",
        }
    }
}

/// A delivery entry in the ledger
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryEntry {
    pub id: String,
    pub event_id: String,
    pub event_type: String,
    pub payload: serde_json::Value,
    pub status: DeliveryStatus,
    pub retry_count: u32,
    pub max_retries: u32,
    pub last_error: Option<String>,
    pub available_at: i64,
    pub created_at: i64,
    pub delivered_at: Option<i64>,
    /// When set, deliver only to this specific endpoint (for scheduled deliveries)
    #[serde(default)]
    pub target_endpoint_id: Option<String>,
    /// How this entry was triggered: "file_change" (default), "manual", or "scheduled"
    #[serde(default)]
    pub trigger_type: Option<String>,
    /// JSON string describing which target received the delivery (set after successful POST)
    #[serde(default)]
    pub delivered_to: Option<String>,
}

/// Trait for delivery ledger operations
///
/// Production: SQLite with WAL mode
/// Testing: In-memory storage
pub trait DeliveryLedgerTrait: Send + Sync {
    /// Enqueue a new delivery
    fn enqueue(
        &self,
        event_type: &str,
        payload: serde_json::Value,
    ) -> Result<String, LedgerError>;

    /// Enqueue a targeted delivery (for a specific endpoint only)
    fn enqueue_targeted(
        &self,
        event_type: &str,
        payload: serde_json::Value,
        target_endpoint_id: &str,
    ) -> Result<String, LedgerError>;

    /// Enqueue a manual push (trigger_type = "manual")
    fn enqueue_manual(
        &self,
        event_type: &str,
        payload: serde_json::Value,
    ) -> Result<String, LedgerError>;

    /// Enqueue a manual push targeted to a specific endpoint
    fn enqueue_manual_targeted(
        &self,
        event_type: &str,
        payload: serde_json::Value,
        target_endpoint_id: &str,
    ) -> Result<String, LedgerError>;

    /// Enqueue a targeted delivery with a custom available_at timestamp.
    /// Used by coalescing flush to stagger deliveries across targets.
    fn enqueue_targeted_at(
        &self,
        event_type: &str,
        payload: serde_json::Value,
        target_endpoint_id: &str,
        available_at: i64,
    ) -> Result<String, LedgerError>;

    /// Claim a batch of pending deliveries for processing
    fn claim_batch(&self, limit: usize) -> Result<Vec<DeliveryEntry>, LedgerError>;

    /// Mark a delivery as successfully completed, optionally recording which target received it
    fn mark_delivered(&self, event_id: &str, delivered_to: Option<String>) -> Result<(), LedgerError>;

    /// Mark a delivery as failed (will retry or move to DLQ)
    fn mark_failed(&self, event_id: &str, error: &str) -> Result<DeliveryStatus, LedgerError>;

    /// Get entries by status
    fn get_by_status(&self, status: DeliveryStatus) -> Result<Vec<DeliveryEntry>, LedgerError>;

    /// Get queue statistics
    fn get_stats(&self) -> Result<LedgerStats, LedgerError>;

    /// Recover orphaned in-flight entries on startup
    fn recover_orphans(&self) -> Result<usize, LedgerError>;

    /// Reset a failed/dlq entry back to pending for manual retry
    fn reset_to_pending(&self, event_id: &str) -> Result<(), LedgerError>;

    /// Get retry history for a specific entry
    fn get_retry_history(&self, entry_id: &str) -> Result<Vec<serde_json::Value>, LedgerError>;

    /// Dismiss a DLQ entry (marks dlq → delivered as "handled")
    fn dismiss_dlq(&self, event_id: &str) -> Result<(), LedgerError>;

    /// Record which target was attempted (so the UI can show it even on failure)
    fn set_attempted_target(&self, event_id: &str, target_json: &str) -> Result<(), LedgerError>;

    /// Mark a single in-flight entry as target_paused (skipped due to degraded target).
    /// Unlike mark_failed, this does NOT increment retry count.
    fn mark_target_paused(&self, event_id: &str, reason: &str) -> Result<(), LedgerError>;

    /// Pause all pending/failed deliveries targeting any of the given endpoint IDs.
    /// Called when a target degrades — entries move to `target_paused` status.
    fn pause_target_deliveries(&self, endpoint_ids: &[&str]) -> Result<usize, LedgerError>;

    /// Resume paused deliveries for the given endpoint IDs back to pending.
    /// Called when a degraded target reconnects successfully.
    fn resume_target_deliveries(&self, endpoint_ids: &[&str]) -> Result<usize, LedgerError>;

    /// Count deliveries paused for any of the given endpoint IDs.
    fn count_paused_for_target(&self, endpoint_ids: &[&str]) -> Result<usize, LedgerError>;
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LedgerStats {
    pub pending: usize,
    pub in_flight: usize,
    pub delivered_today: usize,
    pub failed: usize,
    pub dlq: usize,
    pub target_paused: usize,
}
