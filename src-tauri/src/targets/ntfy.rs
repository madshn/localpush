//! ntfy push target
//!
//! Publishes notifications via the ntfy API.
//! Health check: GET `{server}/v1/health`
//! Publish: POST JSON to `{server}/{topic}`

use reqwest::Client;

use crate::traits::{Target, TargetEndpoint, TargetError, TargetInfo};

/// A push target backed by an ntfy server
pub struct NtfyTarget {
    id: String,
    server_url: String,
    default_topic: Option<String>,
    auth_token: Option<String>,
    client: Client,
}

impl NtfyTarget {
    /// Create a new ntfy target pointing at the given server
    pub fn new(id: String, server_url: String) -> Self {
        Self {
            id,
            server_url: server_url.trim_end_matches('/').to_string(),
            default_topic: None,
            auth_token: None,
            client: Client::new(),
        }
    }

    /// Set the default topic for this target
    pub fn with_topic(mut self, topic: String) -> Self {
        self.default_topic = Some(topic);
        self
    }

    /// Set the bearer auth token for authenticated publishing
    pub fn with_auth(mut self, token: String) -> Self {
        self.auth_token = Some(token);
        self
    }

    /// Publish a notification to a specific topic
    pub async fn publish(
        &self,
        topic: &str,
        title: &str,
        message: &str,
        tags: Option<Vec<String>>,
        priority: Option<u8>,
    ) -> Result<(), TargetError> {
        let url = format!("{}/{}", self.server_url, topic);

        let mut payload = serde_json::json!({
            "topic": topic,
            "title": title,
            "message": message,
        });
        if let Some(tags) = tags {
            payload["tags"] = serde_json::json!(tags);
        }
        if let Some(priority) = priority {
            payload["priority"] = serde_json::json!(priority);
        }

        let mut req = self.client.post(&url).json(&payload);
        if let Some(ref token) = self.auth_token {
            req = req.bearer_auth(token);
        }

        let response = req
            .send()
            .await
            .map_err(|e| TargetError::ConnectionFailed(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(TargetError::ConnectionFailed(format!(
                "HTTP {}: {}",
                status, body
            )));
        }

        Ok(())
    }
}

#[async_trait::async_trait]
impl Target for NtfyTarget {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        "ntfy"
    }

    fn target_type(&self) -> &str {
        "ntfy"
    }

    fn base_url(&self) -> &str {
        &self.server_url
    }

    async fn test_connection(&self) -> Result<TargetInfo, TargetError> {
        let url = format!("{}/v1/health", self.server_url);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| TargetError::ConnectionFailed(e.to_string()))?;

        let healthy = resp.status().is_success();

        Ok(TargetInfo {
            id: self.id.clone(),
            name: "ntfy".to_string(),
            target_type: "ntfy".to_string(),
            base_url: self.server_url.clone(),
            connected: healthy,
            details: serde_json::json!({ "healthy": healthy }),
        })
    }

    async fn list_endpoints(&self) -> Result<Vec<TargetEndpoint>, TargetError> {
        let mut endpoints = Vec::new();

        if let Some(ref topic) = self.default_topic {
            endpoints.push(TargetEndpoint {
                id: topic.clone(),
                name: format!("Topic: {}", topic),
                url: format!("{}/{}", self.server_url, topic),
                authenticated: self.auth_token.is_some(),
                auth_type: if self.auth_token.is_some() {
                    Some("bearer".to_string())
                } else {
                    None
                },
                metadata: serde_json::json!({}),
            });
        }

        Ok(endpoints)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ntfy_target_creation() {
        let target = NtfyTarget::new("ntfy-1".to_string(), "https://ntfy.sh".to_string());
        assert_eq!(target.id(), "ntfy-1");
        assert_eq!(target.target_type(), "ntfy");
        assert_eq!(target.base_url(), "https://ntfy.sh");
    }

    #[test]
    fn test_ntfy_with_topic() {
        let target = NtfyTarget::new("ntfy-1".to_string(), "https://ntfy.sh".to_string())
            .with_topic("localpush-alerts".to_string());
        assert_eq!(target.default_topic.as_deref(), Some("localpush-alerts"));
    }

    #[test]
    fn test_trailing_slash_stripped() {
        let target = NtfyTarget::new("t".to_string(), "https://ntfy.sh/".to_string());
        assert_eq!(target.base_url(), "https://ntfy.sh");
    }
}
