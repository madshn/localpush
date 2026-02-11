//! Make.com push target
//!
//! Discovers webhook endpoints from a Make.com instance via the REST API.
//! Auth: `Authorization: Token {api_key}` header.
//! Endpoints: webhooks from gateway-webhook hooks assigned to team.

use reqwest::Client;
use serde::Deserialize;

use crate::traits::{Target, TargetEndpoint, TargetError, TargetInfo};

/// A push target backed by a Make.com instance
pub struct MakeTarget {
    id: String,
    zone_url: String,
    api_key: String,
    team_id: Option<String>,
    client: Client,
}

#[derive(Deserialize)]
struct TeamsResponse {
    teams: Vec<TeamInfo>,
}

#[derive(Deserialize)]
struct TeamInfo {
    id: String,
    #[allow(dead_code)]
    name: String,
}

#[derive(Deserialize)]
struct HooksResponse {
    hooks: Vec<Hook>,
}

#[derive(Deserialize)]
struct Hook {
    id: u64,
    name: String,
    enabled: bool,
    #[serde(rename = "scenarioId")]
    scenario_id: Option<u64>,
    #[serde(rename = "webhookUrl")]
    webhook_url: String,
}

impl MakeTarget {
    /// Create a new Make.com target with zone URL and API key
    pub fn new(id: String, zone_url: String, api_key: String) -> Self {
        Self {
            id,
            zone_url: zone_url.trim_end_matches('/').to_string(),
            api_key,
            team_id: None,
            client: Client::new(),
        }
    }

    fn api_url(&self, path: &str) -> String {
        format!("{}/api/v2{}", self.zone_url, path)
    }

    async fn fetch_team_id(&self) -> Result<String, TargetError> {
        let url = self.api_url("/teams");

        let resp = self
            .client
            .get(&url)
            .header("Authorization", format!("Token {}", self.api_key))
            .send()
            .await
            .map_err(|e| TargetError::ConnectionFailed(e.to_string()))?;

        if resp.status() == 401 || resp.status() == 403 {
            return Err(TargetError::AuthFailed("Invalid API token".to_string()));
        }
        if !resp.status().is_success() {
            return Err(TargetError::ConnectionFailed(format!(
                "HTTP {}",
                resp.status()
            )));
        }

        let body: TeamsResponse = resp
            .json()
            .await
            .map_err(|e| TargetError::ConnectionFailed(e.to_string()))?;

        body.teams
            .first()
            .map(|t| t.id.clone())
            .ok_or_else(|| TargetError::InvalidConfig("No teams found".to_string()))
    }

    async fn fetch_hooks(&self, team_id: &str) -> Result<Vec<Hook>, TargetError> {
        let url = format!(
            "{}?teamId={}&typeName=gateway-webhook&assigned=true",
            self.api_url("/hooks"),
            team_id
        );

        let resp = self
            .client
            .get(&url)
            .header("Authorization", format!("Token {}", self.api_key))
            .send()
            .await
            .map_err(|e| TargetError::ConnectionFailed(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(TargetError::ConnectionFailed(format!(
                "HTTP {}",
                resp.status()
            )));
        }

        let body: HooksResponse = resp
            .json()
            .await
            .map_err(|e| TargetError::ConnectionFailed(e.to_string()))?;

        Ok(body.hooks)
    }

    fn extract_endpoints(&self, hooks: Vec<Hook>) -> Vec<TargetEndpoint> {
        hooks
            .into_iter()
            .map(|hook| TargetEndpoint {
                id: format!("hook-{}", hook.id),
                name: hook.name.clone(),
                url: hook.webhook_url.clone(),
                authenticated: false, // URL is self-authenticating
                auth_type: None,
                metadata: serde_json::json!({
                    "hook_id": hook.id,
                    "enabled": hook.enabled,
                    "scenario_id": hook.scenario_id,
                }),
            })
            .collect()
    }
}

#[async_trait::async_trait]
impl Target for MakeTarget {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        "Make.com"
    }

    fn target_type(&self) -> &str {
        "make"
    }

    fn base_url(&self) -> &str {
        &self.zone_url
    }

    async fn test_connection(&self) -> Result<TargetInfo, TargetError> {
        let team_id = self.fetch_team_id().await?;

        Ok(TargetInfo {
            id: self.id.clone(),
            name: "Make.com".to_string(),
            target_type: "make".to_string(),
            base_url: self.zone_url.clone(),
            connected: true,
            details: serde_json::json!({ "team_id": team_id }),
        })
    }

    async fn list_endpoints(&self) -> Result<Vec<TargetEndpoint>, TargetError> {
        let team_id = if let Some(ref id) = self.team_id {
            id.clone()
        } else {
            self.fetch_team_id().await?
        };

        let hooks = self.fetch_hooks(&team_id).await?;
        tracing::info!(hook_count = hooks.len(), "Discovered Make.com webhooks");

        Ok(self.extract_endpoints(hooks))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_endpoints_maps_hooks_to_endpoints() {
        let target = MakeTarget::new(
            "make-1".to_string(),
            "https://eu1.make.com".to_string(),
            "fake-key".to_string(),
        );

        let hooks = vec![
            Hook {
                id: 12345,
                name: "Analytics Hook".to_string(),
                enabled: true,
                scenario_id: Some(999),
                webhook_url: "https://hook.eu1.make.com/xyz123".to_string(),
            },
            Hook {
                id: 67890,
                name: "Disabled Hook".to_string(),
                enabled: false,
                scenario_id: None,
                webhook_url: "https://hook.eu1.make.com/abc456".to_string(),
            },
        ];

        let endpoints = target.extract_endpoints(hooks);

        assert_eq!(endpoints.len(), 2);
        assert_eq!(endpoints[0].id, "hook-12345");
        assert_eq!(endpoints[0].name, "Analytics Hook");
        assert_eq!(endpoints[0].url, "https://hook.eu1.make.com/xyz123");
        assert!(!endpoints[0].authenticated);

        assert_eq!(endpoints[1].id, "hook-67890");
        assert!(!endpoints[1].metadata["enabled"].as_bool().unwrap());
    }

    #[test]
    fn trailing_slash_stripped_from_zone_url() {
        let target = MakeTarget::new(
            "t".to_string(),
            "https://eu1.make.com/".to_string(),
            "k".to_string(),
        );
        assert_eq!(target.base_url(), "https://eu1.make.com");
    }

    #[test]
    fn api_url_construction() {
        let target = MakeTarget::new(
            "t".to_string(),
            "https://eu1.make.com".to_string(),
            "k".to_string(),
        );
        assert_eq!(
            target.api_url("/teams"),
            "https://eu1.make.com/api/v2/teams"
        );
    }
}
