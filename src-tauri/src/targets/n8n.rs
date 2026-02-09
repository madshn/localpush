//! n8n push target
//!
//! Discovers webhook endpoints from an n8n instance via the REST API.
//! Auth: `X-N8N-API-KEY` header.
//! Endpoints: active workflows containing `n8n-nodes-base.webhook` nodes.

use reqwest::Client;
use serde::Deserialize;

use crate::traits::{Target, TargetEndpoint, TargetError, TargetInfo};

/// A push target backed by an n8n instance
pub struct N8nTarget {
    id: String,
    instance_url: String,
    api_key: String,
    client: Client,
}

#[derive(Deserialize)]
struct WorkflowListResponse {
    data: Vec<WorkflowSummary>,
    #[serde(rename = "nextCursor")]
    next_cursor: Option<String>,
}

#[derive(Deserialize)]
struct WorkflowSummary {
    id: String,
    #[allow(dead_code)]
    name: String,
    #[allow(dead_code)]
    active: bool,
}

#[derive(Deserialize)]
struct WorkflowFull {
    id: String,
    name: String,
    #[allow(dead_code)]
    active: bool,
    nodes: Vec<WorkflowNode>,
}

#[derive(Deserialize)]
struct WorkflowNode {
    name: String,
    #[serde(rename = "type")]
    node_type: String,
    #[serde(rename = "webhookId")]
    webhook_id: Option<String>,
    parameters: Option<serde_json::Value>,
    #[allow(dead_code)]
    credentials: Option<serde_json::Value>,
}

impl N8nTarget {
    /// Create a new n8n target with instance URL and API key
    pub fn new(id: String, instance_url: String, api_key: String) -> Self {
        Self {
            id,
            instance_url: instance_url.trim_end_matches('/').to_string(),
            api_key,
            client: Client::new(),
        }
    }

    fn api_url(&self, path: &str) -> String {
        format!("{}/api/v1{}", self.instance_url, path)
    }

    async fn fetch_workflows(&self) -> Result<Vec<WorkflowSummary>, TargetError> {
        let mut all = Vec::new();
        let mut cursor: Option<String> = None;

        loop {
            let mut url = format!("{}?active=true&limit=100", self.api_url("/workflows"));
            if let Some(ref c) = cursor {
                url.push_str(&format!("&cursor={}", c));
            }

            let resp = self
                .client
                .get(&url)
                .header("X-N8N-API-KEY", &self.api_key)
                .send()
                .await
                .map_err(|e| TargetError::ConnectionFailed(e.to_string()))?;

            if resp.status() == 401 || resp.status() == 403 {
                return Err(TargetError::AuthFailed("Invalid API key".to_string()));
            }
            if !resp.status().is_success() {
                return Err(TargetError::ConnectionFailed(format!(
                    "HTTP {}",
                    resp.status()
                )));
            }

            let body: WorkflowListResponse = resp
                .json()
                .await
                .map_err(|e| TargetError::ConnectionFailed(e.to_string()))?;

            all.extend(body.data);

            match body.next_cursor {
                Some(c) if !c.is_empty() => cursor = Some(c),
                _ => break,
            }
        }

        Ok(all)
    }

    async fn fetch_workflow_details(&self, id: &str) -> Result<WorkflowFull, TargetError> {
        let url = self.api_url(&format!("/workflows/{}", id));

        let resp = self
            .client
            .get(&url)
            .header("X-N8N-API-KEY", &self.api_key)
            .send()
            .await
            .map_err(|e| TargetError::ConnectionFailed(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(TargetError::ConnectionFailed(format!(
                "HTTP {}",
                resp.status()
            )));
        }

        resp.json()
            .await
            .map_err(|e| TargetError::ConnectionFailed(e.to_string()))
    }

    fn extract_webhook_endpoints(&self, wf: &WorkflowFull) -> Vec<TargetEndpoint> {
        wf.nodes
            .iter()
            .filter(|n| n.node_type == "n8n-nodes-base.webhook")
            .filter_map(|node| {
                let params = node.parameters.as_ref()?;
                let path = params.get("path")?.as_str()?;
                let auth = params
                    .get("authentication")
                    .and_then(|a| a.as_str())
                    .unwrap_or("none");
                let method = params
                    .get("httpMethod")
                    .and_then(|m| m.as_str())
                    .unwrap_or("POST");

                Some(TargetEndpoint {
                    id: format!("{}:{}", wf.id, node.name),
                    name: format!("{} > {}", wf.name, node.name),
                    url: format!("{}/webhook/{}", self.instance_url, path),
                    authenticated: auth != "none",
                    auth_type: Some(auth.to_string()),
                    metadata: serde_json::json!({
                        "workflow_id": wf.id,
                        "workflow_name": wf.name,
                        "node_name": node.name,
                        "http_method": method,
                        "webhook_id": node.webhook_id,
                    }),
                })
            })
            .collect()
    }
}

#[async_trait::async_trait]
impl Target for N8nTarget {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        "n8n"
    }

