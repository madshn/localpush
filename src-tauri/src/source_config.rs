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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WindowSettingState {
    pub label: String,
    pub description: String,
    pub days: i64,
    pub default_days: i64,
    pub min_days: i64,
    pub max_days: i64,
    pub recommended_days: Vec<i64>,
}

#[derive(Debug, Clone, Copy)]
pub struct WindowSettingDef {
    pub label: &'static str,
    pub description: &'static str,
    pub default_days: i64,
    pub min_days: i64,
    pub max_days: i64,
}

pub fn window_setting_for_source(source_id: &str) -> Option<WindowSettingDef> {
    match source_id {
        "claude-sessions" => Some(WindowSettingDef {
            label: "Data Window",
            description:
                "How many recent days of Claude sessions to include in previews and payloads.",
            default_days: 7,
            min_days: 1,
            max_days: 30,
        }),
        "codex-sessions" => Some(WindowSettingDef {
            label: "Data Window",
            description:
                "How many recent days of Codex sessions to include in previews and payloads.",
            default_days: 7,
            min_days: 1,
            max_days: 30,
        }),
        "claude-stats" => Some(WindowSettingDef {
            label: "Data Window",
            description:
                "How many days of Claude activity to include in daily breakdowns and rollups.",
            default_days: 30,
            min_days: 1,
            max_days: 30,
        }),
        "codex-stats" => Some(WindowSettingDef {
            label: "Data Window",
            description: "How many complete UTC days of Codex token metrics to emit.",
            default_days: 1,
            min_days: 1,
            max_days: 30,
        }),
        _ => None,
    }
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
    pub fn set_enabled(
        &self,
        source_id: &str,
        property: &str,
        enabled: bool,
    ) -> Result<(), String> {
        let key = format!("source_config.{}.{}", source_id, property);
        self.config
            .set(&key, &enabled.to_string())
            .map_err(|e| format!("Failed to set property: {}", e))
    }

    pub fn get_window_days(&self, source_id: &str, def: &WindowSettingDef) -> i64 {
        let key = format!("source_window.{}.days", source_id);
        self.config
            .get(&key)
            .ok()
            .flatten()
            .and_then(|v| v.parse::<i64>().ok())
            .map(|days| days.clamp(def.min_days, def.max_days))
            .unwrap_or(def.default_days)
    }

    pub fn get_window_state(&self, source_id: &str) -> Option<WindowSettingState> {
        let def = window_setting_for_source(source_id)?;
        let mut recommended_days = vec![1, 7, 14, 30]
            .into_iter()
            .filter(|day| *day >= def.min_days && *day <= def.max_days)
            .collect::<Vec<_>>();
        let current_days = self.get_window_days(source_id, &def);
        if !recommended_days.contains(&current_days) {
            recommended_days.push(current_days);
            recommended_days.sort_unstable();
        }

        Some(WindowSettingState {
            label: def.label.to_string(),
            description: def.description.to_string(),
            days: current_days,
            default_days: def.default_days,
            min_days: def.min_days,
            max_days: def.max_days,
            recommended_days,
        })
    }

    pub fn set_window_days(&self, source_id: &str, days: i64) -> Result<(), String> {
        let Some(def) = window_setting_for_source(source_id) else {
            return Err(format!(
                "Source {} does not support window configuration",
                source_id
            ));
        };

        if days < def.min_days || days > def.max_days {
            return Err(format!(
                "Window for {} must be between {} and {} days",
                source_id, def.min_days, def.max_days
            ));
        }

        let key = format!("source_window.{}.days", source_id);
        self.config
            .set(&key, &days.to_string())
            .map_err(|e| format!("Failed to set data window: {}", e))
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
    pub fn enabled_set(
        &self,
        source_id: &str,
        defaults: &[PropertyDef],
    ) -> std::collections::HashSet<String> {
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

    #[test]
    fn test_get_window_state_returns_default_window() {
        let config = Arc::new(AppConfig::open_in_memory().unwrap());
        let store = SourceConfigStore::new(config);

        let state = store.get_window_state("claude-sessions").unwrap();
        assert_eq!(state.days, 7);
        assert_eq!(state.default_days, 7);
        assert_eq!(state.max_days, 30);
        assert!(state.recommended_days.contains(&30));
    }

    #[test]
    fn test_set_window_days_persists_override() {
        let config = Arc::new(AppConfig::open_in_memory().unwrap());
        let store = SourceConfigStore::new(config);

        store.set_window_days("claude-sessions", 30).unwrap();

        let state = store.get_window_state("claude-sessions").unwrap();
        assert_eq!(state.days, 30);
    }

    #[test]
    fn test_set_window_days_rejects_invalid_values() {
        let config = Arc::new(AppConfig::open_in_memory().unwrap());
        let store = SourceConfigStore::new(config);

        let err = store.set_window_days("claude-sessions", 31).unwrap_err();
        assert!(err.contains("between 1 and 30"));
    }
}
