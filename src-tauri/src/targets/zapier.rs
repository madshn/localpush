//! Zapier push target
//!
//! Simple paste-URL target with no API discovery.
//! Auth: URL is self-authenticating (hooks.zapier.com).

use reqwest::Client;

use crate::traits::{Target, TargetEndpoint, TargetError, TargetInfo};

/// A push target backed by a Zapier webhook URL
#[derive(Debug)]
pub struct ZapierTarget {
    id: String,
    webhook_url: String,
    name: String,
    client: Client,
}

impl ZapierTarget {
    /// Create a new Zapier target with webhook URL
    pub fn new(id: String, name: String, webhook_url: String) -> Result<Self, TargetError> {
        // Validate URL domain
        if !webhook_url.starts_with("https://hooks.zapier.com/") {
            return Err(TargetError::InvalidConfig(
                "Webhook URL must start with https://hooks.zapier.com/".to_string(),
            ));
        }

        Ok(Self {
            id,
            webhook_url,
            name,
            client: Client::new(),
        })
    }
}

#[async_trait::async_trait]
impl Target for ZapierTarget {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        "Zapier"
    }

    fn target_type(&self) -> &str {
        "zapier"
    }

    fn base_url(&self) -> &str {
        &self.webhook_url
    }

    async fn test_connection(&self) -> Result<TargetInfo, TargetError> {
        // Test with a probe payload
        let test_payload = serde_json::json!({
            "test": true,
            "source": "localpush",
        });

        let resp = self
            .client
            .post(&self.webhook_url)
            .json(&test_payload)
            .send()
            .await
            .map_err(|e| TargetError::ConnectionFailed(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(TargetError::ConnectionFailed(format!(
                "HTTP {}",
                resp.status()
            )));
        }

        Ok(TargetInfo {
            id: self.id.clone(),
            name: "Zapier".to_string(),
            target_type: "zapier".to_string(),
            base_url: self.webhook_url.clone(),
            connected: true,
            details: serde_json::json!({ "name": self.name }),
        })
    }

    async fn list_endpoints(&self) -> Result<Vec<TargetEndpoint>, TargetError> {
        // Zapier only has one endpoint (the webhook URL itself)
        Ok(vec![TargetEndpoint {
            id: format!("{}:default", self.id),
            name: self.name.clone(),
            url: self.webhook_url.clone(),
            authenticated: false, // URL is self-authenticating
            auth_type: None,
            metadata: serde_json::json!({ "name": self.name }),
        }])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_url_accepted() {
        let result = ZapierTarget::new(
            "zap-1".to_string(),
            "Test Zap".to_string(),
            "https://hooks.zapier.com/hooks/catch/123/abc/".to_string(),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn invalid_domain_rejected() {
        let result = ZapierTarget::new(
            "zap-1".to_string(),
            "Test Zap".to_string(),
            "https://malicious.com/hooks/catch/123/abc/".to_string(),
        );
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TargetError::InvalidConfig(_)));
    }

    #[test]
    fn http_rejected() {
        let result = ZapierTarget::new(
            "zap-1".to_string(),
            "Test Zap".to_string(),
            "http://hooks.zapier.com/hooks/catch/123/abc/".to_string(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn list_endpoints_returns_single_endpoint() {
        let target = ZapierTarget::new(
            "zap-1".to_string(),
            "My Webhook".to_string(),
            "https://hooks.zapier.com/hooks/catch/123/abc/".to_string(),
        )
        .unwrap();

        let endpoints = futures::executor::block_on(target.list_endpoints()).unwrap();

        assert_eq!(endpoints.len(), 1);
        assert_eq!(endpoints[0].name, "My Webhook");
        assert_eq!(
            endpoints[0].url,
            "https://hooks.zapier.com/hooks/catch/123/abc/"
        );
        assert!(!endpoints[0].authenticated);
    }
}
