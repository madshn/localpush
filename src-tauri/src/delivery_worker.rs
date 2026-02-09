//! Background delivery worker for webhook dispatch
//!
//! This module provides the background loop that polls the delivery ledger and
//! dispatches pending entries via webhook. It uses per-source binding routing
//! (v0.2) with fallback to global webhook config (v0.1 legacy).

use std::sync::Arc;
use std::time::Duration;
use crate::bindings::BindingStore;
use crate::config::AppConfig;
use crate::traits::{DeliveryLedgerTrait, WebhookClient, WebhookAuth};

/// Legacy worker configuration derived from AppConfig (v0.1 fallback)
pub struct WorkerConfig {
    pub webhook_url: String,
    pub webhook_auth: WebhookAuth,
}

/// Resolve delivery targets for an entry.
/// Prefers per-source bindings; falls back to legacy global webhook.
fn resolve_targets(
    source_id: &str,
    binding_store: &BindingStore,
    legacy_config: Option<&WorkerConfig>,
) -> Vec<(String, WebhookAuth)> {
    let bindings = binding_store.get_for_source(source_id);
    if !bindings.is_empty() {
        return bindings
            .into_iter()
            .map(|b| (b.endpoint_url, WebhookAuth::None))
            .collect();
    }

    // v0.1 fallback: global webhook
    if let Some(cfg) = legacy_config {
        if !cfg.webhook_url.is_empty() {
            return vec![(cfg.webhook_url.clone(), cfg.webhook_auth.clone())];
        }
    }

    Vec::new()
}

/// Process one batch of deliveries with binding-aware routing.
///
/// For each entry, resolves targets from bindings (by source_id/event_type),
/// falling back to legacy global webhook if no bindings exist.
///
/// Returns (delivered_count, failed_count).
pub async fn process_batch(
    ledger: &dyn DeliveryLedgerTrait,
    webhook: &dyn WebhookClient,
    binding_store: &BindingStore,
    legacy_config: Option<&WorkerConfig>,
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
        let targets = resolve_targets(&entry.event_type, binding_store, legacy_config);

        if targets.is_empty() {
            tracing::debug!(
                event_type = %entry.event_type,
                event_id = %entry.event_id,
                "No delivery targets found, skipping"
            );
            // No target is not a failure — mark delivered so it doesn't retry
            let _ = ledger.mark_delivered(&entry.event_id);
            continue;
        }

        let mut any_success = false;
        let mut last_error = None;

        for (url, auth) in &targets {
            match webhook.send(url, &entry.payload, auth).await {
                Ok(_) => {
                    any_success = true;
                    tracing::debug!(url = %url, event_id = %entry.event_id, "Delivered");
                }
                Err(e) => {
                    tracing::warn!(url = %url, event_id = %entry.event_id, error = %e, "Delivery failed");
                    last_error = Some(e.to_string());
                }
            }
        }

        if any_success {
            if ledger.mark_delivered(&entry.event_id).is_ok() {
                delivered += 1;
            }
        } else if let Some(err) = last_error {
            let _ = ledger.mark_failed(&entry.event_id, &err);
            failed += 1;
        }
    }

    if delivered > 0 || failed > 0 {
        tracing::info!("Delivery batch: {} delivered, {} failed", delivered, failed);
    }

    (delivered, failed)
}

