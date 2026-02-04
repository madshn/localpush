//! Background delivery worker for webhook dispatch
//!
//! This module provides the background loop that polls the delivery ledger and
//! dispatches pending entries via webhook. It reads configuration from AppConfig
//! and uses exponential backoff for retries.

use std::sync::Arc;
use std::time::Duration;
use crate::config::AppConfig;
use crate::traits::{DeliveryLedgerTrait, WebhookClient, WebhookAuth};

/// Worker configuration derived from AppConfig
pub struct WorkerConfig {
    pub webhook_url: String,
    pub webhook_auth: WebhookAuth,
}

/// Process one batch of deliveries. Pure function, fully testable.
///
/// Returns (delivered_count, failed_count).
pub async fn process_batch(
    ledger: &dyn DeliveryLedgerTrait,
    webhook: &dyn WebhookClient,
    config: &WorkerConfig,
    batch_size: usize,
) -> (usize, usize) {
    let entries = match ledger.claim_batch(batch_size) {
        Ok(entries) => entries,
        Err(e) => {
            tracing::error!("Failed to claim batch: {}", e);
            return (0, 0);
        }
    };

    let mut delivered = 0;
    let mut failed = 0;

    for entry in entries {
        match webhook.send(&config.webhook_url, &entry.payload, &config.webhook_auth).await {
            Ok(_) => {
                if ledger.mark_delivered(&entry.event_id).is_ok() {
                    delivered += 1;
                }
            }
            Err(e) => {
                let _ = ledger.mark_failed(&entry.event_id, &e.to_string());
                failed += 1;
            }
        }
    }

    if delivered > 0 || failed > 0 {
        tracing::info!("Delivery batch: {} delivered, {} failed", delivered, failed);
    }

    (delivered, failed)
}

/// Read webhook config from AppConfig. Returns None if not configured.
pub fn read_worker_config(config: &AppConfig) -> Option<WorkerConfig> {
    let url = config.get("webhook_url").ok()??;
    let auth_json = config.get("webhook_auth_json").ok()?;
    let auth = match auth_json {
        Some(json) => serde_json::from_str(&json).unwrap_or(WebhookAuth::None),
        None => WebhookAuth::None,
    };
    Some(WorkerConfig {
        webhook_url: url,
        webhook_auth: auth,
    })
}

/// Spawn the background delivery loop. Returns JoinHandle for shutdown.
///
/// The worker polls every 5 seconds, reading config from AppConfig and processing
/// pending entries.
pub fn spawn_worker(
    ledger: Arc<dyn DeliveryLedgerTrait>,
    webhook: Arc<dyn WebhookClient>,
    config: Arc<AppConfig>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        tracing::info!("Delivery worker started (5s interval)");
        let mut interval = tokio::time::interval(Duration::from_secs(5));
        loop {
            interval.tick().await;
            if let Some(worker_config) = read_worker_config(&config) {
                process_batch(&*ledger, &*webhook, &worker_config, 10).await;
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mocks::RecordedWebhookClient;
    use crate::DeliveryLedger;
    use crate::traits::DeliveryStatus;

    fn test_config() -> WorkerConfig {
        WorkerConfig {
            webhook_url: "https://example.com/hook".to_string(),
            webhook_auth: WebhookAuth::None,
        }
    }

    #[tokio::test]
    async fn test_delivers_pending_entry() {
        let ledger = DeliveryLedger::open_in_memory().unwrap();
        let webhook = RecordedWebhookClient::success();
        ledger.enqueue("test.event", serde_json::json!({"hello": "world"})).unwrap();

        let (delivered, failed) = process_batch(&ledger, &webhook, &test_config(), 10).await;

        assert_eq!(delivered, 1);
        assert_eq!(failed, 0);
        assert_eq!(webhook.call_count(), 1);
        assert_eq!(ledger.get_by_status(DeliveryStatus::Delivered).unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_marks_failed_on_error() {
        let ledger = DeliveryLedger::open_in_memory().unwrap();
        let webhook = RecordedWebhookClient::always_fail(
            crate::traits::WebhookError::NetworkError("refused".to_string())
        );
        ledger.enqueue("test.event", serde_json::json!({})).unwrap();

        let (delivered, failed) = process_batch(&ledger, &webhook, &test_config(), 10).await;

        assert_eq!(delivered, 0);
        assert_eq!(failed, 1);
        assert_eq!(ledger.get_by_status(DeliveryStatus::Failed).unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_empty_batch_is_noop() {
        let ledger = DeliveryLedger::open_in_memory().unwrap();
        let webhook = RecordedWebhookClient::success();

        let (delivered, failed) = process_batch(&ledger, &webhook, &test_config(), 10).await;

        assert_eq!(delivered, 0);
        assert_eq!(failed, 0);
        assert_eq!(webhook.call_count(), 0);
    }

    #[tokio::test]
    async fn test_processes_multiple_entries() {
        let ledger = DeliveryLedger::open_in_memory().unwrap();
        let webhook = RecordedWebhookClient::success();

        ledger.enqueue("event.a", serde_json::json!({"a": 1})).unwrap();
        ledger.enqueue("event.b", serde_json::json!({"b": 2})).unwrap();
        ledger.enqueue("event.c", serde_json::json!({"c": 3})).unwrap();

        let (delivered, _failed) = process_batch(&ledger, &webhook, &test_config(), 10).await;

        assert_eq!(delivered, 3);
        assert_eq!(webhook.call_count(), 3);
    }

    #[test]
    fn test_read_worker_config_missing() {
        let config = AppConfig::open_in_memory().unwrap();
        assert!(read_worker_config(&config).is_none());
    }

    #[test]
    fn test_read_worker_config_present() {
        let config = AppConfig::open_in_memory().unwrap();
        config.set("webhook_url", "https://example.com/hook").unwrap();
        config.set("webhook_auth_json", r#"{"type":"none"}"#).unwrap();

        let wc = read_worker_config(&config).unwrap();
        assert_eq!(wc.webhook_url, "https://example.com/hook");
    }
}
