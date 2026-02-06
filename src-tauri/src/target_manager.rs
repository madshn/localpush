//! Target Manager - Registry and orchestrator for push targets
//!
//! The TargetManager maintains the registry of available push targets (ntfy, etc.)
//! and provides connection testing and endpoint discovery.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::config::AppConfig;
use crate::traits::{Target, TargetEndpoint, TargetError, TargetInfo};

/// Error types for TargetManager operations
#[derive(Debug, thiserror::Error)]
pub enum TargetManagerError {
    #[error("Target not found: {0}")]
    NotFound(String),
    #[error("Target error: {0}")]
    TargetError(#[from] TargetError),
}

/// Registry and orchestrator for push targets
pub struct TargetManager {
    targets: Mutex<HashMap<String, Arc<dyn Target>>>,
    #[allow(dead_code)]
    config: Arc<AppConfig>,
}

impl TargetManager {
    /// Create a new TargetManager
    pub fn new(config: Arc<AppConfig>) -> Self {
        Self {
            targets: Mutex::new(HashMap::new()),
            config,
        }
    }

    /// Register a target in the registry
    pub fn register(&self, target: Arc<dyn Target>) {
        let id = target.id().to_string();
        self.targets.lock().unwrap().insert(id, target);
    }

    /// Get a target by ID
    pub fn get(&self, id: &str) -> Option<Arc<dyn Target>> {
        self.targets.lock().unwrap().get(id).cloned()
    }

    /// List all registered targets: (id, name, target_type)
    pub fn list(&self) -> Vec<(String, String, String)> {
        self.targets
            .lock()
            .unwrap()
            .iter()
            .map(|(id, t)| (id.clone(), t.name().to_string(), t.target_type().to_string()))
            .collect()
    }

    /// Test connectivity for a specific target
    pub async fn test_connection(&self, id: &str) -> Result<TargetInfo, TargetManagerError> {
        let target = self
            .get(id)
            .ok_or_else(|| TargetManagerError::NotFound(id.to_string()))?;
        Ok(target.test_connection().await?)
    }

    /// List endpoints for a specific target
    pub async fn list_endpoints(&self, id: &str) -> Result<Vec<TargetEndpoint>, TargetManagerError> {
        let target = self
            .get(id)
            .ok_or_else(|| TargetManagerError::NotFound(id.to_string()))?;
        Ok(target.list_endpoints().await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use crate::targets::NtfyTarget;

    #[test]
    fn test_register_and_list() {
        let config = Arc::new(AppConfig::open_in_memory().unwrap());
        let mgr = TargetManager::new(config);
        let target = Arc::new(NtfyTarget::new("ntfy-1".to_string(), "https://ntfy.sh".to_string()));
        mgr.register(target);

        let list = mgr.list();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].0, "ntfy-1");
    }

    #[test]
    fn test_get_nonexistent() {
        let config = Arc::new(AppConfig::open_in_memory().unwrap());
        let mgr = TargetManager::new(config);
        assert!(mgr.get("nope").is_none());
    }
}
