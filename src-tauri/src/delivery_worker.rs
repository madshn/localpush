//! Background delivery worker for webhook dispatch
//!
//! This module provides the background loop that polls the delivery ledger and
//! dispatches pending entries via webhook. It uses per-source binding routing
//! (v0.2) with fallback to global webhook config (v0.1 legacy).

use std::sync::Arc;
use std::time::Duration;
use crate::bindings::{BindingStore, SourceBinding};
use crate::config::AppConfig;
use crate::target_manager::TargetManager;
use crate::traits::{CredentialStore, DeliveryLedgerTrait, WebhookClient, WebhookAuth};

/// Legacy worker configuration derived from AppConfig (v0.1 fallback)
pub struct WorkerConfig {
    pub webhook_url: String,
    pub webhook_auth: WebhookAuth,
}

/// A resolved delivery target with enough info to attempt native delivery or webhook POST.
#[derive(Debug, Clone)]
pub struct ResolvedTarget {
    pub url: String,
    pub auth: WebhookAuth,
    pub target_id: String,
    pub endpoint_id: String,
}

/// Resolve auth for a single binding by combining headers_json with credential store secret.
fn resolve_binding_auth(binding: &SourceBinding, credentials: &dyn CredentialStore) -> WebhookAuth {
    let headers_json = match &binding.headers_json {
        Some(json) => json,
        None => return WebhookAuth::None,
    };

    let mut headers: Vec<(String, String)> = match serde_json::from_str(headers_json) {
        Ok(h) => h,
        Err(e) => {
            tracing::warn!(
                binding = %binding.endpoint_id,
                error = %e,
                "Failed to parse binding headers_json"
            );
            return WebhookAuth::None;
        }
    };

    if headers.is_empty() {
        return WebhookAuth::None;
    }

    // If there's a credential key, retrieve the secret and fill in the auth header placeholder
    if let Some(ref cred_key) = binding.auth_credential_key {
        match credentials.retrieve(cred_key) {
            Ok(Some(secret)) => {
                // Find the auth header entry (one with empty value) and fill it
                for header in &mut headers {
                    if header.1.is_empty() {
                        header.1 = secret.clone();
                        break;
                    }
                }
            }
            Ok(None) => {
                tracing::warn!(
                    cred_key = %cred_key,
                    binding = %binding.endpoint_id,
                    "Binding credential not found in store"
                );
            }
            Err(e) => {
                tracing::warn!(
                    cred_key = %cred_key,
                    error = %e,
                    "Failed to retrieve binding credential"
                );
            }
        }
    }

    WebhookAuth::Custom { headers }
}

/// Resolve delivery targets for an entry.
///
/// If `target_endpoint_id` is set (targeted/scheduled delivery), return only that
/// specific binding's endpoint. Otherwise, filter to on_change bindings only,
/// with fallback to legacy global webhook.
fn resolve_targets(
    source_id: &str,
    target_endpoint_id: Option<&str>,
    binding_store: &BindingStore,
    legacy_config: Option<&WorkerConfig>,
    credentials: &dyn CredentialStore,
) -> Vec<ResolvedTarget> {
    if let Some(ep_id) = target_endpoint_id {
        // Targeted delivery: find the specific binding
        let bindings = binding_store.get_for_source(source_id);
        if let Some(b) = bindings.into_iter().find(|b| b.endpoint_id == ep_id) {
            let auth = resolve_binding_auth(&b, credentials);
            return vec![ResolvedTarget {
                url: b.endpoint_url,
                auth,
                target_id: b.target_id,
                endpoint_id: b.endpoint_id,
            }];
        }
        tracing::warn!(
            source_id = %source_id,
            endpoint_id = %ep_id,
            "Targeted binding not found"
        );
        return Vec::new();
    }

    // Fan-out: only on_change bindings
    let bindings = binding_store.get_for_source(source_id);
    let on_change_bindings: Vec<_> = bindings
        .into_iter()
        .filter(|b| b.delivery_mode == "on_change")
        .collect();

    if !on_change_bindings.is_empty() {
        return on_change_bindings
            .into_iter()
            .map(|b| {
                let auth = resolve_binding_auth(&b, credentials);
                ResolvedTarget {
                    url: b.endpoint_url,
                    auth,
                    target_id: b.target_id,
                    endpoint_id: b.endpoint_id,
                }
            })
            .collect();
    }

    // v0.1 fallback: global webhook
    if let Some(cfg) = legacy_config {
        if !cfg.webhook_url.is_empty() {
            return vec![ResolvedTarget {
                url: cfg.webhook_url.clone(),
                auth: cfg.webhook_auth.clone(),
                target_id: String::new(),
                endpoint_id: String::new(),
            }];
        }
    }

    Vec::new()
}

