//! Custom webhook target
//!
//! The "escape hatch" target — connect any REST endpoint with configurable auth.
//! Supports: None, Bearer token, Custom header, Basic auth.
//! URL validation: HTTPS required (HTTP allowed only for localhost).

use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::traits::{Target, TargetEndpoint, TargetError, TargetInfo};

/// Authentication type for custom webhook
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AuthType {
    None,
    Bearer { token: String },
    Header { name: String, value: String },
    Basic { username: String, password: String },
}

/// A push target backed by any REST endpoint with configurable auth
#[derive(Debug)]
pub struct CustomTarget {
    id: String,
    name: String,
    webhook_url: String,
    auth_type: AuthType,
    client: Client,
}

impl CustomTarget {
    /// Create a new Custom target with webhook URL and auth
    pub fn new(
        id: String,
        name: String,
        webhook_url: String,
        auth_type: AuthType,
    ) -> Result<Self, TargetError> {
        // Validate URL is HTTPS (allow HTTP for localhost only)
        if !webhook_url.starts_with("https://")
            && !webhook_url.starts_with("http://127.0.0.1")
            && !webhook_url.starts_with("http://localhost")
        {
            return Err(TargetError::InvalidConfig(
                "Webhook URL must use HTTPS (HTTP allowed only for localhost/127.0.0.1)"
                    .to_string(),
            ));
        }

        Ok(Self {
            id,
            name,
            webhook_url: webhook_url.trim_end_matches('/').to_string(),
            auth_type,
            client: Client::new(),
        })
    }

    /// Apply authentication to a request builder
    fn apply_auth(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        match &self.auth_type {
            AuthType::None => req,
            AuthType::Bearer { token } => req.bearer_auth(token),
            AuthType::Header { name, value } => req.header(name, value),
            AuthType::Basic { username, password } => req.basic_auth(username, Some(password)),
        }
    }
}

#[async_trait::async_trait]
impl Target for CustomTarget {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        "Custom"
    }

    fn target_type(&self) -> &str {
        "custom"
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

        let req = self.client.post(&self.webhook_url).json(&test_payload);
        let req = self.apply_auth(req);

        let resp = req
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
            name: "Custom".to_string(),
            target_type: "custom".to_string(),
            base_url: self.webhook_url.clone(),
            connected: true,
            details: serde_json::json!({ "name": self.name }),
        })
    }

    async fn list_endpoints(&self) -> Result<Vec<TargetEndpoint>, TargetError> {
        // Custom target has one endpoint (the webhook URL itself)
        let authenticated = !matches!(self.auth_type, AuthType::None);
        let auth_type_str = match &self.auth_type {
            AuthType::None => None,
            AuthType::Bearer { .. } => Some("bearer".to_string()),
            AuthType::Header { name, .. } => Some(format!("header:{}", name)),
            AuthType::Basic { .. } => Some("basic".to_string()),
        };

        Ok(vec![TargetEndpoint {
            id: format!("{}:default", self.id),
            name: self.name.clone(),
            url: self.webhook_url.clone(),
            authenticated,
            auth_type: auth_type_str,
            metadata: serde_json::json!({ "name": self.name }),
        }])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn https_url_accepted() {
        let result = CustomTarget::new(
            "custom-1".to_string(),
            "My API".to_string(),
            "https://api.example.com/webhook".to_string(),
            AuthType::None,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn localhost_http_accepted() {
        let result = CustomTarget::new(
            "custom-1".to_string(),
            "Local Dev".to_string(),
            "http://localhost:3000/webhook".to_string(),
            AuthType::None,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn localhost_ip_http_accepted() {
        let result = CustomTarget::new(
            "custom-1".to_string(),
            "Local Dev".to_string(),
            "http://127.0.0.1:8080/api/hook".to_string(),
            AuthType::None,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn http_non_localhost_rejected() {
        let result = CustomTarget::new(
            "custom-1".to_string(),
            "Insecure".to_string(),
            "http://api.example.com/webhook".to_string(),
            AuthType::None,
        );
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TargetError::InvalidConfig(_)));
    }

    #[test]
    fn trailing_slash_stripped() {
        let target = CustomTarget::new(
            "custom-1".to_string(),
            "Test".to_string(),
            "https://api.example.com/webhook/".to_string(),
            AuthType::None,
        )
        .unwrap();
        assert_eq!(target.base_url(), "https://api.example.com/webhook");
    }

    #[test]
    fn bearer_auth_applied() {
        let target = CustomTarget::new(
            "custom-1".to_string(),
            "Test".to_string(),
            "https://api.example.com/webhook".to_string(),
            AuthType::Bearer {
                token: "secret123".to_string(),
            },
        )
        .unwrap();

        let client = Client::new();
        let req = client.post("https://api.example.com/test");
        let _req = target.apply_auth(req);

        // Can't easily inspect headers in tests, but we verify the structure compiles
        assert_eq!(target.auth_type, AuthType::Bearer {
            token: "secret123".to_string()
        });
    }

    #[test]
    fn header_auth_structure() {
        let target = CustomTarget::new(
            "custom-1".to_string(),
            "Test".to_string(),
            "https://api.example.com/webhook".to_string(),
            AuthType::Header {
                name: "X-API-Key".to_string(),
                value: "key123".to_string(),
            },
        )
        .unwrap();

        assert_eq!(
            target.auth_type,
            AuthType::Header {
                name: "X-API-Key".to_string(),
                value: "key123".to_string()
            }
        );
    }

    #[test]
    fn basic_auth_structure() {
        let target = CustomTarget::new(
            "custom-1".to_string(),
            "Test".to_string(),
            "https://api.example.com/webhook".to_string(),
            AuthType::Basic {
                username: "user".to_string(),
                password: "pass".to_string(),
            },
        )
        .unwrap();

        assert_eq!(
            target.auth_type,
            AuthType::Basic {
                username: "user".to_string(),
                password: "pass".to_string()
            }
        );
    }

    #[test]
    fn list_endpoints_returns_single_endpoint() {
        let target = CustomTarget::new(
            "custom-1".to_string(),
            "My Webhook".to_string(),
            "https://api.example.com/webhook".to_string(),
            AuthType::Bearer {
                token: "secret".to_string(),
            },
        )
        .unwrap();

        let endpoints = futures::executor::block_on(target.list_endpoints()).unwrap();

        assert_eq!(endpoints.len(), 1);
        assert_eq!(endpoints[0].name, "My Webhook");
        assert_eq!(endpoints[0].url, "https://api.example.com/webhook");
        assert!(endpoints[0].authenticated);
        assert_eq!(endpoints[0].auth_type.as_deref(), Some("bearer"));
    }

    #[test]
    fn list_endpoints_none_auth_not_authenticated() {
        let target = CustomTarget::new(
            "custom-1".to_string(),
            "Public API".to_string(),
            "https://api.example.com/webhook".to_string(),
            AuthType::None,
        )
        .unwrap();

        let endpoints = futures::executor::block_on(target.list_endpoints()).unwrap();

        assert_eq!(endpoints.len(), 1);
        assert!(!endpoints[0].authenticated);
        assert_eq!(endpoints[0].auth_type, None);
    }
}