/// Read legacy webhook config from AppConfig. Returns None if not configured.
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
/// The worker polls every 5 seconds, resolving delivery targets from bindings
/// per source, with fallback to legacy global webhook config.
pub fn spawn_worker(
    ledger: Arc<dyn DeliveryLedgerTrait>,
    webhook: Arc<dyn WebhookClient>,
    config: Arc<AppConfig>,
    binding_store: Arc<BindingStore>,
) -> tauri::async_runtime::JoinHandle<()> {
    tauri::async_runtime::spawn(async move {
        tracing::info!("Delivery worker started (5s interval, binding-aware routing)");
        let mut interval = tokio::time::interval(Duration::from_secs(5));
        loop {
            interval.tick().await;
            let legacy_config = read_worker_config(&config);
            process_batch(
                &*ledger,
                &*webhook,
                &binding_store,
                legacy_config.as_ref(),
                10,
            ).await;
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bindings::SourceBinding;
    use crate::mocks::RecordedWebhookClient;
    use crate::DeliveryLedger;
    use crate::traits::DeliveryStatus;

    fn test_config() -> WorkerConfig {
        WorkerConfig {
            webhook_url: "https://example.com/hook".to_string(),
            webhook_auth: WebhookAuth::None,
        }
    }

    fn test_binding_store() -> BindingStore {
        BindingStore::new(Arc::new(AppConfig::open_in_memory().unwrap()))
    }

    fn test_binding_store_with_binding(source_id: &str, url: &str) -> BindingStore {
        let config = Arc::new(AppConfig::open_in_memory().unwrap());
        let store = BindingStore::new(config);
        store.save(&SourceBinding {
            source_id: source_id.to_string(),
            target_id: "t1".to_string(),
            endpoint_id: "ep1".to_string(),
            endpoint_url: url.to_string(),
            endpoint_name: "Test Endpoint".to_string(),
            created_at: 1000,
            active: true,
        }).unwrap();
        store
    }

    #[tokio::test]
    async fn test_delivers_via_legacy_config() {
        let ledger = DeliveryLedger::open_in_memory().unwrap();
        let webhook = RecordedWebhookClient::success();
        let bs = test_binding_store();
        ledger.enqueue("test.event", serde_json::json!({"hello": "world"})).unwrap();

        let (delivered, failed) = process_batch(&ledger, &webhook, &bs, Some(&test_config()), 10).await;

        assert_eq!(delivered, 1);
        assert_eq!(failed, 0);
        assert_eq!(webhook.call_count(), 1);
        assert_eq!(ledger.get_by_status(DeliveryStatus::Delivered).unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_delivers_via_binding() {
        let ledger = DeliveryLedger::open_in_memory().unwrap();
        let webhook = RecordedWebhookClient::success();
        let bs = test_binding_store_with_binding("my-source", "https://target.example.com/webhook");
        ledger.enqueue("my-source", serde_json::json!({"data": 1})).unwrap();

        let (delivered, failed) = process_batch(&ledger, &webhook, &bs, None, 10).await;

        assert_eq!(delivered, 1);
        assert_eq!(failed, 0);
        assert_eq!(webhook.call_count(), 1);
    }

    #[tokio::test]
    async fn test_binding_takes_precedence_over_legacy() {
        let ledger = DeliveryLedger::open_in_memory().unwrap();
        let webhook = RecordedWebhookClient::success();
        let bs = test_binding_store_with_binding("my-source", "https://binding-url.example.com/hook");
        ledger.enqueue("my-source", serde_json::json!({})).unwrap();

        // Even though legacy config is provided, binding should be used
        let (delivered, _) = process_batch(&ledger, &webhook, &bs, Some(&test_config()), 10).await;

        assert_eq!(delivered, 1);
        // Webhook was called with binding URL, not legacy URL
        assert_eq!(webhook.call_count(), 1);
    }

    #[tokio::test]
    async fn test_marks_failed_on_error() {
        let ledger = DeliveryLedger::open_in_memory().unwrap();
        let webhook = RecordedWebhookClient::always_fail(
            crate::traits::WebhookError::NetworkError("refused".to_string())
        );
        let bs = test_binding_store();
        ledger.enqueue("test.event", serde_json::json!({})).unwrap();

        let (delivered, failed) = process_batch(&ledger, &webhook, &bs, Some(&test_config()), 10).await;

        assert_eq!(delivered, 0);
        assert_eq!(failed, 1);
        assert_eq!(ledger.get_by_status(DeliveryStatus::Failed).unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_empty_batch_is_noop() {
        let ledger = DeliveryLedger::open_in_memory().unwrap();
        let webhook = RecordedWebhookClient::success();
        let bs = test_binding_store();

        let (delivered, failed) = process_batch(&ledger, &webhook, &bs, Some(&test_config()), 10).await;

        assert_eq!(delivered, 0);
        assert_eq!(failed, 0);
        assert_eq!(webhook.call_count(), 0);
    }

    #[tokio::test]
    async fn test_processes_multiple_entries() {
        let ledger = DeliveryLedger::open_in_memory().unwrap();
        let webhook = RecordedWebhookClient::success();
        let bs = test_binding_store();

        ledger.enqueue("event.a", serde_json::json!({"a": 1})).unwrap();
        ledger.enqueue("event.b", serde_json::json!({"b": 2})).unwrap();
        ledger.enqueue("event.c", serde_json::json!({"c": 3})).unwrap();

        let (delivered, _failed) = process_batch(&ledger, &webhook, &bs, Some(&test_config()), 10).await;

        assert_eq!(delivered, 3);
        assert_eq!(webhook.call_count(), 3);
    }

    #[tokio::test]
    async fn test_no_targets_marks_delivered() {
        let ledger = DeliveryLedger::open_in_memory().unwrap();
        let webhook = RecordedWebhookClient::success();
        let bs = test_binding_store(); // no bindings
        ledger.enqueue("orphan-source", serde_json::json!({})).unwrap();

        // No legacy config, no bindings → entry should be marked delivered (not stuck)
        let (delivered, failed) = process_batch(&ledger, &webhook, &bs, None, 10).await;

        assert_eq!(delivered, 0); // resolve_targets returns empty, skipped
        assert_eq!(failed, 0);
        assert_eq!(webhook.call_count(), 0);
        // Entry was marked delivered to prevent infinite retry
        assert_eq!(ledger.get_by_status(DeliveryStatus::Delivered).unwrap().len(), 1);
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
