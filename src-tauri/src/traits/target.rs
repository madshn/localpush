//! Target trait for push notification delivery endpoints

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors that can occur when interacting with a target
#[derive(Debug, Clone, Error)]
pub enum TargetError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    #[error("Authentication failed: {0}")]
    AuthFailed(String),
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
    #[error("Not connected")]
    NotConnected,
}

/// Metadata about a registered target and its connection state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetInfo {
    pub id: String,
    pub name: String,
    pub target_type: String,
    pub base_url: String,
    pub connected: bool,
    pub details: serde_json::Value,
}

/// A single addressable endpoint within a target (e.g., an ntfy topic)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetEndpoint {
    pub id: String,
    pub name: String,
    pub url: String,
    pub authenticated: bool,
    pub auth_type: Option<String>,
    pub metadata: serde_json::Value,
}

/// Trait that all push targets must implement
///
/// Production: ntfy, webhooks, etc.
/// Testing: Mock targets with recorded responses
#[async_trait::async_trait]
pub trait Target: Send + Sync {
    /// Unique identifier for this target instance
    fn id(&self) -> &str;

    /// Human-readable name
    fn name(&self) -> &str;

    /// Target type identifier (e.g., "ntfy")
    fn target_type(&self) -> &str;

    /// Base URL for this target
    fn base_url(&self) -> &str;

    /// Test connectivity and return target info
    async fn test_connection(&self) -> Result<TargetInfo, TargetError>;

    /// List available endpoints (e.g., ntfy topics)
    async fn list_endpoints(&self) -> Result<Vec<TargetEndpoint>, TargetError>;
}
