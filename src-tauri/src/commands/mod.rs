//! Tauri commands exposed to the frontend

use chrono::Datelike;
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::bindings::SourceBinding;
use crate::source_config::{PropertyState, SourceConfigStore};
use crate::state::AppState;
use crate::traits::{DeliveryStatus, Target, WebhookAuth};

#[derive(Debug, Serialize)]
pub struct AppInfoResponse {
    pub version: String,
    pub build_profile: String,
}

#[tauri::command]
pub fn get_app_info() -> AppInfoResponse {
    AppInfoResponse {
        version: env!("CARGO_PKG_VERSION").to_string(),
        build_profile: if cfg!(debug_assertions) {
            "debug".to_string()
        } else {
            "release".to_string()
        },
    }
}

/// Open a URL in the user's default browser
#[tauri::command]
pub fn open_url(url: String) -> Result<(), String> {
    open::that(&url).map_err(|e| format!("Failed to open URL: {}", e))
}

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
    pub watch_path: Option<String>,
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
    pub payload: serde_json::Value,
    pub trigger_type: Option<String>,
    pub delivered_to: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct WebhookConfig {
    pub url: String,
    pub auth: WebhookAuth,
}

#[derive(Debug, Deserialize)]
pub struct CustomTargetConfig {
    pub name: String,
    pub webhook_url: String,
    pub auth_type: String,
    pub auth_token: Option<String>,
    pub auth_header_name: Option<String>,
    pub auth_header_value: Option<String>,
    pub auth_username: Option<String>,
    pub auth_password: Option<String>,
}

