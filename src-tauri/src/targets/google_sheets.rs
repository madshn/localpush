//! Google Sheets push target
//!
//! Delivers payloads by appending rows to Google Sheets spreadsheets.
//! Auth: OAuth2 with token refresh. Endpoints: user's spreadsheets via Drive API.
//! Worksheets are auto-created per source at delivery time.

use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::traits::{CredentialStore, Target, TargetEndpoint, TargetError, TargetInfo};

/// OAuth2 tokens for Google API access
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleTokens {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: i64,
    pub client_id: String,
    pub client_secret: String,
}

/// A push target backed by a Google Sheets account
pub struct GoogleSheetsTarget {
    id: String,
    email: String,
    tokens: GoogleTokens,
    client: Client,
}

#[derive(Deserialize)]
struct TokenRefreshResponse {
    access_token: String,
    expires_in: i64,
}

#[derive(Deserialize)]
struct DriveFileList {
    files: Vec<DriveFile>,
}

#[derive(Deserialize)]
struct DriveFile {
    id: String,
    name: String,
}

#[derive(Deserialize)]
struct SheetProperties {
    properties: SheetMeta,
}

#[derive(Deserialize)]
struct SheetMeta {
    title: String,
}

#[derive(Deserialize)]
struct SpreadsheetDetail {
    sheets: Vec<SheetProperties>,
}

impl GoogleSheetsTarget {
    pub fn new(id: String, email: String, tokens: GoogleTokens) -> Self {
        Self {
            id,
            email,
            tokens,
            client: Client::new(),
        }
    }

    /// Get a valid access token, refreshing if expired.
    /// Updates the credential store with new tokens on refresh.
    async fn get_valid_token(
        &self,
        credentials: &dyn CredentialStore,
    ) -> Result<String, TargetError> {
        let now = chrono::Utc::now().timestamp();
        if now < self.tokens.expires_at - 60 {
            // Token still valid (with 60s buffer)
            return Ok(self.tokens.access_token.clone());
        }

        // Refresh the token
        let new_tokens = self.refresh_token().await?;

        // Update credential store with refreshed tokens
        let cred_key = format!("google-sheets:{}", self.id);
        let mut updated = self.tokens.clone();
        updated.access_token = new_tokens.access_token.clone();
        updated.expires_at = now + new_tokens.expires_in;
        let json = serde_json::to_string(&updated)
            .map_err(|e| TargetError::DeliveryError(format!("Failed to serialize tokens: {}", e)))?;
        let _ = credentials.store(&cred_key, &json);

        Ok(new_tokens.access_token)
    }

    /// Refresh the OAuth2 access token using the refresh token.
    async fn refresh_token(&self) -> Result<TokenRefreshResponse, TargetError> {
        let resp = self
            .client
            .post("https://oauth2.googleapis.com/token")
            .form(&[
                ("client_id", self.tokens.client_id.as_str()),
                ("client_secret", self.tokens.client_secret.as_str()),
                ("refresh_token", self.tokens.refresh_token.as_str()),
                ("grant_type", "refresh_token"),
            ])
            .send()
            .await
            .map_err(|e| TargetError::ConnectionFailed(format!("Token refresh failed: {}", e)))?;

        if resp.status() == 401 || resp.status() == 403 {
            return Err(TargetError::TokenExpired);
        }
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(TargetError::AuthFailed(format!(
                "Token refresh HTTP {}: {}",
                status, body
            )));
        }

