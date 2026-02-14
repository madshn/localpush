//! SQLite-based delivery ledger with WAL for guaranteed delivery

use std::path::Path;
use std::sync::Mutex;
use rusqlite::{Connection, params};
use crate::traits::{DeliveryLedgerTrait, DeliveryEntry, DeliveryStatus, LedgerError, LedgerStats};

pub struct DeliveryLedger {
    conn: Mutex<Connection>,
}

impl DeliveryLedger {
    /// Open or create a ledger database
    pub fn open(path: &Path) -> Result<Self, LedgerError> {
        let conn = Connection::open(path)
            .map_err(|e| LedgerError::DatabaseError(e.to_string()))?;

        // Enable WAL mode for crash recovery
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA wal_autocheckpoint = 1000;
             PRAGMA busy_timeout = 5000;"
        ).map_err(|e| LedgerError::DatabaseError(e.to_string()))?;

        // Create tables
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS delivery_ledger (
                id TEXT PRIMARY KEY,
                event_id TEXT NOT NULL UNIQUE,
                event_type TEXT NOT NULL,
                payload TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'pending',
                retry_count INTEGER NOT NULL DEFAULT 0,
                max_retries INTEGER NOT NULL DEFAULT 5,
                last_error TEXT,
                available_at INTEGER NOT NULL,
                created_at INTEGER NOT NULL,
                delivered_at INTEGER
            );

            CREATE INDEX IF NOT EXISTS idx_ledger_status
                ON delivery_ledger (status, available_at);

            CREATE INDEX IF NOT EXISTS idx_ledger_delivered
                ON delivery_ledger (delivered_at)
                WHERE status = 'delivered';"
        ).map_err(|e| LedgerError::DatabaseError(e.to_string()))?;

        // Idempotent migration: add target_endpoint_id column
        let _ = conn.execute_batch(
            "ALTER TABLE delivery_ledger ADD COLUMN target_endpoint_id TEXT DEFAULT NULL;"
        ); // Ignore error if column already exists

        // Idempotent migration: add retry_log column (JSON array of retry attempts)
        let _ = conn.execute_batch(
            "ALTER TABLE delivery_ledger ADD COLUMN retry_log TEXT DEFAULT '[]';"
        ); // Ignore error if column already exists

        // Idempotent migration: add trigger_type column (file_change, manual, scheduled)
        let _ = conn.execute_batch(
            "ALTER TABLE delivery_ledger ADD COLUMN trigger_type TEXT DEFAULT 'file_change';"
        );

        // Idempotent migration: add delivered_to column (JSON: endpoint_id, endpoint_name, target_type)
        let _ = conn.execute_batch(
            "ALTER TABLE delivery_ledger ADD COLUMN delivered_to TEXT DEFAULT NULL;"
        );

        Ok(Self { conn: Mutex::new(conn) })
    }

    /// Open an in-memory database (for testing)
    pub fn open_in_memory() -> Result<Self, LedgerError> {
        let conn = Connection::open_in_memory()
            .map_err(|e| LedgerError::DatabaseError(e.to_string()))?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS delivery_ledger (
                id TEXT PRIMARY KEY,
                event_id TEXT NOT NULL UNIQUE,
                event_type TEXT NOT NULL,
                payload TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'pending',
                retry_count INTEGER NOT NULL DEFAULT 0,
                max_retries INTEGER NOT NULL DEFAULT 5,
                last_error TEXT,
                available_at INTEGER NOT NULL,
                created_at INTEGER NOT NULL,
                delivered_at INTEGER,
                target_endpoint_id TEXT DEFAULT NULL,
                retry_log TEXT DEFAULT '[]',
                trigger_type TEXT DEFAULT 'file_change',
                delivered_to TEXT DEFAULT NULL
            );"
        ).map_err(|e| LedgerError::DatabaseError(e.to_string()))?;

        Ok(Self { conn: Mutex::new(conn) })
    }
}

