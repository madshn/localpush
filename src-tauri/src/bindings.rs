//! Source-to-target binding management
//!
//! A binding connects a source to a target endpoint. When a source fires,
//! the delivery system looks up bindings and sends the payload to each
//! bound endpoint.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::config::AppConfig;

/// A binding between a source and a target endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceBinding {
    pub source_id: String,
    pub target_id: String,
    pub endpoint_id: String,
    pub endpoint_url: String,
    pub endpoint_name: String,
    pub created_at: i64,
    pub active: bool,
    /// Serialized Vec<(String, String)> of non-secret headers (including auth header name without secret value)
    #[serde(default)]
    pub headers_json: Option<String>,
    /// Credential store key for the secret auth value, e.g. "binding:claude-stats:wf1-Webhook"
    #[serde(default)]
    pub auth_credential_key: Option<String>,
}

/// Manages source-to-target bindings, persisted in config SQLite
pub struct BindingStore {
    config: Arc<AppConfig>,
}

impl BindingStore {
    pub fn new(config: Arc<AppConfig>) -> Self {
        Self { config }
    }

    /// Save a binding. Key format: `binding.{source_id}.{endpoint_id}`
    pub fn save(&self, binding: &SourceBinding) -> Result<(), String> {
        let key = format!("binding.{}.{}", binding.source_id, binding.endpoint_id);
        let json = serde_json::to_string(binding).map_err(|e| e.to_string())?;
        self.config.set(&key, &json).map_err(|e| e.to_string())
    }

    /// Remove a binding
    pub fn remove(&self, source_id: &str, endpoint_id: &str) -> Result<(), String> {
        let key = format!("binding.{}.{}", source_id, endpoint_id);
        self.config.delete(&key).map_err(|e| e.to_string())
    }

    /// Get all active bindings for a source
    pub fn get_for_source(&self, source_id: &str) -> Vec<SourceBinding> {
        let prefix = format!("binding.{}.", source_id);
        self.config
            .get_by_prefix(&prefix)
            .unwrap_or_default()
            .into_iter()
            .filter_map(|(_key, value)| serde_json::from_str(&value).ok())
            .filter(|b: &SourceBinding| b.active)
            .collect()
    }

    /// Get all active bindings across all sources
    pub fn list_all(&self) -> Vec<SourceBinding> {
        self.config
            .get_by_prefix("binding.")
            .unwrap_or_default()
            .into_iter()
            .filter_map(|(_key, value)| serde_json::from_str(&value).ok())
            .filter(|b: &SourceBinding| b.active)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_binding(source_id: &str, endpoint_id: &str) -> SourceBinding {
        SourceBinding {
            source_id: source_id.to_string(),
            target_id: "t1".to_string(),
            endpoint_id: endpoint_id.to_string(),
            endpoint_url: "https://example.com/webhook".to_string(),
            endpoint_name: "Test Endpoint".to_string(),
            created_at: 1000,
            active: true,
            headers_json: None,
            auth_credential_key: None,
        }
    }

    #[test]
    fn test_save_and_retrieve_binding() {
        let config = Arc::new(AppConfig::open_in_memory().unwrap());
        let store = BindingStore::new(config);

        let binding = SourceBinding {
            source_id: "claude-stats".to_string(),
            target_id: "n8n-1".to_string(),
            endpoint_id: "wf1:Webhook".to_string(),
            endpoint_url: "https://flow.example.com/webhook/analytics".to_string(),
            endpoint_name: "Analytics Workflow".to_string(),
            created_at: 1000,
            active: true,
            headers_json: None,
            auth_credential_key: None,
        };

        store.save(&binding).unwrap();
        let bindings = store.get_for_source("claude-stats");
        assert_eq!(bindings.len(), 1);
        assert_eq!(
            bindings[0].endpoint_url,
            "https://flow.example.com/webhook/analytics"
        );
    }

    #[test]
    fn test_remove_binding() {
        let config = Arc::new(AppConfig::open_in_memory().unwrap());
        let store = BindingStore::new(config);

        let binding = test_binding("claude-stats", "wf1:Webhook");
        store.save(&binding).unwrap();
        store.remove("claude-stats", "wf1:Webhook").unwrap();

        let bindings = store.get_for_source("claude-stats");
        assert!(bindings.is_empty());
    }

    #[test]
    fn test_list_all_bindings() {
        let config = Arc::new(AppConfig::open_in_memory().unwrap());
        let store = BindingStore::new(config);

        store.save(&test_binding("claude-stats", "ep1")).unwrap();
        store
            .save(&test_binding("claude-sessions", "ep2"))
            .unwrap();

        let all = store.list_all();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_inactive_bindings_excluded() {
        let config = Arc::new(AppConfig::open_in_memory().unwrap());
        let store = BindingStore::new(config);

        let mut binding = test_binding("claude-stats", "ep1");
        binding.active = false;
        store.save(&binding).unwrap();

        store.save(&test_binding("claude-stats", "ep2")).unwrap();

        let bindings = store.get_for_source("claude-stats");
        assert_eq!(bindings.len(), 1);
        assert_eq!(bindings[0].endpoint_id, "ep2");
    }

    #[test]
    fn test_binding_with_headers_json_round_trips() {
        let config = Arc::new(AppConfig::open_in_memory().unwrap());
        let store = BindingStore::new(config);

        let headers: Vec<(String, String)> = vec![
            ("Authorization".to_string(), String::new()),
            ("X-Custom".to_string(), "value".to_string()),
        ];
        let mut binding = test_binding("claude-stats", "ep1");
        binding.headers_json = Some(serde_json::to_string(&headers).unwrap());
        binding.auth_credential_key = Some("binding:claude-stats:ep1".to_string());

        store.save(&binding).unwrap();
        let loaded = store.get_for_source("claude-stats");
        assert_eq!(loaded.len(), 1);
        assert!(loaded[0].headers_json.is_some());
        assert_eq!(loaded[0].auth_credential_key.as_deref(), Some("binding:claude-stats:ep1"));

        // Verify headers deserialize correctly
        let parsed: Vec<(String, String)> = serde_json::from_str(loaded[0].headers_json.as_ref().unwrap()).unwrap();
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].0, "Authorization");
        assert_eq!(parsed[1].1, "value");
    }

    #[test]
    fn test_binding_without_new_fields_deserializes() {
        // Simulate a v0.1-era binding JSON without headers_json/auth_credential_key
        let json = r#"{
            "source_id": "claude-stats",
            "target_id": "t1",
            "endpoint_id": "ep1",
            "endpoint_url": "https://example.com/webhook",
            "endpoint_name": "Test",
            "created_at": 1000,
            "active": true
        }"#;
        let binding: SourceBinding = serde_json::from_str(json).unwrap();
        assert!(binding.headers_json.is_none());
        assert!(binding.auth_credential_key.is_none());
    }
}