        resp.json()
            .await
            .map_err(|e| TargetError::DeliveryError(format!("Failed to parse token response: {}", e)))
    }

    /// List user's spreadsheets via Google Drive API.
    async fn list_spreadsheets(
        &self,
        access_token: &str,
    ) -> Result<Vec<DriveFile>, TargetError> {
        let resp = self
            .client
            .get("https://www.googleapis.com/drive/v3/files")
            .query(&[
                ("q", "mimeType='application/vnd.google-apps.spreadsheet'"),
                ("fields", "files(id,name)"),
                ("pageSize", "100"),
                ("orderBy", "modifiedTime desc"),
            ])
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|e| TargetError::ConnectionFailed(e.to_string()))?;

        if resp.status() == 401 || resp.status() == 403 {
            return Err(TargetError::AuthFailed("Drive API access denied".to_string()));
        }
        if !resp.status().is_success() {
            return Err(TargetError::ConnectionFailed(format!(
                "Drive API HTTP {}",
                resp.status()
            )));
        }

        let file_list: DriveFileList = resp
            .json()
            .await
            .map_err(|e| TargetError::DeliveryError(format!("Failed to parse Drive response: {}", e)))?;

        Ok(file_list.files)
    }

    /// Ensure a worksheet (tab) exists in the spreadsheet. Creates it if missing.
    async fn ensure_worksheet(
        &self,
        access_token: &str,
        spreadsheet_id: &str,
        sheet_name: &str,
    ) -> Result<bool, TargetError> {
        // Get existing sheets. Returns true if sheet was newly created.
        let url = format!(
            "https://sheets.googleapis.com/v4/spreadsheets/{}?fields=sheets.properties.title",
            spreadsheet_id
        );
        let resp = self
            .client
            .get(&url)
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|e| TargetError::ConnectionFailed(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(TargetError::DeliveryError(format!(
                "Failed to get spreadsheet details: HTTP {}",
                resp.status()
            )));
        }

        let detail: SpreadsheetDetail = resp
            .json()
            .await
            .map_err(|e| TargetError::DeliveryError(format!("Failed to parse spreadsheet: {}", e)))?;

        // Check if sheet already exists
        let exists = detail
            .sheets
            .iter()
            .any(|s| s.properties.title == sheet_name);

        if exists {
            return Ok(false); // Not newly created
        }

        // Create the sheet via batchUpdate
        let batch_url = format!(
            "https://sheets.googleapis.com/v4/spreadsheets/{}:batchUpdate",
            spreadsheet_id
        );
        let body = serde_json::json!({
            "requests": [{
                "addSheet": {
                    "properties": {
                        "title": sheet_name
                    }
                }
            }]
        });

        let resp = self
            .client
            .post(&batch_url)
            .bearer_auth(access_token)
            .json(&body)
            .send()
            .await
            .map_err(|e| TargetError::DeliveryError(format!("Failed to create worksheet: {}", e)))?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(TargetError::DeliveryError(format!(
                "Failed to create worksheet '{}': {}",
                sheet_name, body
            )));
        }

        tracing::info!(spreadsheet_id = %spreadsheet_id, sheet = %sheet_name, "Created worksheet");
        Ok(true) // Newly created
    }

    /// Append a row to a worksheet in the spreadsheet.
    async fn append_row(
        &self,
        access_token: &str,
        spreadsheet_id: &str,
        sheet_name: &str,
        headers: &[String],
        values: &[serde_json::Value],
    ) -> Result<(), TargetError> {
        let range = format!("'{}'!A1", sheet_name);
        let url = format!(
            "https://sheets.googleapis.com/v4/spreadsheets/{}/values/{}:append",
            spreadsheet_id, range
        );

        let body = serde_json::json!({
            "values": [
                headers,
                values,
            ]
        });

        let resp = self
            .client
            .post(&url)
            .query(&[
                ("valueInputOption", "RAW"),
                ("insertDataOption", "INSERT_ROWS"),
            ])
            .bearer_auth(access_token)
            .json(&body)
            .send()
            .await
            .map_err(|e| TargetError::DeliveryError(format!("Sheets API append failed: {}", e)))?;

        let status = resp.status();
        if status == 429 {
            return Err(TargetError::DeliveryError(
                "Google Sheets rate limit exceeded (429). Will retry.".to_string(),
            ));
        }
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(TargetError::DeliveryError(format!(
                "Sheets API append HTTP {}: {}",
                status, body
            )));
        }

        Ok(())
    }

    /// Append a data-only row (no headers) to an existing worksheet.
    async fn append_data_row(
        &self,
        access_token: &str,
        spreadsheet_id: &str,
        sheet_name: &str,
        values: &[serde_json::Value],
    ) -> Result<(), TargetError> {
        let range = format!("'{}'!A1", sheet_name);
        let url = format!(
            "https://sheets.googleapis.com/v4/spreadsheets/{}/values/{}:append",
            spreadsheet_id, range
        );

        let body = serde_json::json!({
            "values": [values]
        });

        let resp = self
            .client
            .post(&url)
            .query(&[
                ("valueInputOption", "RAW"),
                ("insertDataOption", "INSERT_ROWS"),
            ])
            .bearer_auth(access_token)
            .json(&body)
            .send()
            .await
            .map_err(|e| TargetError::DeliveryError(format!("Sheets API append failed: {}", e)))?;

        let status = resp.status();
        if status == 429 {
            return Err(TargetError::DeliveryError(
                "Google Sheets rate limit exceeded (429). Will retry.".to_string(),
            ));
        }
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(TargetError::DeliveryError(format!(
                "Sheets API append HTTP {}: {}",
                status, body
            )));
        }

        Ok(())
    }
}

