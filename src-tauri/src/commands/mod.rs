//! Tauri commands exposed to the frontend

use std::sync::Arc;
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
pub fn get_delivery_status(state: State<'_, Arc<AppState>>) -> Result<DeliveryStatusResponse, String> {
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
pub fn get_sources(_state: State<'_, Arc<AppState>>) -> Result<Vec<SourceResponse>, String> {
    // For MVP, only Claude Code Stats
    Ok(vec![
        SourceResponse {
            id: "claude-stats".to_string(),
            name: "Claude Code Stats".to_string(),
            description: "Token usage, sessions, messages from Claude Code".to_string(),
            enabled: false, // TODO: Track enabled state
            last_sync: None,
        },
    ])
}

/// Get the delivery queue
#[tauri::command]
pub fn get_delivery_queue(state: State<'_, Arc<AppState>>) -> Result<Vec<DeliveryQueueItem>, String> {
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
    _state: State<'_, Arc<AppState>>,
    source_id: String,
) -> Result<(), String> {
    tracing::info!("Enabling source: {}", source_id);
    // TODO: Implement source enabling
    Ok(())
}

/// Disable a data source
#[tauri::command]
pub fn disable_source(
    _state: State<'_, Arc<AppState>>,
    source_id: String,
) -> Result<(), String> {
    tracing::info!("Disabling source: {}", source_id);
    // TODO: Implement source disabling
    Ok(())
}

/// Add a webhook target
#[tauri::command]
pub async fn add_webhook_target(
    state: State<'_, Arc<AppState>>,
    config: WebhookConfig,
) -> Result<(), String> {
    // Store credentials securely
    let cred_key = format!("webhook:{}", uuid::Uuid::new_v4());
    let cred_value = serde_json::to_string(&config.auth).map_err(|e| e.to_string())?;

    state.credentials.store(&cred_key, &cred_value).map_err(|e| e.to_string())?;

    tracing::info!("Added webhook target: {}", config.url);
    Ok(())
}

/// Test a webhook connection
#[tauri::command]
pub async fn test_webhook(
    state: State<'_, Arc<AppState>>,
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
    _state: State<'_, Arc<AppState>>,
    source_id: String,
) -> Result<serde_json::Value, String> {
    match source_id.as_str() {
        "claude-stats" => {
            // TODO: Read actual Claude Code stats
            Ok(serde_json::json!({
                "preview": {
                    "tokens_today": 0,
                    "sessions_today": 0,
                    "messages_today": 0,
                },
                "fields": [
                    {"name": "token.total", "description": "Total tokens used", "type": "number"},
                    {"name": "token.sessions", "description": "Number of coding sessions", "type": "number"},
                    {"name": "token.messages", "description": "Messages sent", "type": "number"},
                ],
                "collects": [
                    "Daily token counts",
                    "Session counts",
                    "Message and tool counts"
                ],
                "does_not_collect": [
                    "Conversation content",
                    "Code or file contents",
                    "Anything you typed"
                ]
            }))
        }
        _ => Err(format!("Unknown source: {}", source_id)),
    }
}