impl DeliveryLedgerTrait for DeliveryLedger {
    fn enqueue(&self, event_type: &str, payload: serde_json::Value) -> Result<String, LedgerError> {
        let id = uuid::Uuid::new_v4().to_string();
        let event_id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().timestamp();
        let payload_str = serde_json::to_string(&payload)
            .map_err(|e| LedgerError::DatabaseError(e.to_string()))?;

        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO delivery_ledger (id, event_id, event_type, payload, available_at, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
            params![id, event_id, event_type, payload_str, now],
        ).map_err(|e| LedgerError::DatabaseError(e.to_string()))?;

        tracing::debug!("Enqueued delivery: {} ({})", event_id, event_type);
        Ok(event_id)
    }

    fn enqueue_targeted(
        &self,
        event_type: &str,
        payload: serde_json::Value,
        target_endpoint_id: &str,
    ) -> Result<String, LedgerError> {
        let id = uuid::Uuid::new_v4().to_string();
        let event_id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().timestamp();
        let payload_str = serde_json::to_string(&payload)
            .map_err(|e| LedgerError::DatabaseError(e.to_string()))?;

        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO delivery_ledger (id, event_id, event_type, payload, available_at, created_at, target_endpoint_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?5, ?6)",
            params![id, event_id, event_type, payload_str, now, target_endpoint_id],
        ).map_err(|e| LedgerError::DatabaseError(e.to_string()))?;