/// Info about a delivery that transitioned to DLQ (all retries exhausted).
#[derive(Debug, Clone)]
pub struct DlqTransition {
    pub source_id: String,
    pub error: String,
}

/// Result of processing a delivery batch.
#[derive(Debug, Default)]
pub struct BatchResult {
    pub delivered: usize,
    pub failed: usize,
    pub dlq_transitions: Vec<DlqTransition>,
}

/// Process one batch of deliveries with binding-aware routing.
///
/// For each entry, resolves targets from bindings (by source_id/event_type),
/// falling back to legacy global webhook if no bindings exist.
/// Native targets (e.g. Google Sheets) get first chance via `deliver()`;
/// if they return `Ok(true)`, webhook POST is skipped.
pub async fn process_batch(
    ledger: &dyn DeliveryLedgerTrait,
    webhook: &dyn WebhookClient,
    binding_store: &BindingStore,
    legacy_config: Option<&WorkerConfig>,
    credentials: &dyn CredentialStore,
    target_manager: Option<&TargetManager>,
    batch_size: usize,
) -> BatchResult {
    let entries = match ledger.claim_batch(batch_size) {
        Ok(entries) => entries,
        Err(e) => {
            tracing::error!("Failed to claim batch: {}", e);
            return BatchResult::default();
        }
    };

    let mut result = BatchResult::default();

    for entry in entries {
        let targets = resolve_targets(&entry.event_type, entry.target_endpoint_id.as_deref(), binding_store, legacy_config, credentials);

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

        for rt in &targets {
            // Try native delivery first (e.g. Google Sheets appends rows directly)
            if !rt.target_id.is_empty() {
                if let Some(tm) = target_manager {
                    if let Some(target) = tm.get(&rt.target_id) {
                        match target.deliver(&rt.endpoint_id, &entry.payload, &entry.event_type, credentials).await {
                            Ok(true) => {
                                any_success = true;
                                tracing::debug!(
                                    target_id = %rt.target_id,
                                    endpoint_id = %rt.endpoint_id,
                                    event_id = %entry.event_id,
                                    "Delivered natively"
                                );
                                continue; // Skip webhook POST
                            }
                            Ok(false) => {
                                // Target doesn't handle delivery — fall through to webhook
                            }
                            Err(e) => {
                                tracing::warn!(
                                    target_id = %rt.target_id,
                                    event_id = %entry.event_id,
                                    error = %e,
                                    "Native delivery failed"
                                );
                                last_error = Some(e.to_string());
                                continue; // Don't also try webhook for this target
                            }
                        }
                    }
                }
            }

            // Webhook delivery (default path)
            match webhook.send(&rt.url, &entry.payload, &rt.auth).await {
                Ok(_) => {
                    any_success = true;
                    tracing::debug!(url = %rt.url, event_id = %entry.event_id, "Delivered");
                }
                Err(e) => {
                    tracing::warn!(url = %rt.url, event_id = %entry.event_id, error = %e, "Delivery failed");
                    last_error = Some(e.to_string());
                }
            }
        }

        if any_success {
            if ledger.mark_delivered(&entry.event_id).is_ok() {
                result.delivered += 1;
            }
        } else if let Some(err) = last_error {
            if let Ok(new_status) = ledger.mark_failed(&entry.event_id, &err) {
                if new_status == crate::traits::DeliveryStatus::Dlq {
                    result.dlq_transitions.push(DlqTransition {
                        source_id: entry.event_type.clone(),
                        error: err.clone(),
                    });
                }
            }
            result.failed += 1;
        }
    }

    if result.delivered > 0 || result.failed > 0 {
        tracing::info!("Delivery batch: {} delivered, {} failed", result.delivered, result.failed);
    }

    result
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

/// Update the tray icon to reflect DLQ status.
///
/// Shows a red indicator ("!") next to the tray icon when there are DLQ entries,
/// clears it when all DLQ entries are resolved.
fn update_tray_for_dlq(app_handle: &tauri::AppHandle, has_dlq: bool) {
    if let Some(tray) = app_handle.tray_by_id("main-tray") {
        if has_dlq {
            let _ = tray.set_title(Some("!"));
        } else {
            let _ = tray.set_title(Some(""));
        }
    }
}

/// Send a macOS notification when a delivery hits DLQ.
fn notify_dlq(app_handle: &tauri::AppHandle, transition: &DlqTransition) {
    use tauri_plugin_notification::NotificationExt;
    let source_label = transition.source_id.replace('-', " ");
    let _ = app_handle
        .notification()
        .builder()
        .title("LocalPush: Delivery failed")
        .body(format!(
            "Your {} delivery failed after 5 retries.\nOpen LocalPush to investigate.",
            source_label
        ))
        .show();
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
    credentials: Arc<dyn CredentialStore>,
    target_manager: Arc<TargetManager>,
    app_handle: tauri::AppHandle,
) -> tauri::async_runtime::JoinHandle<()> {
    tauri::async_runtime::spawn(async move {
        tracing::info!("Delivery worker started (5s interval, binding-aware routing)");
        let mut interval = tokio::time::interval(Duration::from_secs(5));
        let mut tick_count: u64 = 0;
        let mut tray_showing_error = false;
        loop {
            interval.tick().await;
            tick_count += 1;
            let legacy_config = read_worker_config(&config);
            let has_legacy = legacy_config.is_some();
            let binding_count = binding_store.count();
            tracing::debug!(
                tick = tick_count,
                bindings = binding_count,
                has_legacy_webhook = has_legacy,
                "Delivery worker tick"
            );
            let result = process_batch(
                &*ledger,
                &*webhook,
                &binding_store,
                legacy_config.as_ref(),
                &*credentials,
                Some(&target_manager),
                10,
            ).await;

            // Handle DLQ transitions: notify + update tray
            for transition in &result.dlq_transitions {
                tracing::error!(
                    source = %transition.source_id,
                    error = %transition.error,
                    "Delivery moved to DLQ — notifying user"
                );
                notify_dlq(&app_handle, transition);
            }

            // Update tray icon based on DLQ state (check every tick, not just on transitions)
            let has_dlq = ledger.get_stats().map(|s| s.dlq > 0).unwrap_or(false);
            if has_dlq != tray_showing_error {
                update_tray_for_dlq(&app_handle, has_dlq);
                tray_showing_error = has_dlq;
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bindings::SourceBinding;
    use crate::mocks::{InMemoryCredentialStore, RecordedWebhookClient};
    use crate::DeliveryLedger;
    use crate::traits::DeliveryStatus;

    fn test_config() -> WorkerConfig {
        WorkerConfig {
            webhook_url: "https://example.com/hook".to_string(),
            webhook_auth: WebhookAuth::None,
        }
    }

    fn test_credentials() -> InMemoryCredentialStore {
        InMemoryCredentialStore::new()
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
            headers_json: None,
            auth_credential_key: None,
            delivery_mode: "on_change".to_string(),
            schedule_time: None,
            schedule_day: None,
            last_scheduled_at: None,
        }).unwrap();
        store
    }

    #[tokio::test]
    async fn test_delivers_via_legacy_config() {
        let ledger = DeliveryLedger::open_in_memory().unwrap();
        let webhook = RecordedWebhookClient::success();
        let bs = test_binding_store();
        let creds = test_credentials();
        ledger.enqueue("test.event", serde_json::json!({"hello": "world"})).unwrap();

        let result = process_batch(&ledger, &webhook, &bs, Some(&test_config()), &creds, None, 10).await;

        assert_eq!(result.delivered, 1);
        assert_eq!(result.failed, 0);
        assert_eq!(webhook.call_count(), 1);
        assert_eq!(ledger.get_by_status(DeliveryStatus::Delivered).unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_delivers_via_binding() {
        let ledger = DeliveryLedger::open_in_memory().unwrap();
        let webhook = RecordedWebhookClient::success();
        let bs = test_binding_store_with_binding("my-source", "https://target.example.com/webhook");
        let creds = test_credentials();
        ledger.enqueue("my-source", serde_json::json!({"data": 1})).unwrap();

        let result = process_batch(&ledger, &webhook, &bs, None, &creds, None, 10).await;

        assert_eq!(result.delivered, 1);
        assert_eq!(result.failed, 0);
        assert_eq!(webhook.call_count(), 1);
    }

    #[tokio::test]
    async fn test_binding_takes_precedence_over_legacy() {
        let ledger = DeliveryLedger::open_in_memory().unwrap();
        let webhook = RecordedWebhookClient::success();
        let bs = test_binding_store_with_binding("my-source", "https://binding-url.example.com/hook");
        let creds = test_credentials();
        ledger.enqueue("my-source", serde_json::json!({})).unwrap();

        // Even though legacy config is provided, binding should be used
        let result = process_batch(&ledger, &webhook, &bs, Some(&test_config()), &creds, None, 10).await;

        assert_eq!(result.delivered, 1);
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
        let creds = test_credentials();
        ledger.enqueue("test.event", serde_json::json!({})).unwrap();

        let result = process_batch(&ledger, &webhook, &bs, Some(&test_config()), &creds, None, 10).await;

        assert_eq!(result.delivered, 0);
        assert_eq!(result.failed, 1);
        assert_eq!(ledger.get_by_status(DeliveryStatus::Failed).unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_empty_batch_is_noop() {
        let ledger = DeliveryLedger::open_in_memory().unwrap();
        let webhook = RecordedWebhookClient::success();
        let bs = test_binding_store();
        let creds = test_credentials();

        let result = process_batch(&ledger, &webhook, &bs, Some(&test_config()), &creds, None, 10).await;

        assert_eq!(result.delivered, 0);
        assert_eq!(result.failed, 0);
        assert_eq!(webhook.call_count(), 0);
    }

    #[tokio::test]
    async fn test_processes_multiple_entries() {
        let ledger = DeliveryLedger::open_in_memory().unwrap();
        let webhook = RecordedWebhookClient::success();
        let bs = test_binding_store();
        let creds = test_credentials();

        ledger.enqueue("event.a", serde_json::json!({"a": 1})).unwrap();
        ledger.enqueue("event.b", serde_json::json!({"b": 2})).unwrap();
        ledger.enqueue("event.c", serde_json::json!({"c": 3})).unwrap();

        let result = process_batch(&ledger, &webhook, &bs, Some(&test_config()), &creds, None, 10).await;

        assert_eq!(result.delivered, 3);
        assert_eq!(webhook.call_count(), 3);
    }

    #[tokio::test]
    async fn test_no_targets_marks_delivered() {
        let ledger = DeliveryLedger::open_in_memory().unwrap();
        let webhook = RecordedWebhookClient::success();
        let bs = test_binding_store(); // no bindings
        let creds = test_credentials();
        ledger.enqueue("orphan-source", serde_json::json!({})).unwrap();

        // No legacy config, no bindings → entry should be marked delivered (not stuck)
        let result = process_batch(&ledger, &webhook, &bs, None, &creds, None, 10).await;

        assert_eq!(result.delivered, 0); // resolve_targets returns empty, skipped
        assert_eq!(result.failed, 0);
        assert_eq!(webhook.call_count(), 0);
        // Entry was marked delivered to prevent infinite retry
        assert_eq!(ledger.get_by_status(DeliveryStatus::Delivered).unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_binding_with_custom_auth_headers() {
        let ledger = DeliveryLedger::open_in_memory().unwrap();
        let webhook = RecordedWebhookClient::success();
        let config = Arc::new(AppConfig::open_in_memory().unwrap());
        let bs = BindingStore::new(config);
        let creds = InMemoryCredentialStore::with_entries(vec![
            ("binding:my-source:ep1", "Bearer secret-token-123"),
        ]);

        // Binding with headers_json (auth header name with empty value) + credential key
        let headers: Vec<(String, String)> = vec![
            ("Authorization".to_string(), String::new()), // placeholder for secret
            ("X-Metrick-Source".to_string(), "localpush".to_string()),
        ];
        bs.save(&SourceBinding {
            source_id: "my-source".to_string(),
            target_id: "t1".to_string(),
            endpoint_id: "ep1".to_string(),
            endpoint_url: "https://target.example.com/webhook".to_string(),
            endpoint_name: "Auth Endpoint".to_string(),
            created_at: 1000,
            active: true,
            headers_json: Some(serde_json::to_string(&headers).unwrap()),
            auth_credential_key: Some("binding:my-source:ep1".to_string()),
            delivery_mode: "on_change".to_string(),
            schedule_time: None,
            schedule_day: None,
            last_scheduled_at: None,
        }).unwrap();

        ledger.enqueue("my-source", serde_json::json!({"data": 1})).unwrap();

        let result = process_batch(&ledger, &webhook, &bs, None, &creds, None, 10).await;

        assert_eq!(result.delivered, 1);
        assert_eq!(result.failed, 0);

        // Verify the auth was resolved correctly
        let requests = webhook.requests();
        assert_eq!(requests.len(), 1);
        match &requests[0].auth {
            WebhookAuth::Custom { headers } => {
                assert_eq!(headers.len(), 2);
                assert_eq!(headers[0], ("Authorization".to_string(), "Bearer secret-token-123".to_string()));
                assert_eq!(headers[1], ("X-Metrick-Source".to_string(), "localpush".to_string()));
            }
            other => panic!("Expected Custom auth, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_binding_without_auth_sends_none() {
        let ledger = DeliveryLedger::open_in_memory().unwrap();
        let webhook = RecordedWebhookClient::success();
        let bs = test_binding_store_with_binding("my-source", "https://target.example.com/webhook");
        let creds = test_credentials();
        ledger.enqueue("my-source", serde_json::json!({"data": 1})).unwrap();

        let result = process_batch(&ledger, &webhook, &bs, None, &creds, None, 10).await;

        assert_eq!(result.delivered, 1);
        let requests = webhook.requests();
        assert!(matches!(&requests[0].auth, WebhookAuth::None));
    }

    #[tokio::test]
    async fn test_non_dlq_failure_has_empty_transitions() {
        let ledger = DeliveryLedger::open_in_memory().unwrap();
        let webhook = RecordedWebhookClient::always_fail(
            crate::traits::WebhookError::NetworkError("refused".to_string())
        );
        let bs = test_binding_store();
        let creds = test_credentials();
        ledger.enqueue("test.event", serde_json::json!({})).unwrap();

        let result = process_batch(&ledger, &webhook, &bs, Some(&test_config()), &creds, None, 10).await;

        assert_eq!(result.failed, 1);
        assert!(result.dlq_transitions.is_empty(), "first failure is not DLQ");
        assert_eq!(ledger.get_by_status(DeliveryStatus::Failed).unwrap().len(), 1);
    }

    #[test]
    fn test_resolve_binding_auth_no_headers() {
        let creds = test_credentials();
        let binding = SourceBinding {
            source_id: "s1".to_string(),
            target_id: "t1".to_string(),
            endpoint_id: "ep1".to_string(),
            endpoint_url: "https://example.com".to_string(),
            endpoint_name: "Test".to_string(),
            created_at: 1000,
            active: true,
            headers_json: None,
            auth_credential_key: None,
            delivery_mode: "on_change".to_string(),
            schedule_time: None,
            schedule_day: None,
            last_scheduled_at: None,
        };
        assert!(matches!(resolve_binding_auth(&binding, &creds), WebhookAuth::None));
    }

    #[test]
    fn test_resolve_binding_auth_with_credential() {
        let creds = InMemoryCredentialStore::with_entries(vec![
            ("binding:s1:ep1", "my-secret"),
        ]);
        let headers: Vec<(String, String)> = vec![
            ("Authorization".to_string(), String::new()),
        ];
        let binding = SourceBinding {
            source_id: "s1".to_string(),
            target_id: "t1".to_string(),
            endpoint_id: "ep1".to_string(),
            endpoint_url: "https://example.com".to_string(),
            endpoint_name: "Test".to_string(),
            created_at: 1000,
            active: true,
            headers_json: Some(serde_json::to_string(&headers).unwrap()),
            auth_credential_key: Some("binding:s1:ep1".to_string()),
            delivery_mode: "on_change".to_string(),
            schedule_time: None,
            schedule_day: None,
            last_scheduled_at: None,
        };
        match resolve_binding_auth(&binding, &creds) {
            WebhookAuth::Custom { headers } => {
                assert_eq!(headers.len(), 1);
                assert_eq!(headers[0].1, "my-secret");
            }
            other => panic!("Expected Custom, got {:?}", other),
        }
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

    // ========================================================================
    // Native delivery tests (Target.deliver() integration)
    // ========================================================================

    use crate::target_manager::TargetManager;
    use crate::traits::{Target, TargetInfo, TargetEndpoint, TargetError, CredentialStore as CredTrait};

    /// Mock target that handles delivery natively (returns Ok(true))
    struct NativeDeliveryTarget;

    #[async_trait::async_trait]
    impl Target for NativeDeliveryTarget {
        fn id(&self) -> &str { "native-t1" }
        fn name(&self) -> &str { "Native Target" }
        fn target_type(&self) -> &str { "native" }
        fn base_url(&self) -> &str { "https://native.example.com" }

        async fn test_connection(&self) -> Result<TargetInfo, TargetError> {
            Ok(TargetInfo {
                id: self.id().to_string(),
                name: self.name().to_string(),
                target_type: self.target_type().to_string(),
                base_url: self.base_url().to_string(),
                connected: true,
                details: serde_json::json!({}),
            })
        }

        async fn list_endpoints(&self) -> Result<Vec<TargetEndpoint>, TargetError> {
            Ok(vec![])
        }

        async fn deliver(
            &self,
            _endpoint_id: &str,
            _payload: &serde_json::Value,
            _event_type: &str,
            _credentials: &dyn CredTrait,
        ) -> Result<bool, TargetError> {
            Ok(true) // Handled natively
        }
    }

    /// Mock target that does NOT handle delivery (returns Ok(false))
    struct PassthroughTarget;

    #[async_trait::async_trait]
    impl Target for PassthroughTarget {
        fn id(&self) -> &str { "passthrough-t1" }
        fn name(&self) -> &str { "Passthrough Target" }
        fn target_type(&self) -> &str { "passthrough" }
        fn base_url(&self) -> &str { "https://passthrough.example.com" }

        async fn test_connection(&self) -> Result<TargetInfo, TargetError> {
            Ok(TargetInfo {
                id: self.id().to_string(),
                name: self.name().to_string(),
                target_type: self.target_type().to_string(),
                base_url: self.base_url().to_string(),
                connected: true,
                details: serde_json::json!({}),
            })
        }

        async fn list_endpoints(&self) -> Result<Vec<TargetEndpoint>, TargetError> {
            Ok(vec![])
        }

        // Uses default deliver() → Ok(false)
    }

    #[tokio::test]
    async fn test_native_delivery_skips_webhook() {
        let ledger = DeliveryLedger::open_in_memory().unwrap();
        let webhook = RecordedWebhookClient::success();
        let config = Arc::new(AppConfig::open_in_memory().unwrap());
        let bs = BindingStore::new(config.clone());
        let creds = test_credentials();
        let tm = TargetManager::new(config.clone());

        // Register native target
        tm.register(Arc::new(NativeDeliveryTarget));

        // Create binding pointing to the native target
        bs.save(&SourceBinding {
            source_id: "my-source".to_string(),
            target_id: "native-t1".to_string(),
            endpoint_id: "ep1".to_string(),
            endpoint_url: "https://native.example.com/endpoint".to_string(),
            endpoint_name: "Native Endpoint".to_string(),
            created_at: 1000,
            active: true,
            headers_json: None,
            auth_credential_key: None,
            delivery_mode: "on_change".to_string(),
            schedule_time: None,
            schedule_day: None,
            last_scheduled_at: None,
        }).unwrap();

        ledger.enqueue("my-source", serde_json::json!({"data": 1})).unwrap();

        let result = process_batch(&ledger, &webhook, &bs, None, &creds, Some(&tm), 10).await;

        assert_eq!(result.delivered, 1, "Entry should be marked delivered");
        assert_eq!(result.failed, 0);
        assert_eq!(webhook.call_count(), 0, "Webhook should NOT be called when target handles delivery natively");
    }

    #[tokio::test]
    async fn test_passthrough_target_uses_webhook() {
        let ledger = DeliveryLedger::open_in_memory().unwrap();
        let webhook = RecordedWebhookClient::success();
        let config = Arc::new(AppConfig::open_in_memory().unwrap());
        let bs = BindingStore::new(config.clone());
        let creds = test_credentials();
        let tm = TargetManager::new(config.clone());

        // Register passthrough target (deliver() returns Ok(false))
        tm.register(Arc::new(PassthroughTarget));

        // Create binding pointing to the passthrough target
        bs.save(&SourceBinding {
            source_id: "my-source".to_string(),
            target_id: "passthrough-t1".to_string(),
            endpoint_id: "ep1".to_string(),
            endpoint_url: "https://passthrough.example.com/hook".to_string(),
            endpoint_name: "Passthrough Endpoint".to_string(),
            created_at: 1000,
            active: true,
            headers_json: None,
            auth_credential_key: None,
            delivery_mode: "on_change".to_string(),
            schedule_time: None,
            schedule_day: None,
            last_scheduled_at: None,
        }).unwrap();

        ledger.enqueue("my-source", serde_json::json!({"data": 1})).unwrap();

        let result = process_batch(&ledger, &webhook, &bs, None, &creds, Some(&tm), 10).await;

        assert_eq!(result.delivered, 1, "Entry should be delivered via webhook");
        assert_eq!(result.failed, 0);
        assert_eq!(webhook.call_count(), 1, "Webhook SHOULD be called when target returns Ok(false)");
    }
}
