//! Webhook client trait for HTTP delivery

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum WebhookError {
    #[error("Network error: {0}")]
    NetworkError(String),
    #[error("HTTP error: {0}")]
    HttpError(u16),
    #[error("Timeout")]
    Timeout,
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),
    #[error("Serialization error: {0}")]
    SerializationError(String),
}

impl WebhookError {
    /// Whether this error is retryable
    pub fn is_retryable(&self) -> bool {
        match self {
            WebhookError::NetworkError(_) => true,
            WebhookError::HttpError(code) => {
                // Retry server errors and rate limits, not client errors
                *code >= 500 || *code == 429
            }
            WebhookError::Timeout => true,
            WebhookError::InvalidUrl(_) => false,
            WebhookError::SerializationError(_) => false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookResponse {
    pub status: u16,
    pub body: Option<String>,
    pub duration_ms: u64,
}

/// Authentication configuration for webhooks
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WebhookAuth {
    #[serde(rename = "none")]
    None,
    #[serde(rename = "header")]
    Header { name: String, value: String },
    #[serde(rename = "bearer")]
    Bearer { token: String },
    #[serde(rename = "basic")]
    Basic { username: String, password: String },
}

/// Trait for webhook HTTP delivery
///
/// Production: reqwest HTTP client
/// Testing: Recorded responses
#[cfg_attr(test, mockall::automock)]
#[async_trait::async_trait]
pub trait WebhookClient: Send + Sync {
    /// Send a webhook payload
    async fn send(
        &self,
        url: &str,
        payload: &serde_json::Value,
        auth: &WebhookAuth,
    ) -> Result<WebhookResponse, WebhookError>;

    /// Test webhook connectivity
    async fn test(&self, url: &str, auth: &WebhookAuth) -> Result<WebhookResponse, WebhookError>;
}