/// Flatten a JSON payload into dot-notation key-value pairs for spreadsheet columns.
///
/// - Nested objects use dot notation: `{"a": {"b": 1}}` → `("a.b", 1)`
/// - Arrays are skipped (not representable as scalar columns)
/// - Top-level "metadata" key is excluded (internal LocalPush field)
pub fn flatten_payload(payload: &serde_json::Value) -> Vec<(String, serde_json::Value)> {
    let mut result = Vec::new();
    if let Some(obj) = payload.as_object() {
        for (key, value) in obj {
            if key == "metadata" {
                continue;
            }
            flatten_recursive(key, value, &mut result);
        }
    }
    result
}

fn flatten_recursive(
    prefix: &str,
    value: &serde_json::Value,
    result: &mut Vec<(String, serde_json::Value)>,
) {
    match value {
        serde_json::Value::Object(map) => {
            for (key, val) in map {
                let new_prefix = format!("{}.{}", prefix, key);
                flatten_recursive(&new_prefix, val, result);
            }
        }
        serde_json::Value::Array(_) => {
            // Skip arrays — not representable as scalar columns
        }
        _ => {
            result.push((prefix.to_string(), value.clone()));
        }
    }
}

#[async_trait::async_trait]
impl Target for GoogleSheetsTarget {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.email
    }

    fn target_type(&self) -> &str {
        "google-sheets"
    }

    fn base_url(&self) -> &str {
        "https://sheets.google.com"
    }

    async fn test_connection(&self) -> Result<TargetInfo, TargetError> {
        let token = self.get_valid_token(&NullCredentialStore).await?;
        let spreadsheets = self.list_spreadsheets(&token).await?;

        Ok(TargetInfo {
            id: self.id.clone(),
            name: self.email.clone(),
            target_type: "google-sheets".to_string(),
            base_url: "https://sheets.google.com".to_string(),
            connected: true,
            details: serde_json::json!({ "spreadsheet_count": spreadsheets.len() }),
        })
    }

    async fn list_endpoints(&self) -> Result<Vec<TargetEndpoint>, TargetError> {
        let token = self.get_valid_token(&NullCredentialStore).await?;
        let spreadsheets = self.list_spreadsheets(&token).await?;

        Ok(spreadsheets
            .into_iter()
            .map(|f| TargetEndpoint {
                id: f.id.clone(),
                name: f.name,
                url: f.id, // endpoint URL = spreadsheet ID for Google Sheets
                authenticated: true,
                auth_type: Some("oauth2".to_string()),
                metadata: serde_json::json!({}),
            })
            .collect())
    }

    async fn deliver(
        &self,
        endpoint_id: &str,
        payload: &serde_json::Value,
        event_type: &str,
        credentials: &dyn CredentialStore,
    ) -> Result<bool, TargetError> {
        let token = self.get_valid_token(credentials).await?;

        // Use event_type (source ID) as the worksheet tab name
        let sheet_name = event_type;

        // Ensure worksheet exists (returns true if newly created)
        let is_new = self
            .ensure_worksheet(&token, endpoint_id, sheet_name)
            .await?;

        // Flatten payload to columns
        let pairs = flatten_payload(payload);
        if pairs.is_empty() {
            tracing::warn!(endpoint_id = %endpoint_id, "Empty payload after flattening, skipping");
            return Ok(true); // Nothing to write, but handled
        }

        let headers: Vec<String> = pairs.iter().map(|(k, _)| k.clone()).collect();
        let values: Vec<serde_json::Value> = pairs.into_iter().map(|(_, v)| v).collect();

        // Only write header row on first push to a new worksheet
        if is_new {
            self.append_row(&token, endpoint_id, sheet_name, &headers, &values)
                .await?;
        } else {
            self.append_data_row(&token, endpoint_id, sheet_name, &values)
                .await?;
        }

        tracing::info!(
            endpoint_id = %endpoint_id,
            sheet = %sheet_name,
            columns = headers.len(),
            "Row appended to Google Sheet"
        );

        Ok(true) // Handled natively — skip webhook POST
    }
}

/// No-op credential store for test_connection/list_endpoints (token refresh
/// during these operations won't persist, which is acceptable).
struct NullCredentialStore;