    fn target_type(&self) -> &str {
        "n8n"
    }

    fn base_url(&self) -> &str {
        &self.instance_url
    }

    async fn test_connection(&self) -> Result<TargetInfo, TargetError> {
        let workflows = self.fetch_workflows().await?;

        Ok(TargetInfo {
            id: self.id.clone(),
            name: "n8n".to_string(),
            target_type: "n8n".to_string(),
            base_url: self.instance_url.clone(),
            connected: true,
            details: serde_json::json!({ "active_workflows": workflows.len() }),
        })
    }

    async fn list_endpoints(&self) -> Result<Vec<TargetEndpoint>, TargetError> {
        let workflows = self.fetch_workflows().await?;
        tracing::debug!(count = workflows.len(), "Fetching workflow details in parallel");

        let futures: Vec<_> = workflows
            .iter()
            .map(|wf| self.fetch_workflow_details(&wf.id))
            .collect();
        let results = futures::future::join_all(futures).await;

        let mut endpoints = Vec::new();
        for result in results {
            match result {
                Ok(full) => endpoints.extend(self.extract_webhook_endpoints(&full)),
                Err(e) => tracing::warn!(error = %e, "Failed to fetch workflow details, skipping"),
            }
        }

        tracing::info!(endpoint_count = endpoints.len(), "Discovered webhook endpoints");
        Ok(endpoints)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_workflow() -> WorkflowFull {
        WorkflowFull {
            id: "abc123".to_string(),
            name: "Test Workflow".to_string(),
            active: true,
            nodes: vec![
                WorkflowNode {
                    name: "Analytics Webhook".to_string(),
                    node_type: "n8n-nodes-base.webhook".to_string(),
                    webhook_id: Some("a8e3-uuid".to_string()),
                    parameters: Some(serde_json::json!({
                        "path": "analytics",
                        "httpMethod": "POST",
                        "authentication": "none",
                    })),
                    credentials: None,
                },
                WorkflowNode {
                    name: "Process Data".to_string(),
                    node_type: "n8n-nodes-base.code".to_string(),
                    webhook_id: None,
                    parameters: None,
                    credentials: None,
                },
            ],
        }
    }

    #[test]
    fn extract_webhook_endpoints_returns_only_webhooks() {
        let target = N8nTarget::new(
            "n8n-1".to_string(),
            "https://flow.example.com".to_string(),
            "fake".to_string(),
        );
        let wf = mock_workflow();
        let endpoints = target.extract_webhook_endpoints(&wf);

        assert_eq!(endpoints.len(), 1);
        assert_eq!(
            endpoints[0].url,
            "https://flow.example.com/webhook/analytics"
        );
        assert!(!endpoints[0].authenticated);
        assert_eq!(endpoints[0].name, "Test Workflow > Analytics Webhook");
    }

    #[test]
    fn authenticated_webhook_detected() {
        let target = N8nTarget::new(
            "n8n-1".to_string(),
            "https://flow.example.com".to_string(),
            "fake".to_string(),
        );
        let mut wf = mock_workflow();
        wf.nodes[0].parameters = Some(serde_json::json!({
            "path": "secure",
            "httpMethod": "POST",
            "authentication": "headerAuth",
        }));

        let endpoints = target.extract_webhook_endpoints(&wf);

        assert_eq!(endpoints.len(), 1);
        assert!(endpoints[0].authenticated);
        assert_eq!(endpoints[0].auth_type.as_deref(), Some("headerAuth"));
    }

    #[test]
    fn non_webhook_nodes_ignored() {
        let target = N8nTarget::new(
            "n8n-1".to_string(),
            "https://flow.example.com".to_string(),
            "fake".to_string(),
        );
        let wf = WorkflowFull {
            id: "wf1".to_string(),
            name: "No Webhooks".to_string(),
            active: true,
            nodes: vec![WorkflowNode {
                name: "Code".to_string(),
                node_type: "n8n-nodes-base.code".to_string(),
                webhook_id: None,
                parameters: None,
                credentials: None,
            }],
        };

        assert!(target.extract_webhook_endpoints(&wf).is_empty());
    }

    #[test]
    fn trailing_slash_stripped_from_instance_url() {
        let target = N8nTarget::new(
            "t".to_string(),
            "https://flow.example.com/".to_string(),
            "k".to_string(),
        );
        assert_eq!(target.base_url(), "https://flow.example.com");
    }
}