        tracing::debug!("Enqueued targeted delivery: {} ({}) -> {}", event_id, event_type, target_endpoint_id);
        Ok(event_id)
    }

    fn enqueue_manual(&self, event_type: &str, payload: serde_json::Value) -> Result<String, LedgerError> {
        let id = uuid::Uuid::new_v4().to_string();
        let event_id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().timestamp();
        let payload_str = serde_json::to_string(&payload)
            .map_err(|e| LedgerError::DatabaseError(e.to_string()))?;

        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO delivery_ledger (id, event_id, event_type, payload, available_at, created_at, trigger_type)
             VALUES (?1, ?2, ?3, ?4, ?5, ?5, 'manual')",
            params![id, event_id, event_type, payload_str, now],
        ).map_err(|e| LedgerError::DatabaseError(e.to_string()))?;

        tracing::debug!("Enqueued manual delivery: {} ({})", event_id, event_type);
        Ok(event_id)
    }

    fn claim_batch(&self, limit: usize) -> Result<Vec<DeliveryEntry>, LedgerError> {
        let now = chrono::Utc::now().timestamp();

        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, event_id, event_type, payload, status, retry_count, max_retries,
                    last_error, available_at, created_at, delivered_at, target_endpoint_id,
                    trigger_type, delivered_to
             FROM delivery_ledger
             WHERE status IN ('pending', 'failed') AND available_at <= ?1
             ORDER BY available_at ASC
             LIMIT ?2"
        ).map_err(|e| LedgerError::DatabaseError(e.to_string()))?;

        let entries: Vec<DeliveryEntry> = stmt.query_map(params![now, limit], |row| {
            let status_str: String = row.get(4)?;
            let status = match status_str.as_str() {
                "pending" => DeliveryStatus::Pending,
                "in_flight" => DeliveryStatus::InFlight,
                "delivered" => DeliveryStatus::Delivered,
                "failed" => DeliveryStatus::Failed,
                "dlq" => DeliveryStatus::Dlq,
                _ => DeliveryStatus::Pending,
            };

            let payload_str: String = row.get(3)?;
            let payload: serde_json::Value = serde_json::from_str(&payload_str)
                .unwrap_or(serde_json::Value::Null);

            Ok(DeliveryEntry {
                id: row.get(0)?,
                event_id: row.get(1)?,
                event_type: row.get(2)?,
                payload,
                status,
                retry_count: row.get(5)?,
                max_retries: row.get(6)?,
                last_error: row.get(7)?,
                available_at: row.get(8)?,
                created_at: row.get(9)?,
                delivered_at: row.get(10)?,
                target_endpoint_id: row.get(11)?,
                trigger_type: row.get(12)?,
                delivered_to: row.get(13)?,
            })
        }).map_err(|e| LedgerError::DatabaseError(e.to_string()))?
        .filter_map(Result::ok)
        .collect();

        // Mark claimed entries as in_flight
        for entry in &entries {
            conn.execute(
                "UPDATE delivery_ledger SET status = 'in_flight' WHERE id = ?1",
                params![entry.id],
            ).map_err(|e| LedgerError::DatabaseError(e.to_string()))?;
        }

        // Return entries with updated status
        Ok(entries.into_iter().map(|mut e| {
            e.status = DeliveryStatus::InFlight;
            e
        }).collect())
    }

    fn mark_delivered(&self, event_id: &str, delivered_to: Option<String>) -> Result<(), LedgerError> {
        let now = chrono::Utc::now().timestamp();

        let conn = self.conn.lock().unwrap();
        let rows = conn.execute(
            "UPDATE delivery_ledger
             SET status = 'delivered', delivered_at = ?1, delivered_to = ?3
             WHERE event_id = ?2 AND status = 'in_flight'",
            params![now, event_id, delivered_to.as_deref()],
        ).map_err(|e| LedgerError::DatabaseError(e.to_string()))?;

        if rows == 0 {
            return Err(LedgerError::NotFound(event_id.to_string()));
        }

        tracing::info!("Delivery confirmed: {}", event_id);
        Ok(())
    }

    fn mark_failed(&self, event_id: &str, error: &str) -> Result<DeliveryStatus, LedgerError> {
        let now = chrono::Utc::now().timestamp();

        let conn = self.conn.lock().unwrap();
        // Get current retry count, max, and retry_log
        let (retry_count, max_retries, retry_log_str): (u32, u32, String) = conn.query_row(
            "SELECT retry_count, max_retries, COALESCE(retry_log, '[]') FROM delivery_ledger WHERE event_id = ?1",
            params![event_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        ).map_err(|e| LedgerError::DatabaseError(e.to_string()))?;

        let new_retry_count = retry_count + 1;

        let (new_status, next_available) = if new_retry_count >= max_retries {
            (DeliveryStatus::Dlq, now)
        } else {
            // Exponential backoff: 1s, 2s, 4s, 8s, 16s...
            let delay = (1 << new_retry_count).min(3600); // Max 1 hour
            (DeliveryStatus::Failed, now + delay as i64)
        };

        // Append to retry_log
        let mut retry_log: Vec<serde_json::Value> = serde_json::from_str(&retry_log_str)
            .unwrap_or_default();
        retry_log.push(serde_json::json!({
            "at": now,
            "error": error,
            "attempt": new_retry_count
        }));
        let new_retry_log_str = serde_json::to_string(&retry_log)
            .unwrap_or_else(|_| "[]".to_string());

        conn.execute(
            "UPDATE delivery_ledger
             SET status = ?1, retry_count = ?2, last_error = ?3, available_at = ?4, retry_log = ?5
             WHERE event_id = ?6",
            params![new_status.as_str(), new_retry_count, error, next_available, new_retry_log_str, event_id],
        ).map_err(|e| LedgerError::DatabaseError(e.to_string()))?;

        tracing::warn!("Delivery failed: {} (attempt {}/{}): {}",
            event_id, new_retry_count, max_retries, error);

        Ok(new_status)
    }

    fn get_by_status(&self, status: DeliveryStatus) -> Result<Vec<DeliveryEntry>, LedgerError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, event_id, event_type, payload, status, retry_count, max_retries,
                    last_error, available_at, created_at, delivered_at, target_endpoint_id,
                    trigger_type, delivered_to
             FROM delivery_ledger
             WHERE status = ?1
             ORDER BY created_at DESC
             LIMIT 100"
        ).map_err(|e| LedgerError::DatabaseError(e.to_string()))?;

        let entries = stmt.query_map(params![status.as_str()], |row| {
            let payload_str: String = row.get(3)?;
            let payload: serde_json::Value = serde_json::from_str(&payload_str)
                .unwrap_or(serde_json::Value::Null);

            Ok(DeliveryEntry {
                id: row.get(0)?,
                event_id: row.get(1)?,
                event_type: row.get(2)?,
                payload,
                status,
                retry_count: row.get(5)?,
                max_retries: row.get(6)?,
                last_error: row.get(7)?,
                available_at: row.get(8)?,
                created_at: row.get(9)?,
                delivered_at: row.get(10)?,
                target_endpoint_id: row.get(11)?,
                trigger_type: row.get(12)?,
                delivered_to: row.get(13)?,
            })
        }).map_err(|e| LedgerError::DatabaseError(e.to_string()))?
        .filter_map(Result::ok)
        .collect();

        Ok(entries)
    }

    fn get_stats(&self) -> Result<LedgerStats, LedgerError> {
        let today_start = chrono::Utc::now()
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc()
            .timestamp();

        let conn = self.conn.lock().unwrap();
        let stats: LedgerStats = conn.query_row(
            "SELECT
                SUM(CASE WHEN status = 'pending' THEN 1 ELSE 0 END) as pending,
                SUM(CASE WHEN status = 'in_flight' THEN 1 ELSE 0 END) as in_flight,
                SUM(CASE WHEN status = 'delivered' AND delivered_at >= ?1 THEN 1 ELSE 0 END) as delivered_today,
                SUM(CASE WHEN status = 'failed' THEN 1 ELSE 0 END) as failed,
                SUM(CASE WHEN status = 'dlq' THEN 1 ELSE 0 END) as dlq
             FROM delivery_ledger",
            params![today_start],
            |row| {
                Ok(LedgerStats {
                    pending: row.get::<_, i64>(0).unwrap_or(0) as usize,
                    in_flight: row.get::<_, i64>(1).unwrap_or(0) as usize,
                    delivered_today: row.get::<_, i64>(2).unwrap_or(0) as usize,
                    failed: row.get::<_, i64>(3).unwrap_or(0) as usize,
                    dlq: row.get::<_, i64>(4).unwrap_or(0) as usize,
                })
            }
        ).map_err(|e| LedgerError::DatabaseError(e.to_string()))?;

        Ok(stats)
    }

    fn recover_orphans(&self) -> Result<usize, LedgerError> {
        let now = chrono::Utc::now().timestamp();
        let stale_threshold = now - 300; // 5 minutes

        let conn = self.conn.lock().unwrap();
        let rows = conn.execute(
            "UPDATE delivery_ledger
             SET status = 'failed',
                 last_error = 'Recovered from crash - previous attempt status unknown',
                 available_at = ?1
             WHERE status = 'in_flight' AND available_at < ?2",
            params![now, stale_threshold],
        ).map_err(|e| LedgerError::DatabaseError(e.to_string()))?;

        if rows > 0 {
            tracing::warn!("Recovered {} orphaned in-flight entries", rows);
        }

        Ok(rows)
    }

    fn reset_to_pending(&self, event_id: &str) -> Result<(), LedgerError> {
        let now = chrono::Utc::now().timestamp();

        let conn = self.conn.lock().unwrap();
        let rows = conn.execute(
            "UPDATE delivery_ledger
             SET status = 'pending', available_at = ?1, last_error = NULL
             WHERE event_id = ?2 AND status IN ('failed', 'dlq')",
            params![now, event_id],
        ).map_err(|e| LedgerError::DatabaseError(e.to_string()))?;

        if rows == 0 {
            return Err(LedgerError::NotFound(event_id.to_string()));
        }

        tracing::info!("Delivery reset to pending: {}", event_id);
        Ok(())
    }

    fn get_retry_history(&self, entry_id: &str) -> Result<Vec<serde_json::Value>, LedgerError> {
        let conn = self.conn.lock().unwrap();

        // Query retry_log column by entry id
        let retry_log_str: String = conn.query_row(
            "SELECT COALESCE(retry_log, '[]') FROM delivery_ledger WHERE id = ?1",
            params![entry_id],
            |row| row.get(0),
        ).map_err(|e| {
            if matches!(e, rusqlite::Error::QueryReturnedNoRows) {
                LedgerError::NotFound(entry_id.to_string())
            } else {
                LedgerError::DatabaseError(e.to_string())
            }
        })?;

        // Parse JSON array
        let retry_log: Vec<serde_json::Value> = serde_json::from_str(&retry_log_str)
            .map_err(|e| LedgerError::DatabaseError(format!("Failed to parse retry_log: {}", e)))?;

        Ok(retry_log)
    }

    fn dismiss_dlq(&self, event_id: &str) -> Result<(), LedgerError> {
        let now = chrono::Utc::now().timestamp();
        let conn = self.conn.lock().unwrap();
        let rows = conn.execute(
            "UPDATE delivery_ledger
             SET status = 'delivered', delivered_at = ?1
             WHERE event_id = ?2 AND status = 'dlq'",
            params![now, event_id],
        ).map_err(|e| LedgerError::DatabaseError(e.to_string()))?;

        if rows == 0 {
            return Err(LedgerError::NotFound(event_id.to_string()));
        }

        tracing::info!("DLQ entry dismissed: {}", event_id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enqueue_and_claim() {
        let ledger = DeliveryLedger::open_in_memory().unwrap();

        // Enqueue
        let event_id = ledger.enqueue(
            "test.event",
            serde_json::json!({"key": "value"})
        ).unwrap();

        assert!(!event_id.is_empty());

        // Claim
        let batch = ledger.claim_batch(10).unwrap();
        assert_eq!(batch.len(), 1);
        assert_eq!(batch[0].event_type, "test.event");
        assert_eq!(batch[0].status, DeliveryStatus::InFlight);
    }

    #[test]
    fn test_delivery_success() {
        let ledger = DeliveryLedger::open_in_memory().unwrap();

        let event_id = ledger.enqueue("test.event", serde_json::json!({})).unwrap();
        ledger.claim_batch(1).unwrap();

        ledger.mark_delivered(&event_id, None).unwrap();

        let delivered = ledger.get_by_status(DeliveryStatus::Delivered).unwrap();
        assert_eq!(delivered.len(), 1);
    }

    #[test]
    fn test_retry_with_backoff() {
        let ledger = DeliveryLedger::open_in_memory().unwrap();

        let event_id = ledger.enqueue("test.event", serde_json::json!({})).unwrap();
        ledger.claim_batch(1).unwrap();

        // First failure
        let status = ledger.mark_failed(&event_id, "Connection refused").unwrap();
        assert_eq!(status, DeliveryStatus::Failed);

        // Check retry count increased
        let failed = ledger.get_by_status(DeliveryStatus::Failed).unwrap();
        assert_eq!(failed[0].retry_count, 1);
    }

    #[test]
    fn test_dlq_after_max_retries() {
        let ledger = DeliveryLedger::open_in_memory().unwrap();

        let event_id = ledger.enqueue("test.event", serde_json::json!({})).unwrap();

        // Simulate 5 failures (default max_retries)
        for i in 0..5 {
            ledger.claim_batch(1).unwrap();
            let status = ledger.mark_failed(&event_id, &format!("Error {}", i)).unwrap();

            if i < 4 {
                assert_eq!(status, DeliveryStatus::Failed);
            } else {
                assert_eq!(status, DeliveryStatus::Dlq);
            }
        }
    }

    #[test]
    fn test_get_retry_history() {
        let ledger = DeliveryLedger::open_in_memory().unwrap();

        let event_id = ledger.enqueue("test.event", serde_json::json!({})).unwrap();

        // Get the entry ID
        let entries = ledger.claim_batch(1).unwrap();
        let entry_id = entries[0].id.clone();

        // Initial retry history should be empty
        let history = ledger.get_retry_history(&entry_id).unwrap();
        assert_eq!(history.len(), 0);

        // Fail twice
        ledger.mark_failed(&event_id, "Connection refused").unwrap();
        ledger.claim_batch(1).unwrap();
        ledger.mark_failed(&event_id, "Timeout").unwrap();

        // Should have 2 entries in retry history
        let history = ledger.get_retry_history(&entry_id).unwrap();
        assert_eq!(history.len(), 2);

        // Verify structure
        assert!(history[0].get("at").is_some());
        assert_eq!(history[0].get("error").unwrap().as_str().unwrap(), "Connection refused");
        assert_eq!(history[0].get("attempt").unwrap().as_u64().unwrap(), 1);

        assert_eq!(history[1].get("error").unwrap().as_str().unwrap(), "Timeout");
        assert_eq!(history[1].get("attempt").unwrap().as_u64().unwrap(), 2);
    }

    #[test]
    fn test_get_retry_history_not_found() {
        let ledger = DeliveryLedger::open_in_memory().unwrap();
        let result = ledger.get_retry_history("nonexistent-id");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), LedgerError::NotFound(_)));
    }
}
