//! Tauri commands exposed to the frontend

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::state::AppState;
use crate::traits::{DeliveryStatus, WebhookAuth};

#[derive(Debug, Serialize)]
pub struct DeliveryStatusResponse {
    pub overall: String,
    pub pending_count: usize,
    pub failed_count: usize,
    pub last_delivery: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SourceResponse {
    pub id: String,
    pub name: String,
    pub description: String,
    pub enabled: bool,
    pub last_sync: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DeliveryQueueItem {
    pub id: String,
    pub event_type: String,
    pub status: String,
    pub retry_count: u32,
    pub last_error: Option<String>,
    pub created_at: String,
    pub delivered_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct WebhookConfig {
    pub url: String,
    pub auth: WebhookAuth,
}

/// Get the current delivery status
#[tauri::command]
pub fn get_delivery_status(state: State<'_, AppState>) -> Result<DeliveryStatusResponse, String> {
    tracing::info!(command = "get_delivery_status", "Command invoked");
    match state.ledger.get_stats() {
        Ok(stats) => {
            let overall = if stats.failed > 0 {
                "error"
            } else if stats.pending > 0 || stats.in_flight > 0 {
                "pending"
            } else {
                "active"
            };

            tracing::debug!(
                pending = stats.pending,
                in_flight = stats.in_flight,
                failed = stats.failed,
                overall = %overall,
                "Delivery status retrieved"
            );

            Ok(DeliveryStatusResponse {
                overall: overall.to_string(),
                pending_count: stats.pending + stats.in_flight,
                failed_count: stats.failed,
                last_delivery: None, // TODO: Track last delivery timestamp
            })
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to get delivery status");
            Err(e.to_string())
        }
    }
}

/// Get available data sources
#[tauri::command]
pub fn get_sources(state: State<'_, AppState>) -> Result<Vec<SourceResponse>, String> {
    tracing::info!(command = "get_sources", "Command invoked");
    let sources = state.source_manager.list_sources();
    tracing::debug!(source_count = sources.len(), "Sources retrieved");
    Ok(sources.into_iter().map(|s| SourceResponse {
        id: s.id,
        description: format!("Data from {}", s.name),
        name: s.name,
        enabled: s.enabled,
        last_sync: None, // TODO: track last sync time
    }).collect())
}

/// Get the delivery queue
#[tauri::command]
pub fn get_delivery_queue(state: State<'_, AppState>) -> Result<Vec<DeliveryQueueItem>, String> {
    tracing::info!(command = "get_delivery_queue", "Command invoked");
    let mut items = Vec::new();

    for status in [DeliveryStatus::Pending, DeliveryStatus::InFlight, DeliveryStatus::Failed, DeliveryStatus::Delivered] {
        match state.ledger.get_by_status(status) {
            Ok(entries) => {
                for entry in entries {
                    items.push(DeliveryQueueItem {
                        id: entry.id,
                        event_type: entry.event_type,
                        status: entry.status.as_str().to_string(),
                        retry_count: entry.retry_count,
                        last_error: entry.last_error,
                        created_at: chrono::DateTime::from_timestamp(entry.created_at, 0)
                            .map(|dt| dt.to_rfc3339())
                            .unwrap_or_default(),
                        delivered_at: entry.delivered_at
                            .and_then(|ts| chrono::DateTime::from_timestamp(ts, 0))
                            .map(|dt| dt.to_rfc3339()),
                    });
                }
            }
            Err(e) => {
                tracing::error!(status = %status.as_str(), error = %e, "Failed to get entries by status");
                return Err(e.to_string());
            }
        }
    }

    tracing::debug!(queue_size = items.len(), "Delivery queue retrieved");
    Ok(items)
}

/// Enable a data source
#[tauri::command]
pub fn enable_source(
    state: State<'_, AppState>,
    source_id: String,
) -> Result<(), String> {
    tracing::info!(command = "enable_source", source_id = %source_id, "Command invoked");
    match state.source_manager.enable(&source_id) {
        Ok(()) => {
            tracing::info!(source_id = %source_id, "Source enabled successfully");
            Ok(())
        }
        Err(e) => {
            tracing::error!(source_id = %source_id, error = %e, "Failed to enable source");
            Err(e.to_string())
        }
    }
}

/// Disable a data source
#[tauri::command]
pub fn disable_source(
    state: State<'_, AppState>,
    source_id: String,
) -> Result<(), String> {
    tracing::info!(command = "disable_source", source_id = %source_id, "Command invoked");
    match state.source_manager.disable(&source_id) {
        Ok(()) => {
            tracing::info!(source_id = %source_id, "Source disabled successfully");
            Ok(())
        }
        Err(e) => {
            tracing::error!(source_id = %source_id, error = %e, "Failed to disable source");
            Err(e.to_string())
        }
    }
}

/// Add a webhook target
#[tauri::command]
pub async fn add_webhook_target(
    state: State<'_, AppState>,
    config: WebhookConfig,
) -> Result<(), String> {
    tracing::info!(command = "add_webhook_target", url = %config.url, "Command invoked");

    // Store URL in config
    if let Err(e) = state.config.set("webhook_url", &config.url) {
        tracing::error!(error = %e, "Failed to store webhook URL");
        return Err(e.to_string());
    }

    // Store auth as JSON in config
    let auth_json = serde_json::to_string(&config.auth).map_err(|e| {
        tracing::error!(error = %e, "Failed to serialize auth");
        e.to_string()
    })?;
    if let Err(e) = state.config.set("webhook_auth_json", &auth_json) {
        tracing::error!(error = %e, "Failed to store webhook auth");
        return Err(e.to_string());
    }

    // Also store in keychain for security
    let cred_key = "webhook:default";
    let cred_value = serde_json::to_string(&config.auth).map_err(|e| {
        tracing::error!(error = %e, "Failed to serialize auth for keychain");
        e.to_string()
    })?;
    if let Err(e) = state.credentials.store(cred_key, &cred_value) {
        tracing::error!(error = %e, "Failed to store webhook credentials in keychain");
        return Err(e.to_string());
    }

    tracing::info!(url = %config.url, "Webhook target configured successfully");
    Ok(())
}

/// Test a webhook connection
#[tauri::command]
pub async fn test_webhook(
    state: State<'_, AppState>,
    config: WebhookConfig,
) -> Result<String, String> {
    tracing::info!(command = "test_webhook", url = %config.url, "Command invoked");
    match state.webhook_client.test(&config.url, &config.auth).await {
        Ok(response) => {
            tracing::info!(
                url = %config.url,
                duration_ms = response.duration_ms,
                "Webhook test successful"
            );
            Ok(format!("Connected! Response in {}ms", response.duration_ms))
        }
        Err(e) => {
            tracing::error!(url = %config.url, error = %e, "Webhook test failed");
            Err(e.to_string())
        }
    }
}

/// Get a preview of data from a source (Radical Transparency)
#[tauri::command]
pub fn get_source_preview(
    state: State<'_, AppState>,
    source_id: String,
) -> Result<serde_json::Value, String> {
    tracing::info!(command = "get_source_preview", source_id = %source_id, "Command invoked");
    let source = state.source_manager.get_source(&source_id)
        .ok_or_else(|| {
            tracing::error!(source_id = %source_id, "Unknown source");
            format!("Unknown source: {}", source_id)
        })?;

    match source.preview() {
        Ok(preview) => {
            tracing::debug!(source_id = %source_id, "Preview generated successfully");
            serde_json::to_value(preview).map_err(|e| {
                tracing::error!(source_id = %source_id, error = %e, "Failed to serialize preview");
                e.to_string()
            })
        }
        Err(e) => {
            tracing::error!(source_id = %source_id, error = %e, "Failed to generate preview");
            Err(e.to_string())
        }
    }
}

/// Get webhook configuration
#[tauri::command]
pub fn get_webhook_config(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    tracing::info!(command = "get_webhook_config", "Command invoked");

    match state.config.get("webhook_url") {
        Ok(url) => {
            match state.config.get("webhook_auth_json") {
                Ok(auth_json) => {
                    let auth = auth_json.and_then(|j| serde_json::from_str::<WebhookAuth>(&j).ok());
                    tracing::debug!(has_url = url.is_some(), has_auth = auth.is_some(), "Webhook config retrieved");
                    Ok(serde_json::json!({
                        "url": url.unwrap_or_default(),
                        "auth": auth
                    }))
                }
                Err(e) => {
                    tracing::error!(error = %e, "Failed to get webhook auth");
                    Err(e.to_string())
                }
            }
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to get webhook url");
            Err(e.to_string())
        }
    }
}

/// Get a setting value
#[tauri::command]
pub fn get_setting(state: State<'_, AppState>, key: String) -> Result<Option<String>, String> {
    tracing::info!(command = "get_setting", key = %key, "Command invoked");
    match state.config.get(&key) {
        Ok(value) => {
            tracing::debug!(key = %key, found = value.is_some(), "Setting retrieved");
            Ok(value)
        }
        Err(e) => {
            tracing::error!(key = %key, error = %e, "Failed to get setting");
            Err(e.to_string())
        }
    }
}

/// Set a setting value
#[tauri::command]
pub fn set_setting(state: State<'_, AppState>, key: String, value: String) -> Result<(), String> {
    tracing::info!(command = "set_setting", key = %key, "Command invoked");
    match state.config.set(&key, &value) {
        Ok(()) => {
            tracing::info!(key = %key, "Setting updated successfully");
            Ok(())
        }
        Err(e) => {
            tracing::error!(key = %key, error = %e, "Failed to set setting");
            Err(e.to_string())
        }
    }
}

/// Retry a failed delivery
#[tauri::command]
pub fn retry_delivery(state: State<'_, AppState>, event_id: String) -> Result<(), String> {
    tracing::info!(command = "retry_delivery", event_id = %event_id, "Command invoked");
    // Reset the entry to pending status so the worker picks it up
    match state.ledger.reset_to_pending(&event_id) {
        Ok(()) => {
            tracing::info!(event_id = %event_id, "Delivery reset to pending for retry");
            Ok(())
        }
        Err(e) => {
            tracing::error!(event_id = %event_id, error = %e, "Failed to retry delivery");
            Err(e.to_string())
        }
    }
}