/// Get the current delivery status
#[tauri::command]
pub fn get_delivery_status(state: State<'_, AppState>) -> Result<DeliveryStatusResponse, String> {
    tracing::debug!(command = "get_delivery_status", "Command invoked");
    match state.ledger.get_stats() {
        Ok(stats) => {
            let overall = if stats.dlq > 0 || stats.failed > 0 || stats.target_paused > 0 {
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
                target_paused = stats.target_paused,
                overall = %overall,
                "Delivery status retrieved"
            );

            Ok(DeliveryStatusResponse {
                overall: overall.to_string(),
                pending_count: stats.pending + stats.in_flight,
                failed_count: stats.failed + stats.target_paused,
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
        id: s.id.clone(),
        description: format!("Data from {}", s.name),
        name: s.name,
        enabled: s.enabled,
        last_sync: None, // TODO: track last sync time
        watch_path: s.watch_path.map(|p| p.display().to_string()),
    }).collect())
}

/// Get the delivery queue
#[tauri::command]
pub fn get_delivery_queue(state: State<'_, AppState>) -> Result<Vec<DeliveryQueueItem>, String> {
    tracing::debug!(command = "get_delivery_queue", "Command invoked");
    let mut items = Vec::new();

    for status in [
        DeliveryStatus::Pending,
        DeliveryStatus::InFlight,
        DeliveryStatus::Failed,
        DeliveryStatus::Dlq,
        DeliveryStatus::TargetPaused,
        DeliveryStatus::Delivered,
    ] {
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
                        payload: entry.payload,
                        trigger_type: entry.trigger_type,
                        delivered_to: entry.delivered_to,
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

/// Get a sample payload from a source (for testing with recipients before enabling)
///
/// Calls source.parse() to produce the real payload JSON, but does NOT enqueue it.
#[tauri::command]
pub fn get_source_sample_payload(
    state: State<'_, AppState>,
    source_id: String,
) -> Result<serde_json::Value, String> {
    tracing::info!(command = "get_source_sample_payload", source_id = %source_id, "Command invoked");
    let source = state.source_manager.get_source(&source_id)
        .ok_or_else(|| {
            tracing::error!(source_id = %source_id, "Unknown source for sample payload");
            format!("Unknown source: {}", source_id)
        })?;

    source.parse().map_err(|e| {
        tracing::error!(source_id = %source_id, error = %e, "Failed to generate sample payload");
        e.to_string()
    })
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

/// Connect an n8n target (instance URL + API key)
#[tauri::command]
pub async fn connect_n8n_target(
    state: State<'_, AppState>,
    instance_url: String,
    api_key: String,
) -> Result<serde_json::Value, String> {
    tracing::info!(command = "connect_n8n_target", url = %instance_url, "Command invoked");

    let target_id = format!(
        "n8n-{}",
        uuid::Uuid::new_v4()
            .to_string()
            .split('-')
            .next()
            .unwrap_or("0")
    );
    let target =
        crate::targets::N8nTarget::new(target_id.clone(), instance_url.clone(), api_key.clone());

    // Test connection before persisting
    let info = target.test_connection().await.map_err(|e| {
        tracing::error!(error = %e, "n8n connection test failed");
        e.to_string()
    })?;

    // Store API key in keychain
    let cred_key = format!("n8n:{}", target_id);
    if let Err(e) = state.credentials.store(&cred_key, &api_key) {
        tracing::warn!(error = %e, "Failed to store n8n API key in keychain");
    }

    // Store URL and type in config
    let _ = state
        .config
        .set(&format!("target.{}.url", target_id), &instance_url);
    let _ = state
        .config
        .set(&format!("target.{}.type", target_id), "n8n");

    // Register target
    state
        .target_manager
        .register(std::sync::Arc::new(target));

    tracing::info!(target_id = %target_id, "n8n target connected successfully");
    serde_json::to_value(info).map_err(|e| e.to_string())
}

/// Connect an ntfy target (server URL + optional topic + auth)
#[tauri::command]
pub async fn connect_ntfy_target(
    state: State<'_, AppState>,
    server_url: String,
    topic: Option<String>,
    auth_token: Option<String>,
) -> Result<serde_json::Value, String> {
    tracing::info!(command = "connect_ntfy_target", url = %server_url, "Command invoked");

    let target_id = format!(
        "ntfy-{}",
        uuid::Uuid::new_v4()
            .to_string()
            .split('-')
            .next()
            .unwrap_or("0")
    );
    let mut target = crate::targets::NtfyTarget::new(target_id.clone(), server_url.clone());
    if let Some(ref t) = topic {
        target = target.with_topic(t.clone());
    }
    if let Some(ref token) = auth_token {
        target = target.with_auth(token.clone());
    }

    let info = target.test_connection().await.map_err(|e| {
        tracing::error!(error = %e, "ntfy connection test failed");
        e.to_string()
    })?;

    // Store in config
    let _ = state
        .config
        .set(&format!("target.{}.url", target_id), &server_url);
    let _ = state
        .config
        .set(&format!("target.{}.type", target_id), "ntfy");
    if let Some(ref t) = topic {
        let _ = state
            .config
            .set(&format!("target.{}.topic", target_id), t);
    }
    if let Some(ref token) = auth_token {
        let cred_key = format!("ntfy:{}", target_id);
        if let Err(e) = state.credentials.store(&cred_key, token) {
            tracing::warn!(error = %e, "Failed to store ntfy auth in keychain");
        }
    }

    state
        .target_manager
        .register(std::sync::Arc::new(target));

    tracing::info!(target_id = %target_id, "ntfy target connected successfully");
    serde_json::to_value(info).map_err(|e| e.to_string())
}

/// Connect a Make.com target (zone URL + API key)
#[tauri::command]
pub async fn connect_make_target(
    state: State<'_, AppState>,
    zone_url: String,
    api_key: String,
) -> Result<serde_json::Value, String> {
    tracing::info!(command = "connect_make_target", url = %zone_url, "Command invoked");

    let target_id = format!(
        "make-{}",
        uuid::Uuid::new_v4()
            .to_string()
            .split('-')
            .next()
            .unwrap_or("0")
    );
    let target =
        crate::targets::MakeTarget::new(target_id.clone(), zone_url.clone(), api_key.clone(), None);

    // Test connection before persisting
    let info = target.test_connection().await.map_err(|e| {
        tracing::error!(error = %e, "Make.com connection test failed");
        e.to_string()
    })?;

    // Extract team_id from test_connection response
    let team_id = info.details
        .get("team_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // Store API key in keychain
    let cred_key = format!("make:{}", target_id);
    if let Err(e) = state.credentials.store(&cred_key, &api_key) {
        tracing::warn!(error = %e, "Failed to store Make.com API key in keychain");
    }

    // Store URL, type, and team_id in config
    let _ = state
        .config
        .set(&format!("target.{}.url", target_id), &zone_url);
    let _ = state
        .config
        .set(&format!("target.{}.type", target_id), "make");
    if let Some(ref tid) = team_id {
        let _ = state
            .config
            .set(&format!("target.{}.team_id", target_id), tid);
        tracing::debug!(target_id = %target_id, team_id = %tid, "Persisted Make.com team_id");
    }

    // Register target with team_id
    let target_with_team_id = crate::targets::MakeTarget::new(
        target_id.clone(),
        zone_url.clone(),
        api_key.clone(),
        team_id
    );
    state
        .target_manager
        .register(std::sync::Arc::new(target_with_team_id));

    tracing::info!(target_id = %target_id, "Make.com target connected successfully");
    serde_json::to_value(info).map_err(|e| e.to_string())
}

/// Connect a Zapier target (name + webhook URL)
#[tauri::command]
pub async fn connect_zapier_target(
    state: State<'_, AppState>,
    name: String,
    webhook_url: String,
) -> Result<serde_json::Value, String> {
    tracing::info!(command = "connect_zapier_target", url = %webhook_url, "Command invoked");

    let target_id = format!(
        "zapier-{}",
        uuid::Uuid::new_v4()
            .to_string()
            .split('-')
            .next()
            .unwrap_or("0")
    );
    let target = crate::targets::ZapierTarget::new(target_id.clone(), name.clone(), webhook_url.clone())
        .map_err(|e| {
            tracing::error!(error = %e, "Invalid Zapier webhook URL");
            e.to_string()
        })?;

    // Test connection before persisting
    let info = target.test_connection().await.map_err(|e| {
        tracing::error!(error = %e, "Zapier connection test failed");
        e.to_string()
    })?;

    // Store URL, name, and type in config
    let _ = state
        .config
        .set(&format!("target.{}.url", target_id), &webhook_url);
    let _ = state
        .config
        .set(&format!("target.{}.name", target_id), &name);
    let _ = state
        .config
        .set(&format!("target.{}.type", target_id), "zapier");

    // Register target
    state
        .target_manager
        .register(std::sync::Arc::new(target));

    tracing::info!(target_id = %target_id, "Zapier target connected successfully");
    serde_json::to_value(info).map_err(|e| e.to_string())
}

/// Connect a Custom webhook target (any REST endpoint with configurable auth)
#[tauri::command]
pub async fn connect_custom_target(
    state: State<'_, AppState>,
    config: CustomTargetConfig,
) -> Result<serde_json::Value, String> {
    tracing::info!(command = "connect_custom_target", url = %config.webhook_url, auth_type = %config.auth_type, "Command invoked");

    let target_id = format!(
        "custom-{}",
        uuid::Uuid::new_v4()
            .to_string()
            .split('-')
            .next()
            .unwrap_or("0")
    );

    // Parse auth type
    let auth = match config.auth_type.as_str() {
        "none" => crate::targets::AuthType::None,
        "bearer" => {
            let token = config.auth_token.as_ref().ok_or_else(|| {
                tracing::error!("Bearer auth requires token");
                "Bearer auth requires token".to_string()
            })?.clone();
            crate::targets::AuthType::Bearer { token }
        }
        "header" => {
            let name = config.auth_header_name.as_ref().ok_or_else(|| {
                tracing::error!("Header auth requires header name");
                "Header auth requires header name".to_string()
            })?.clone();
            let value = config.auth_header_value.as_ref().ok_or_else(|| {
                tracing::error!("Header auth requires header value");
                "Header auth requires header value".to_string()
            })?.clone();
            crate::targets::AuthType::Header { name, value }
        }
        "basic" => {
            let username = config.auth_username.as_ref().ok_or_else(|| {
                tracing::error!("Basic auth requires username");
                "Basic auth requires username".to_string()
            })?.clone();
            let password = config.auth_password.as_ref().ok_or_else(|| {
                tracing::error!("Basic auth requires password");
                "Basic auth requires password".to_string()
            })?.clone();
            crate::targets::AuthType::Basic { username, password }
        }
        _ => {
            tracing::error!(auth_type = %config.auth_type, "Invalid auth type");
            return Err(format!("Invalid auth type: {}", config.auth_type));
        }
    };

    // Create target
    let target =
        crate::targets::CustomTarget::new(target_id.clone(), config.name.clone(), config.webhook_url.clone(), auth)
            .map_err(|e| {
                tracing::error!(error = %e, "Invalid custom webhook configuration");
                e.to_string()
            })?;

    // Test connection before persisting
    let info = target.test_connection().await.map_err(|e| {
        tracing::error!(error = %e, "Custom webhook connection test failed");
        e.to_string()
    })?;

    // Store specific auth secrets in credential store
    match config.auth_type.as_str() {
        "bearer" => {
            if let Some(ref token) = config.auth_token {
                let cred_key = format!("custom:{}:token", target_id);
                if let Err(e) = state.credentials.store(&cred_key, token) {
                    tracing::warn!(error = %e, "Failed to store bearer token in keychain");
                }
            }
        }
        "header" => {
            if let Some(ref value) = config.auth_header_value {
                let cred_key = format!("custom:{}:header_value", target_id);
                if let Err(e) = state.credentials.store(&cred_key, value) {
                    tracing::warn!(error = %e, "Failed to store header value in keychain");
                }
            }
        }
        "basic" => {
            if let Some(ref password) = config.auth_password {
                let cred_key = format!("custom:{}:password", target_id);
                if let Err(e) = state.credentials.store(&cred_key, password) {
                    tracing::warn!(error = %e, "Failed to store password in keychain");
                }
            }
        }
        _ => {}
    }

    // Store URL, name, auth_type, and other metadata in config
    let _ = state
        .config
        .set(&format!("target.{}.url", target_id), &config.webhook_url);
    let _ = state
        .config
        .set(&format!("target.{}.name", target_id), &config.name);
    let _ = state
        .config
        .set(&format!("target.{}.type", target_id), "custom");
    let _ = state
        .config
        .set(&format!("target.{}.auth_type", target_id), &config.auth_type);

    // Store non-secret auth metadata
    if config.auth_type == "header" {
        if let Some(ref header_name) = config.auth_header_name {
            let _ = state
                .config
                .set(&format!("target.{}.auth_header_name", target_id), header_name);
        }
    } else if config.auth_type == "basic" {
        if let Some(ref username) = config.auth_username {
            let _ = state
                .config
                .set(&format!("target.{}.auth_username", target_id), username);
        }
    }

    // Register target
    state
        .target_manager
        .register(std::sync::Arc::new(target));

    tracing::info!(target_id = %target_id, "Custom webhook target connected successfully");
    serde_json::to_value(info).map_err(|e| e.to_string())
}

/// List all registered targets
#[tauri::command]
pub async fn list_targets(state: State<'_, AppState>) -> Result<Vec<serde_json::Value>, String> {
    tracing::info!(command = "list_targets", "Command invoked");
    let targets = state.target_manager.list();
    Ok(targets
        .into_iter()
        .map(|(id, name, target_type)| {
            serde_json::json!({ "id": id, "name": name, "target_type": target_type })
        })
        .collect())
}

/// Test connection to a specific target
#[tauri::command]
pub async fn test_target_connection(
    state: State<'_, AppState>,
    target_id: String,
) -> Result<serde_json::Value, String> {
    tracing::info!(command = "test_target_connection", target_id = %target_id, "Command invoked");
    let info = state
        .target_manager
        .test_connection(&target_id)
        .await
        .map_err(|e| e.to_string())?;
    serde_json::to_value(info).map_err(|e| e.to_string())
}

/// Get health status for all targets, including degradation info and queued count.
#[tauri::command]
pub async fn get_target_health(
    state: State<'_, AppState>,
) -> Result<Vec<serde_json::Value>, String> {
    tracing::debug!(command = "get_target_health", "Command invoked");
    let targets = state.target_manager.list();
    let mut result = Vec::new();

    for (target_id, target_name, target_type) in &targets {
        let degradation = state.health_tracker.is_degraded(target_id);

        // Get endpoint_ids for this target to count paused deliveries
        let endpoint_ids: Vec<String> = state.binding_store.list_all()
            .into_iter()
            .filter(|b| b.target_id == *target_id)
            .map(|b| b.endpoint_id)
            .collect();
        let ep_refs: Vec<&str> = endpoint_ids.iter().map(|s| s.as_str()).collect();
        let queued_count = state.ledger
            .count_paused_for_target(&ep_refs)
            .unwrap_or(0);

        let health = if let Some(info) = degradation {
            serde_json::json!({
                "target_id": target_id,
                "target_name": target_name,
                "target_type": target_type,
                "status": "degraded",
                "reason": info.reason,
                "degraded_at": info.degraded_at,
                "queued_count": queued_count,
            })
        } else {
            serde_json::json!({
                "target_id": target_id,
                "target_name": target_name,
                "target_type": target_type,
                "status": "healthy",
                "queued_count": 0,
            })
        };
        result.push(health);
    }

    Ok(result)
}

/// Reconnect a degraded target: test connection, and if successful,
/// mark as healthy and resume all paused deliveries.
#[tauri::command]
pub async fn reconnect_target(
    state: State<'_, AppState>,
    target_id: String,
) -> Result<serde_json::Value, String> {
    tracing::info!(command = "reconnect_target", target_id = %target_id, "Command invoked");

    // Test the target's connection
    match state.target_manager.test_connection(&target_id).await {
        Ok(info) => {
            // Connection succeeded — mark healthy and resume deliveries
            state.health_tracker.mark_reconnected(&target_id);

            let endpoint_ids: Vec<String> = state.binding_store.list_all()
                .into_iter()
                .filter(|b| b.target_id == target_id)
                .map(|b| b.endpoint_id)
                .collect();
            let ep_refs: Vec<&str> = endpoint_ids.iter().map(|s| s.as_str()).collect();
            let resumed = state.ledger
                .resume_target_deliveries(&ep_refs)
                .map_err(|e| format!("Failed to resume deliveries: {}", e))?;

            tracing::info!(
                target_id = %target_id,
                resumed_count = resumed,
                "Target reconnected and deliveries resumed"
            );

            Ok(serde_json::json!({
                "target_id": target_id,
                "status": "healthy",
                "resumed_count": resumed,
                "target_info": serde_json::to_value(info).unwrap_or_default(),
            }))
        }
        Err(e) => {
            let err_str = e.to_string();
            let needs_reauth = err_str.contains("Token") || err_str.contains("Auth") || err_str.contains("401") || err_str.contains("403");
            tracing::warn!(target_id = %target_id, error = %err_str, needs_reauth = %needs_reauth, "Reconnect failed");

            if needs_reauth {
                Err("Re-authentication required. Please re-authenticate in Settings.".to_string())
            } else {
                Err(format!("Reconnect failed: {}", err_str))
            }
        }
    }
}

/// List endpoints for a specific target.
/// Falls back to existing bindings if the live endpoint listing fails (e.g. expired token).
#[tauri::command]
pub async fn list_target_endpoints(
    state: State<'_, AppState>,
    target_id: String,
) -> Result<Vec<serde_json::Value>, String> {
    tracing::info!(command = "list_target_endpoints", target_id = %target_id, "Command invoked");

    // Try live endpoint listing first
    match state.target_manager.list_endpoints(&target_id).await {
        Ok(endpoints) => {
            endpoints
                .into_iter()
                .map(|e| serde_json::to_value(e).map_err(|e| e.to_string()))
                .collect()
        }
        Err(e) => {
            tracing::warn!(target_id = %target_id, error = %e, "Live endpoint listing failed, falling back to existing bindings");

            // Fall back to endpoints from existing bindings for this target (deduplicated)
            // Infer auth_type from the registered target's type
            let auth_type = state.target_manager.get(&target_id)
                .map(|t| match t.target_type() {
                    "google-sheets" => "oauth2",
                    _ => "custom",
                })
                .unwrap_or("custom");

            let bindings = state.binding_store.list_all();
            let mut seen = std::collections::HashSet::new();
            let fallback: Vec<serde_json::Value> = bindings
                .into_iter()
                .filter(|b| b.target_id == target_id && seen.insert(b.endpoint_id.clone()))
                .map(|b| serde_json::json!({
                    "id": b.endpoint_id,
                    "name": b.endpoint_name,
                    "url": b.endpoint_url,
                    "authenticated": true,
                    "auth_type": auth_type,
                    "metadata": {}
                }))
                .collect();

            if fallback.is_empty() {
                Err(format!("Target error: {}", e))
            } else {
                tracing::info!(target_id = %target_id, count = fallback.len(), "Returning cached endpoints from bindings");
                Ok(fallback)
            }
        }
    }
}

/// Create a binding from a source to a target endpoint
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn create_binding(
    state: State<'_, AppState>,
    source_id: String,
    target_id: String,
    endpoint_id: String,
    endpoint_url: String,
    endpoint_name: String,
    custom_headers: Option<Vec<(String, String)>>,
    auth_header_name: Option<String>,
    auth_header_value: Option<String>,
    preserve_auth_credential_key: Option<String>,
    delivery_mode: Option<String>,
    schedule_time: Option<String>,
    schedule_day: Option<String>,
) -> Result<(), String> {
    tracing::info!(
        command = "create_binding",
        source_id = %source_id,
        endpoint_id = %endpoint_id,
        has_custom_headers = custom_headers.is_some(),
        has_auth = auth_header_name.is_some(),
        preserve_existing_auth = preserve_auth_credential_key.is_some(),
        "Command invoked"
    );

    // Build headers_json: combine custom headers + auth header name (placeholder for secret)
    let mut all_headers: Vec<(String, String)> = custom_headers.unwrap_or_default();
    let mut auth_credential_key = None;

    if let Some(ref auth_name) = auth_header_name {
        // Add auth header with empty value as placeholder (secret stored separately)
        all_headers.push((auth_name.clone(), String::new()));

        if let Some(ref auth_value) = auth_header_value {
            if !auth_value.is_empty() {
                // New auth value provided — store in credential store
                let cred_key = format!("binding:{}:{}", source_id, endpoint_id);
                state.credentials.store(&cred_key, auth_value).map_err(|e| {
                    tracing::error!(error = %e, "Failed to store binding credential");
                    e.to_string()
                })?;
                auth_credential_key = Some(cred_key);
            } else if let Some(ref existing_key) = preserve_auth_credential_key {
                // No new value but existing credential key — preserve it
                tracing::debug!(key = %existing_key, "Preserving existing auth credential key");
                auth_credential_key = Some(existing_key.clone());
            }
        } else if let Some(ref existing_key) = preserve_auth_credential_key {
            // No auth value at all but existing credential key — preserve it
            tracing::debug!(key = %existing_key, "Preserving existing auth credential key");
            auth_credential_key = Some(existing_key.clone());
        }
    }

    let headers_json = if all_headers.is_empty() {
        None
    } else {
        Some(serde_json::to_string(&all_headers).map_err(|e| e.to_string())?)
    };

    let binding = SourceBinding {
        source_id,
        target_id,
        endpoint_id,
        endpoint_url,
        endpoint_name,
        created_at: chrono::Utc::now().timestamp(),
        active: true,
        headers_json,
        auth_credential_key,
        delivery_mode: delivery_mode.unwrap_or_else(|| "on_change".to_string()),
        schedule_time,
        schedule_day,
        last_scheduled_at: None,
    };
    state.binding_store.save(&binding)
}

/// Remove a binding
#[tauri::command]
pub fn remove_binding(
    state: State<'_, AppState>,
    source_id: String,
    endpoint_id: String,
) -> Result<(), String> {
    tracing::info!(
        command = "remove_binding",
        source_id = %source_id,
        endpoint_id = %endpoint_id,
        "Command invoked"
    );
    state.binding_store.remove(&source_id, &endpoint_id)
}

/// Get all bindings for a source
#[tauri::command]
pub fn get_source_bindings(
    state: State<'_, AppState>,
    source_id: String,
) -> Result<Vec<serde_json::Value>, String> {
    tracing::info!(command = "get_source_bindings", source_id = %source_id, "Command invoked");
    let bindings = state.binding_store.get_for_source(&source_id);
    bindings
        .into_iter()
        .map(|b| serde_json::to_value(b).map_err(|e| e.to_string()))
        .collect()
}

/// List all bindings across all sources
#[tauri::command]
pub fn list_all_bindings(
    state: State<'_, AppState>,
) -> Result<Vec<serde_json::Value>, String> {
    tracing::info!(command = "list_all_bindings", "Command invoked");
    let bindings = state.binding_store.list_all();
    bindings
        .into_iter()
        .map(|b| serde_json::to_value(b).map_err(|e| e.to_string()))
        .collect()
}

/// Manually trigger a source to parse and enqueue for delivery.
/// The delivery worker will pick it up on the next poll cycle (≤5s).
/// Payload is filtered based on enabled properties before enqueue.
#[tauri::command]
pub fn trigger_source_push(
    state: State<'_, AppState>,
    source_id: String,
) -> Result<String, String> {
    tracing::info!(command = "trigger_source_push", source_id = %source_id, "Command invoked");

    // Parse and filter payload based on enabled properties
    let payload = state.source_manager.parse_and_filter(&source_id).map_err(|e| {
        tracing::error!(source_id = %source_id, error = %e, "Source parse/filter failed");
        e.to_string()
    })?;

    // "Push Now" delivers to ALL bindings (including scheduled ones), not just on_change.
    // Create one targeted delivery per binding for independent tracking.
    let bindings = state.binding_store.get_for_source(&source_id);
    if bindings.is_empty() {
        // No bindings — fall back to legacy fan-out (global webhook)
        let event_id = state.ledger.enqueue_manual(&source_id, payload).map_err(|e| {
            tracing::error!(source_id = %source_id, error = %e, "Ledger enqueue failed");
            e.to_string()
        })?;
        tracing::info!(source_id = %source_id, event_id = %event_id, "Manual push enqueued (legacy fan-out)");
        return Ok(event_id);
    }

    let mut first_event_id = String::new();
    for binding in &bindings {
        let event_id = state.ledger.enqueue_manual_targeted(
            &source_id,
            payload.clone(),
            &binding.endpoint_id,
        ).map_err(|e| {
            tracing::error!(source_id = %source_id, endpoint = %binding.endpoint_id, error = %e, "Ledger enqueue failed");
            e.to_string()
        })?;

        // Write target display info immediately so the activity log shows it
        let (target_type, base_url) = state.target_manager
            .get(&binding.target_id)
            .map(|t| (t.target_type().to_string(), t.base_url().to_string()))
            .unwrap_or_else(|| ("webhook".to_string(), String::new()));
        let target_json = binding.build_delivered_to_json(&target_type, &base_url);
        let _ = state.ledger.set_attempted_target(&event_id, &target_json);

        tracing::info!(
            source_id = %source_id,
            endpoint_id = %binding.endpoint_id,
            endpoint_name = %binding.endpoint_name,
            event_id = %event_id,
            "Manual push enqueued (targeted)"
        );
        if first_event_id.is_empty() {
            first_event_id = event_id;
        }
    }

    tracing::info!(source_id = %source_id, binding_count = bindings.len(), "Manual push enqueued to all bindings");
    Ok(first_event_id)
}

/// Replay a delivery: re-enqueue the exact same payload for redelivery
#[tauri::command]
pub fn replay_delivery(
    state: State<'_, AppState>,
    event_type: String,
    payload: serde_json::Value,
) -> Result<String, String> {
    tracing::info!(command = "replay_delivery", event_type = %event_type, "Command invoked");

    let event_id = state.ledger.enqueue(&event_type, payload).map_err(|e| {
        tracing::error!(event_type = %event_type, error = %e, "Replay enqueue failed");
        e.to_string()
    })?;

    tracing::info!(event_type = %event_type, event_id = %event_id, "Replay enqueued");
    Ok(event_id)
}

/// Connect a Google Sheets target (OAuth2 tokens from frontend)
#[tauri::command]
pub async fn connect_google_sheets_target(
    state: State<'_, AppState>,
    email: String,
    access_token: String,
    refresh_token: String,
    expires_at: i64,
    client_id: String,
    client_secret: String,
) -> Result<serde_json::Value, String> {
    tracing::info!(command = "connect_google_sheets_target", email = %email, "Command invoked");

    let target_id = format!(
        "gsheets-{}",
        uuid::Uuid::new_v4()
            .to_string()
            .split('-')
            .next()
            .unwrap_or("0")
    );

    let tokens = crate::targets::google_sheets::GoogleTokens {
        access_token,
        refresh_token,
        expires_at,
        client_id,
        client_secret,
    };

    let target = crate::targets::GoogleSheetsTarget::new(
        target_id.clone(),
        email.clone(),
        tokens.clone(),
    );

    // Test connection before persisting
    let info = target.test_connection().await.map_err(|e| {
        tracing::error!(error = %e, "Google Sheets connection test failed");
        e.to_string()
    })?;

    // Store tokens in credential store
    let cred_key = format!("google-sheets:{}", target_id);
    let tokens_json = serde_json::to_string(&tokens).map_err(|e| e.to_string())?;
    if let Err(e) = state.credentials.store(&cred_key, &tokens_json) {
        tracing::warn!(error = %e, "Failed to store Google Sheets tokens");
    }

    // Store config
    let _ = state
        .config
        .set(&format!("target.{}.url", target_id), "https://sheets.google.com");
    let _ = state
        .config
        .set(&format!("target.{}.type", target_id), "google-sheets");
    let _ = state
        .config
        .set(&format!("target.{}.email", target_id), &email);

    // Register target
    state
        .target_manager
        .register(std::sync::Arc::new(target));

    tracing::info!(target_id = %target_id, email = %email, "Google Sheets target connected successfully");
    serde_json::to_value(info).map_err(|e| e.to_string())
}

/// Re-authenticate an existing Google Sheets target with fresh OAuth tokens.
/// Preserves the target_id and all existing bindings.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn reauth_google_sheets_target(
    state: State<'_, AppState>,
    target_id: String,
    email: String,
    access_token: String,
    refresh_token: String,
    expires_at: i64,
    client_id: String,
    client_secret: String,
) -> Result<serde_json::Value, String> {
    tracing::info!(command = "reauth_google_sheets_target", target_id = %target_id, email = %email, "Command invoked");

    // Verify target exists
    if state.target_manager.get(&target_id).is_none() {
        return Err(format!("Target {} not found", target_id));
    }

    let tokens = crate::targets::google_sheets::GoogleTokens {
        access_token,
        refresh_token,
        expires_at,
        client_id,
        client_secret,
    };

    // Create new target instance with same ID
    let target = crate::targets::GoogleSheetsTarget::new(
        target_id.clone(),
        email.clone(),
        tokens.clone(),
    );

    // Test connection — fail early if tokens are bad
    let info = target.test_connection().await.map_err(|e| {
        tracing::error!(error = %e, "Google Sheets re-auth connection test failed");
        e.to_string()
    })?;

    // Update credential store
    let cred_key = format!("google-sheets:{}", target_id);
    let tokens_json = serde_json::to_string(&tokens).map_err(|e| e.to_string())?;
    if let Err(e) = state.credentials.store(&cred_key, &tokens_json) {
        tracing::warn!(error = %e, "Failed to store updated Google Sheets tokens");
    }

    // Update email in config (may have changed)
    let _ = state
        .config
        .set(&format!("target.{}.email", target_id), &email);

    // Re-register — HashMap::insert replaces old entry, bindings still reference same target_id
    state
        .target_manager
        .register(std::sync::Arc::new(target));

    // Mark healthy and resume paused deliveries
    state.health_tracker.mark_reconnected(&target_id);

    let endpoint_ids: Vec<String> = state.binding_store.list_all()
        .into_iter()
        .filter(|b| b.target_id == target_id)
        .map(|b| b.endpoint_id)
        .collect();
    let ep_refs: Vec<&str> = endpoint_ids.iter().map(|s| s.as_str()).collect();
    let resumed = state.ledger
        .resume_target_deliveries(&ep_refs)
        .unwrap_or(0);

    tracing::info!(
        target_id = %target_id,
        email = %email,
        resumed_count = resumed,
        "Google Sheets target re-authenticated successfully"
    );

    Ok(serde_json::json!({
        "target_id": target_id,
        "status": "healthy",
        "resumed_count": resumed,
        "target_info": serde_json::to_value(info).unwrap_or_default(),
    }))
}

/// Get available properties for a source
#[tauri::command]
pub fn get_source_properties(
    state: State<'_, AppState>,
    source_id: String,
) -> Result<Vec<PropertyState>, String> {
    tracing::info!(command = "get_source_properties", source_id = %source_id, "Command invoked");

    let source = state
        .source_manager
        .get_source(&source_id)
        .ok_or_else(|| format!("Source {} not found", source_id))?;

    let available_properties = source.available_properties();
    let config_store = SourceConfigStore::new(state.config.clone());
    let property_states = config_store.get_all(&source_id, &available_properties);

    tracing::debug!(
        source_id = %source_id,
        property_count = property_states.len(),
        "Source properties retrieved"
    );

    Ok(property_states)
}

/// Set a source property on/off
#[tauri::command]
pub fn set_source_property(
    state: State<'_, AppState>,
    source_id: String,
    property: String,
    enabled: bool,
) -> Result<(), String> {
    tracing::info!(
        command = "set_source_property",
        source_id = %source_id,
        property = %property,
        enabled = enabled,
        "Command invoked"
    );

    // Verify source exists
    state
        .source_manager
        .get_source(&source_id)
        .ok_or_else(|| format!("Source {} not found", source_id))?;

    let config_store = SourceConfigStore::new(state.config.clone());
    config_store.set_enabled(&source_id, &property, enabled)?;

    tracing::info!(
        source_id = %source_id,
        property = %property,
        enabled = enabled,
        "Source property updated"
    );

    Ok(())
}

/// Get error diagnosis for a failed delivery
#[tauri::command]
pub fn get_error_diagnosis(
    state: State<'_, AppState>,
    entry_id: String,
) -> Result<crate::error_diagnosis::ErrorDiagnosis, String> {
    tracing::debug!(command = "get_error_diagnosis", entry_id = %entry_id, "Command invoked");

    // Find the entry by iterating through all statuses
    let mut entry = None;
    for status in [
        DeliveryStatus::Failed,
        DeliveryStatus::Dlq,
        DeliveryStatus::TargetPaused,
        DeliveryStatus::Delivered,
    ] {
        if let Ok(entries) = state.ledger.get_by_status(status) {
            if let Some(e) = entries.into_iter().find(|e| e.id == entry_id) {
                entry = Some(e);
                break;
            }
        }
    }

    let entry = entry.ok_or_else(|| {
        tracing::error!(entry_id = %entry_id, "Delivery entry not found");
        format!("Entry {} not found", entry_id)
    })?;

    // Extract HTTP status code from error text if present
    let status_code = entry.last_error.as_ref().and_then(|err| {
        if let Some(start) = err.find("HTTP ") {
            if let Some(code_str) = err[start + 5..].split_whitespace().next() {
                code_str.parse::<u16>().ok()
            } else {
                None
            }
        } else {
            None
        }
    });

    let error_text = entry.last_error.as_deref().unwrap_or("Unknown error");

    // Get source and endpoint names for better context
    let source_name = entry.event_type.replace('-', " ");
    let endpoint_name = entry.target_endpoint_id.as_deref().unwrap_or("target");

    let diagnosis = crate::error_diagnosis::diagnose_error(
        status_code,
        error_text,
        &source_name,
        endpoint_name,
    );

    tracing::debug!(
        entry_id = %entry_id,
        category = ?diagnosis.category,
        "Error diagnosis generated"
    );

    Ok(diagnosis)
}

/// Get retry history for a delivery entry
#[tauri::command]
pub fn get_retry_history(
    state: State<'_, AppState>,
    entry_id: String,
) -> Result<Vec<serde_json::Value>, String> {
    tracing::debug!(command = "get_retry_history", entry_id = %entry_id, "Command invoked");

    // Query retry_log directly from the ledger
    match state.ledger.get_retry_history(&entry_id) {
        Ok(history) => {
            tracing::debug!(
                entry_id = %entry_id,
                attempts = history.len(),
                "Retry history retrieved"
            );
            Ok(history)
        }
        Err(e) => {
            tracing::error!(entry_id = %entry_id, error = %e, "Failed to get retry history");
            Err(e.to_string())
        }
    }
}

/// Get count of DLQ entries
#[tauri::command]
pub fn get_dlq_count(state: State<'_, AppState>) -> Result<u32, String> {
    tracing::debug!(command = "get_dlq_count", "Command invoked");

    match state.ledger.get_stats() {
        Ok(stats) => {
            tracing::debug!(dlq_count = stats.dlq, "DLQ count retrieved");
            Ok(stats.dlq as u32)
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to get DLQ count");
            Err(e.to_string())
        }
    }
}

/// Dismiss a DLQ entry (marks it as handled)
#[tauri::command]
pub fn dismiss_dlq_entry(
    state: State<'_, AppState>,
    entry_id: String,
) -> Result<(), String> {
    tracing::info!(command = "dismiss_dlq_entry", entry_id = %entry_id, "Command invoked");

    // Find the entry by event_id (convert entry_id to event_id)
    let mut event_id_opt = None;
    if let Ok(dlq_entries) = state.ledger.get_by_status(DeliveryStatus::Dlq) {
        if let Some(entry) = dlq_entries.into_iter().find(|e| e.id == entry_id) {
            event_id_opt = Some(entry.event_id);
        }
    }

    let event_id = event_id_opt.ok_or_else(|| {
        tracing::error!(entry_id = %entry_id, "DLQ entry not found");
        format!("DLQ entry {} not found", entry_id)
    })?;

    // Dismiss DLQ entry (transitions dlq → delivered)
    state.ledger.dismiss_dlq(&event_id).map_err(|e| {
        tracing::error!(event_id = %event_id, error = %e, "Failed to dismiss DLQ entry");
        e.to_string()
    })?;

    tracing::info!(entry_id = %entry_id, "DLQ entry dismissed");
    Ok(())
}

/// Replay a delivery by creating a new pending entry with the same payload
#[tauri::command]
pub fn replay_delivery_by_id(
    state: State<'_, AppState>,
    entry_id: String,
) -> Result<String, String> {
    tracing::info!(command = "replay_delivery_by_id", entry_id = %entry_id, "Command invoked");

    // Find the entry
    let mut entry = None;
    for status in [
        DeliveryStatus::Failed,
        DeliveryStatus::Dlq,
        DeliveryStatus::TargetPaused,
        DeliveryStatus::Delivered,
    ] {
        if let Ok(entries) = state.ledger.get_by_status(status) {
            if let Some(e) = entries.into_iter().find(|e| e.id == entry_id) {
                entry = Some(e);
                break;
            }
        }
    }

    let entry = entry.ok_or_else(|| {
        tracing::error!(entry_id = %entry_id, "Delivery entry not found for replay");
        format!("Entry {} not found", entry_id)
    })?;

    // Re-enqueue with the same payload and target
    let new_event_id = if let Some(ref target_id) = entry.target_endpoint_id {
        state.ledger.enqueue_targeted(&entry.event_type, entry.payload, target_id)
    } else {
        state.ledger.enqueue(&entry.event_type, entry.payload)
    }.map_err(|e| {
        tracing::error!(entry_id = %entry_id, error = %e, "Failed to replay delivery");
        e.to_string()
    })?;

    // Carry forward target display info from original entry if available
    if let Some(ref target_ep_id) = entry.target_endpoint_id {
        // Look up binding for this source+endpoint to build display JSON
        let binding = state.binding_store
            .get_for_source(&entry.event_type)
            .into_iter()
            .find(|b| b.endpoint_id == *target_ep_id);
        if let Some(binding) = binding {
            let (target_type, base_url) = state.target_manager
                .get(&binding.target_id)
                .map(|t| (t.target_type().to_string(), t.base_url().to_string()))
                .unwrap_or_else(|| ("webhook".to_string(), String::new()));
            let target_json = binding.build_delivered_to_json(&target_type, &base_url);
            let _ = state.ledger.set_attempted_target(&new_event_id, &target_json);
        }
    }

    tracing::info!(
        entry_id = %entry_id,
        new_event_id = %new_event_id,
        "Delivery replayed successfully"
    );

    Ok(new_event_id)
}

/// Open the feedback/issues page in the default browser
#[tauri::command]
pub fn open_feedback() -> Result<(), String> {
    tracing::info!(command = "open_feedback", "Command invoked");
    open::that("https://github.com/madshn/localpush/issues")
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to open feedback URL");
            format!("Failed to open browser: {}", e)
        })
}

/// Timeline gap structure for scheduled deliveries that didn't happen
#[derive(Debug, Serialize)]
pub struct TimelineGap {
    pub source_id: String,
    pub source_name: String,
    pub binding_id: String,
    pub expected_at: String,
    pub delivery_mode: String,
    pub last_delivered_at: Option<String>,
}

/// Get timeline gaps for scheduled deliveries
#[tauri::command]
pub fn get_timeline_gaps(
    state: State<'_, AppState>,
) -> Result<Vec<TimelineGap>, String> {
    tracing::debug!(command = "get_timeline_gaps", "Command invoked");

    let mut gaps = Vec::new();
    let bindings = state.binding_store.get_scheduled_bindings();
    let now = chrono::Local::now();

    for binding in bindings {
        // Interval bindings store schedule_time as minutes (e.g. "15"), not HH:MM.
        // Timeline gaps only apply to daily/weekly modes with a fixed target time.
        if binding.delivery_mode == "interval" {
            continue;
        }

        // Parse schedule time
        let schedule_time = match &binding.schedule_time {
            Some(t) => t,
            None => continue,
        };

        let target_time = match chrono::NaiveTime::parse_from_str(schedule_time, "%H:%M") {
            Ok(t) => t,
            Err(_) => {
                tracing::warn!(
                    source_id = %binding.source_id,
                    schedule_time = %schedule_time,
                    "Invalid schedule_time format"
                );
                continue;
            }
        };

        // Calculate expected delivery time for today
        let today_target = now
            .date_naive()
            .and_time(target_time);
        let today_target_ts = match today_target
            .and_local_timezone(now.timezone())
            .single()
        {
            Some(dt) => dt.timestamp(),
            None => continue,
        };

        // Check if we're past the expected delivery time
        if now.timestamp() < today_target_ts {
            continue; // Not yet time for today's delivery
        }

        // For weekly: check day of week
        if binding.delivery_mode == "weekly" {
            let target_day = match binding.schedule_day.as_deref() {
                Some(d) => match parse_weekday_for_gaps(d) {
                    Some(wd) => wd,
                    None => continue,
                },
                None => continue,
            };

            if now.weekday() != target_day {
                continue; // Not the right day for weekly delivery
            }
        }

        // Check if delivery happened after today's target time
        let has_delivered_today = binding.last_scheduled_at
            .map(|last| last >= today_target_ts)
            .unwrap_or(false);

        if !has_delivered_today {
            // There's a gap - expected delivery didn't happen
            let source = state.source_manager.get_source(&binding.source_id);
            let source_name = source
                .map(|s| s.name().to_string())
                .unwrap_or_else(|| binding.source_id.clone());

            gaps.push(TimelineGap {
                source_id: binding.source_id.clone(),
                source_name,
                binding_id: format!("{}.{}", binding.source_id, binding.endpoint_id),
                expected_at: chrono::DateTime::from_timestamp(today_target_ts, 0)
                    .map(|dt| dt.to_rfc3339())
                    .unwrap_or_default(),
                delivery_mode: binding.delivery_mode.clone(),
                last_delivered_at: binding.last_scheduled_at
                    .and_then(|ts| chrono::DateTime::from_timestamp(ts, 0))
                    .map(|dt| dt.to_rfc3339()),
            });
        }
    }

    tracing::debug!(gaps_found = gaps.len(), "Timeline gaps retrieved");
    Ok(gaps)
}

fn parse_weekday_for_gaps(s: &str) -> Option<chrono::Weekday> {
    use chrono::Weekday;
    match s.to_lowercase().as_str() {
        "monday" => Some(Weekday::Mon),
        "tuesday" => Some(Weekday::Tue),
        "wednesday" => Some(Weekday::Wed),
        "thursday" => Some(Weekday::Thu),
        "friday" => Some(Weekday::Fri),
        "saturday" => Some(Weekday::Sat),
        "sunday" => Some(Weekday::Sun),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_weekday_for_gaps() {
        use chrono::Weekday;
        assert_eq!(parse_weekday_for_gaps("monday"), Some(Weekday::Mon));
        assert_eq!(parse_weekday_for_gaps("TUESDAY"), Some(Weekday::Tue));
        assert_eq!(parse_weekday_for_gaps("Sunday"), Some(Weekday::Sun));
        assert_eq!(parse_weekday_for_gaps("invalid"), None);
    }
}
