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
}

impl DeliveryStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            DeliveryStatus::Pending => "pending",
            DeliveryStatus::InFlight => "in_flight",
            DeliveryStatus::Delivered => "delivered",
            DeliveryStatus::Failed => "failed",
            DeliveryStatus::Dlq => "dlq",
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
}

/// Trait for delivery ledger operations
///
/// Production: SQLite with WAL mode
/// Testing: In-memory storage
#[cfg_attr(test, mockall::automock)]
pub trait DeliveryLedgerTrait: Send + Sync {
    /// Enqueue a new delivery
    fn enqueue(
        &self,
        event_type: &str,
        payload: serde_json::Value,
    ) -> Result<String, LedgerError>;

    /// Claim a batch of pending deliveries for processing
    fn claim_batch(&self, limit: usize) -> Result<Vec<DeliveryEntry>, LedgerError>;

    /// Mark a delivery as successfully completed
    fn mark_delivered(&self, event_id: &str) -> Result<(), LedgerError>;

    /// Mark a delivery as failed (will retry or move to DLQ)
    fn mark_failed(&self, event_id: &str, error: &str) -> Result<DeliveryStatus, LedgerError>;

    /// Get entries by status
    fn get_by_status(&self, status: DeliveryStatus) -> Result<Vec<DeliveryEntry>, LedgerError>;

    /// Get queue statistics
    fn get_stats(&self) -> Result<LedgerStats, LedgerError>;

    /// Recover orphaned in-flight entries on startup
    fn recover_orphans(&self) -> Result<usize, LedgerError>;
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LedgerStats {
    pub pending: usize,
    pub in_flight: usize,
    pub delivered_today: usize,
    pub failed: usize,
    pub dlq: usize,
}
