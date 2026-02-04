//! Reqwest-based webhook client implementation

use std::time::{Duration, Instant};
use reqwest::Client;
use crate::traits::{WebhookClient, WebhookAuth, WebhookError, WebhookResponse};

const TIMEOUT_SECONDS: u64 = 25;

pub struct ReqwestWebhookClient {
    client: Client,
}

impl ReqwestWebhookClient {
    pub fn new() -> Result<Self, WebhookError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(TIMEOUT_SECONDS))
            .build()
            .map_err(|e| WebhookError::NetworkError(e.to_string()))?;

        tracing::debug!("Initialized webhook client with {}s timeout", TIMEOUT_SECONDS);
        Ok(Self { client })
    }

    fn apply_auth(&self, mut request: reqwest::RequestBuilder, auth: &WebhookAuth) -> reqwest::RequestBuilder {
        match auth {
            WebhookAuth::None => request,
            WebhookAuth::Header { name, value } => {
                tracing::debug!("Adding custom header: {}", name);
                request.header(name, value)
            }
            WebhookAuth::Bearer { token } => {
                tracing::debug!("Adding Bearer token");
                request.bearer_auth(token)
            }
            WebhookAuth::Basic { username, password } => {
                tracing::debug!("Adding Basic auth for user: {}", username);
                request.basic_auth(username, Some(password))
            }
        }
    }
}

#[async_trait::async_trait]
impl WebhookClient for ReqwestWebhookClient {
    async fn send(
        &self,
        url: &str,
        payload: &serde_json::Value,
        auth: &WebhookAuth,
    ) -> Result<WebhookResponse, WebhookError> {
        tracing::info!("Sending webhook to: {}", url);

        // Validate URL
        reqwest::Url::parse(url)
            .map_err(|e| WebhookError::InvalidUrl(e.to_string()))?;

        let start = Instant::now();

        // Build request
        let request = self.client
            .post(url)
            .header("Content-Type", "application/json")
            .json(payload);

        let request = self.apply_auth(request, auth);

        // Send request
        let response = request
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    tracing::warn!("Webhook timeout: {}", url);
                    WebhookError::Timeout
                } else if e.is_connect() || e.is_request() {
                    tracing::warn!("Network error: {}", e);
                    WebhookError::NetworkError(e.to_string())
                } else {
                    tracing::error!("Unexpected error: {}", e);
                    WebhookError::NetworkError(e.to_string())
                }
            })?;

        let duration_ms = start.elapsed().as_millis() as u64;
        let status = response.status().as_u16();

        // Read response body (best effort, don't fail if body read fails)
        let body = match response.text().await {
            Ok(text) if !text.is_empty() => Some(text),
            _ => None,
        };

        tracing::info!("Webhook response: status={}, duration={}ms", status, duration_ms);

        // Check for HTTP errors
        if !(200..300).contains(&status) {
            tracing::warn!("HTTP error response: {}", status);
            return Err(WebhookError::HttpError(status));
        }

        Ok(WebhookResponse {
            status,
            body,
            duration_ms,
        })
    }

    async fn test(&self, url: &str, auth: &WebhookAuth) -> Result<WebhookResponse, WebhookError> {
        tracing::info!("Testing webhook connectivity: {}", url);

        let test_payload = serde_json::json!({
            "test": true,
            "message": "LocalPush connectivity test",
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });

        self.send(url, &test_payload, auth).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = ReqwestWebhookClient::new();
        assert!(client.is_ok());
    }

    #[test]
    fn test_invalid_url() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let client = ReqwestWebhookClient::new().unwrap();
            let payload = serde_json::json!({});
            let result = client.send("not-a-url", &payload, &WebhookAuth::None).await;

            assert!(matches!(result, Err(WebhookError::InvalidUrl(_))));
        });
    }

    #[test]
    fn test_auth_application() {
        let client = ReqwestWebhookClient::new().unwrap();

        // Test that auth methods don't panic
        let request = client.client.post("https://example.com");
        let _ = client.apply_auth(request, &WebhookAuth::None);

        let request = client.client.post("https://example.com");
        let _ = client.apply_auth(request, &WebhookAuth::Header {
            name: "X-Api-Key".to_string(),
            value: "test".to_string(),
        });

        let request = client.client.post("https://example.com");
        let _ = client.apply_auth(request, &WebhookAuth::Bearer {
            token: "test-token".to_string(),
        });

        let request = client.client.post("https://example.com");
        let _ = client.apply_auth(request, &WebhookAuth::Basic {
            username: "user".to_string(),
            password: "pass".to_string(),
        });
    }
}