impl CredentialStore for NullCredentialStore {
    fn store(&self, _key: &str, _value: &str) -> Result<(), crate::traits::CredentialError> {
        Ok(())
    }
    fn retrieve(&self, _key: &str) -> Result<Option<String>, crate::traits::CredentialError> {
        Ok(None)
    }
    fn delete(&self, _key: &str) -> Result<bool, crate::traits::CredentialError> {
        Ok(false)
    }
    fn exists(&self, _key: &str) -> Result<bool, crate::traits::CredentialError> {
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flatten_flat_object() {
        let payload = serde_json::json!({
            "name": "test",
            "count": 42,
            "active": true
        });
        let pairs = flatten_payload(&payload);
        assert_eq!(pairs.len(), 3);
        assert!(pairs.iter().any(|(k, v)| k == "name" && v == "test"));
        assert!(pairs.iter().any(|(k, v)| k == "count" && v == &serde_json::json!(42)));
        assert!(pairs.iter().any(|(k, v)| k == "active" && v == &serde_json::json!(true)));
    }

    #[test]
    fn flatten_nested_object() {
        let payload = serde_json::json!({
            "summary": {
                "total_messages": 100,
                "total_sessions": 5
            }
        });
        let pairs = flatten_payload(&payload);
        assert_eq!(pairs.len(), 2);
        assert!(pairs.iter().any(|(k, v)| k == "summary.total_messages" && v == &serde_json::json!(100)));
        assert!(pairs.iter().any(|(k, v)| k == "summary.total_sessions" && v == &serde_json::json!(5)));
    }

    #[test]
    fn flatten_arrays_skipped() {
        let payload = serde_json::json!({
            "name": "test",
            "tags": ["a", "b", "c"],
            "nested": {
                "items": [1, 2, 3],
                "count": 3
            }
        });
        let pairs = flatten_payload(&payload);
        // "tags" and "nested.items" should be skipped
        assert_eq!(pairs.len(), 2);
        assert!(pairs.iter().any(|(k, _)| k == "name"));
        assert!(pairs.iter().any(|(k, _)| k == "nested.count"));
    }

    #[test]
    fn flatten_metadata_excluded() {
        let payload = serde_json::json!({
            "metadata": {
                "source": "claude-stats",
                "version": "1.0"
            },
            "data": "value"
        });
        let pairs = flatten_payload(&payload);
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].0, "data");
    }

    #[test]
    fn flatten_realistic_claude_stats_payload() {
        let payload = serde_json::json!({
            "metadata": {
                "source": "claude-stats",
                "schema_version": 2
            },
            "summary": {
                "total_sessions": 50,
                "total_messages": 500,
                "total_cost_usd": 25.50
            },
            "today": {
                "messages": 42,
                "sessions": 3,
                "cost_usd": 1.50
            },
            "daily_activity": [
                {"date": "2026-02-04", "messages": 42}
            ]
        });
        let pairs = flatten_payload(&payload);
        // metadata excluded, daily_activity (array) skipped
        // Expected: summary.total_sessions, summary.total_messages, summary.total_cost_usd,
        //           today.messages, today.sessions, today.cost_usd
        assert_eq!(pairs.len(), 6);
        assert!(pairs.iter().any(|(k, _)| k == "summary.total_cost_usd"));
        assert!(pairs.iter().any(|(k, _)| k == "today.messages"));
        assert!(!pairs.iter().any(|(k, _)| k.starts_with("metadata")));
        assert!(!pairs.iter().any(|(k, _)| k.starts_with("daily_activity")));
    }

    #[test]
    fn google_tokens_serialization_round_trip() {
        let tokens = GoogleTokens {
            access_token: "ya29.abc".to_string(),
            refresh_token: "1//0abc".to_string(),
            expires_at: 1700000000,
            client_id: "123.apps.googleusercontent.com".to_string(),
            client_secret: "GOCSPX-secret".to_string(),
        };
        let json = serde_json::to_string(&tokens).unwrap();
        let parsed: GoogleTokens = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.access_token, "ya29.abc");
        assert_eq!(parsed.refresh_token, "1//0abc");
        assert_eq!(parsed.expires_at, 1700000000);
        assert_eq!(parsed.client_id, "123.apps.googleusercontent.com");
        assert_eq!(parsed.client_secret, "GOCSPX-secret");
    }

    #[test]
    fn target_accessors() {
        let target = GoogleSheetsTarget::new(
            "gs-1".to_string(),
            "user@gmail.com".to_string(),
            GoogleTokens {
                access_token: "token".to_string(),
                refresh_token: "refresh".to_string(),
                expires_at: 0,
                client_id: "cid".to_string(),
                client_secret: "csecret".to_string(),
            },
        );
        assert_eq!(target.id(), "gs-1");
        assert_eq!(target.name(), "user@gmail.com");
        assert_eq!(target.target_type(), "google-sheets");
        assert_eq!(target.base_url(), "https://sheets.google.com");
    }
}
