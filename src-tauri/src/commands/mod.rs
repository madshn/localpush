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
    let stats = state.ledger.get_stats().map_err(|e| e.to_string())?;

    let overall = if stats.failed > 0 {
        "error"
    } else if stats.pending > 0 || stats.in_flight > 0 {
        "pending"
    } else {
        "active"
    };

    Ok(DeliveryStatusResponse {
        overall: overall.to_string(),
        pending_count: stats.pending + stats.in_flight,
        failed_count: stats.failed,
        last_delivery: None, // TODO: Track last delivery timestamp
    })
}

/// Get available data sources
#[tauri::command]
pub fn get_sources(state: State<'_, AppState>) -> Result<Vec<SourceResponse>, String> {
    let sources = state.source_manager.list_sources();
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
    let mut items = Vec::new();

    for status in [DeliveryStatus::Pending, DeliveryStatus::InFlight, DeliveryStatus::Failed, DeliveryStatus::Delivered] {
        let entries = state.ledger.get_by_status(status).map_err(|e| e.to_string())?;
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

    Ok(items)
}

/// Enable a data source
#[tauri::command]
pub fn enable_source(
    state: State<'_, AppState>,
    source_id: String,
) -> Result<(), String> {
    state.source_manager.enable(&source_id).map_err(|e| e.to_string())
}

/// Disable a data source
#[tauri::command]
pub fn disable_source(
    state: State<'_, AppState>,
    source_id: String,
) -> Result<(), String> {
    state.source_manager.disable(&source_id).map_err(|e| e.to_string())
}

/// Add a webhook target
#[tauri::command]
pub async fn add_webhook_target(
    state: State<'_, AppState>,
    config: WebhookConfig,
) -> Result<(), String> {
    // Store URL in config
    state.config.set("webhook_url", &config.url).map_err(|e| e.to_string())?;

    // Store auth as JSON in config
    let auth_json = serde_json::to_string(&config.auth).map_err(|e| e.to_string())?;
    state.config.set("webhook_auth_json", &auth_json).map_err(|e| e.to_string())?;

    // Also store in keychain for security
    let cred_key = "webhook:default";
    let cred_value = serde_json::to_string(&config.auth).map_err(|e| e.to_string())?;
    state.credentials.store(cred_key, &cred_value).map_err(|e| e.to_string())?;

    tracing::info!("Configured webhook target: {}", config.url);
    Ok(())
}

/// Test a webhook connection
#[tauri::command]
pub async fn test_webhook(
    state: State<'_, AppState>,
    config: WebhookConfig,
) -> Result<String, String> {
    let response = state.webhook_client
        .test(&config.url, &config.auth)
        .await
        .map_err(|e| e.to_string())?;

    Ok(format!("Connected! Response in {}ms", response.duration_ms))
}

/// Get a preview of data from a source (Radical Transparency)
#[tauri::command]
pub fn get_source_preview(
    state: State<'_, AppState>,
    source_id: String,
) -> Result<serde_json::Value, String> {
    let source = state.source_manager.get_source(&source_id)
        .ok_or_else(|| format!("Unknown source: {}", source_id))?;
    let preview = source.preview().map_err(|e| e.to_string())?;
    serde_json::to_value(preview).map_err(|e| e.to_string())
}
