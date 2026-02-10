use crate::config::AppConfig;
use std::sync::Arc;

/// Property definition for a source.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PropertyDef {
    pub key: String,
    pub label: String,
    pub description: String,
    pub default_enabled: bool,
    pub privacy_sensitive: bool,
}

/// Current state of a property (definition + enabled status).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PropertyState {
    pub key: String,
    pub label: String,
    pub description: String,
    pub enabled: bool,
    pub privacy_sensitive: bool,
}

/// Store for per-source property configuration.
///
/// Uses the existing AppConfig with key pattern: `source_config.{source_id}.{property}`
pub struct SourceConfigStore {
    config: Arc<AppConfig>,
}

impl SourceConfigStore {
    pub fn new(config: Arc<AppConfig>) -> Self {
        Self { config }
    }

    /// Check if a property is enabled for a source.
    pub fn is_enabled(&self, source_id: &str, property: &str, default: bool) -> bool {
        let key = format!("source_config.{}.{}", source_id, property);
        self.config
            .get(&key)
            .ok()
            .flatten()
            .and_then(|v: String| v.parse::<bool>().ok())
            .unwrap_or(default)
    }

    /// Enable or disable a property for a source.
    pub fn set_enabled(&self, source_id: &str, property: &str, enabled: bool) -> Result<(), String> {
        let key = format!("source_config.{}.{}", source_id, property);
        self.config
            .set(&key, &enabled.to_string())
            .map_err(|e| format!("Failed to set property: {}", e))
    }

    /// Get all property states for a source, given default definitions.
    pub fn get_all(&self, source_id: &str, defaults: &[PropertyDef]) -> Vec<PropertyState> {
        defaults
            .iter()
            .map(|def| PropertyState {
                key: def.key.clone(),
                label: def.label.clone(),
                description: def.description.clone(),
                enabled: self.is_enabled(source_id, &def.key, def.default_enabled),
                privacy_sensitive: def.privacy_sensitive,
            })
            .collect()
    }

    /// Build a set of enabled property keys for a source.
    pub fn enabled_set(&self, source_id: &str, defaults: &[PropertyDef]) -> std::collections::HashSet<String> {
        defaults
            .iter()
            .filter(|def| self.is_enabled(source_id, &def.key, def.default_enabled))
            .map(|def| def.key.clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_enabled_returns_default_if_not_set() {
        let config = Arc::new(AppConfig::open_in_memory().unwrap());
        let store = SourceConfigStore::new(config);

        assert!(store.is_enabled("test-source", "prop1", true));
        assert!(!store.is_enabled("test-source", "prop2", false));
    }

    #[test]
    fn test_set_enabled_overrides_default() {
        let config = Arc::new(AppConfig::open_in_memory().unwrap());
        let store = SourceConfigStore::new(config);

        store.set_enabled("test-source", "prop1", false).unwrap();
        assert!(!store.is_enabled("test-source", "prop1", true));

        store.set_enabled("test-source", "prop1", true).unwrap();
        assert!(store.is_enabled("test-source", "prop1", false));
    }

    #[test]
    fn test_get_all_returns_property_states() {
        let config = Arc::new(AppConfig::open_in_memory().unwrap());
        let store = SourceConfigStore::new(config);

        let defaults = vec![
            PropertyDef {
                key: "prop1".to_string(),
                label: "Property 1".to_string(),
                description: "First property".to_string(),
                default_enabled: true,
                privacy_sensitive: false,
            },
            PropertyDef {
                key: "prop2".to_string(),
                label: "Property 2".to_string(),
                description: "Second property".to_string(),
                default_enabled: false,
                privacy_sensitive: true,
            },
        ];

        let states = store.get_all("test-source", &defaults);
        assert_eq!(states.len(), 2);
        assert_eq!(states[0].key, "prop1");
        assert!(states[0].enabled);
        assert_eq!(states[1].key, "prop2");
        assert!(!states[1].enabled);
        assert!(states[1].privacy_sensitive);
    }

    #[test]
    fn test_enabled_set_filters_disabled() {
        let config = Arc::new(AppConfig::open_in_memory().unwrap());
        let store = SourceConfigStore::new(config);

        let defaults = vec![
            PropertyDef {
                key: "enabled_prop".to_string(),
                label: "Enabled".to_string(),
                description: "".to_string(),
                default_enabled: true,
                privacy_sensitive: false,
            },
            PropertyDef {
                key: "disabled_prop".to_string(),
                label: "Disabled".to_string(),
                description: "".to_string(),
                default_enabled: false,
                privacy_sensitive: false,
            },
        ];

        let enabled = store.enabled_set("test-source", &defaults);
        assert!(enabled.contains("enabled_prop"));
        assert!(!enabled.contains("disabled_prop"));
    }
}
