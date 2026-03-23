//! Backend-to-frontend event constants for Tauri event push.
//!
//! Instead of polling, the backend emits these events when state changes.
//! The frontend listens and invalidates relevant React Query caches.

pub const DELIVERY_STATUS_CHANGED: &str = "delivery:status-changed";
pub const SOURCE_DATA_UPDATED: &str = "source:data-updated";
pub const DLQ_CHANGED: &str = "dlq:changed";
pub const TARGET_HEALTH_CHANGED: &str = "target:health-changed";
