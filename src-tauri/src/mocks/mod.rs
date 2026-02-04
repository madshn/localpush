//! Test doubles for dependency injection
//!
//! Provides in-memory implementations of all external dependencies for isolated testing.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use async_trait::async_trait;
use serde_json::Value;

use crate::traits::{
    CredentialStore, CredentialError,
    FileWatcher, FileWatcherError, FileEvent, FileEventKind,
    WebhookClient, WebhookError, WebhookResponse, WebhookAuth,
};

// Re-export ledger's in-memory implementation
pub use crate::ledger::DeliveryLedger as InMemoryLedger;

// ============================================================================
// InMemoryCredentialStore
// ============================================================================

/// In-memory credential store for testing
///
/// Thread-safe storage backed by HashMap. No actual keychain interaction.
#[derive(Clone)]
pub struct InMemoryCredentialStore {
    store: Arc<Mutex<HashMap<String, String>>>,
}

impl InMemoryCredentialStore {
    pub fn new() -> Self {
        Self {
            store: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Create store with pre-populated entries
    pub fn with_entries(entries: Vec<(&str, &str)>) -> Self {
        let mut map = HashMap::new();
        for (key, value) in entries {
            map.insert(key.to_string(), value.to_string());
        }
        Self {
            store: Arc::new(Mutex::new(map)),
        }
    }

    /// Get all stored keys (for assertions)
    pub fn keys(&self) -> Vec<String> {
        self.store.lock().unwrap().keys().cloned().collect()
    }

    /// Clear all entries
    pub fn clear(&self) {
        self.store.lock().unwrap().clear();
    }
}

impl Default for InMemoryCredentialStore {
    fn default() -> Self {
        Self::new()
    }
}

impl CredentialStore for InMemoryCredentialStore {
    fn store(&self, key: &str, value: &str) -> Result<(), CredentialError> {
        self.store.lock().unwrap().insert(key.to_string(), value.to_string());
        Ok(())
    }

    fn retrieve(&self, key: &str) -> Result<Option<String>, CredentialError> {
        Ok(self.store.lock().unwrap().get(key).cloned())
    }

    fn delete(&self, key: &str) -> Result<bool, CredentialError> {
        Ok(self.store.lock().unwrap().remove(key).is_some())
    }

    fn exists(&self, key: &str) -> Result<bool, CredentialError> {
        Ok(self.store.lock().unwrap().contains_key(key))
    }
}

// ============================================================================
// ManualFileWatcher
// ============================================================================

/// Manual file watcher for testing
///
/// Does not actually watch the file system. Tests call methods directly to
/// simulate file events.
#[derive(Clone)]
pub struct ManualFileWatcher {
    watched: Arc<Mutex<Vec<PathBuf>>>,
    event_handler: Arc<Mutex<Option<Arc<dyn Fn(FileEvent) + Send + Sync>>>>,
}

impl ManualFileWatcher {
    pub fn new() -> Self {
        Self {
            watched: Arc::new(Mutex::new(Vec::new())),
            event_handler: Arc::new(Mutex::new(None)),
        }
    }

    /// Check if a path is currently watched
    pub fn is_watching(&self, path: &PathBuf) -> bool {
        self.watched.lock().unwrap().contains(path)
    }

    /// Clear all watched paths
    pub fn clear(&self) {
        self.watched.lock().unwrap().clear();
    }

    /// Simulate a file event (for testing)
    pub fn simulate_event(&self, path: PathBuf) {
        if let Some(handler) = self.event_handler.lock().unwrap().as_ref() {
            handler(FileEvent {
                path,
                kind: FileEventKind::Modified,
                timestamp: chrono::Utc::now(),
            });
        }
    }
}

impl Default for ManualFileWatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl FileWatcher for ManualFileWatcher {
    fn watch(&self, path: PathBuf) -> Result<(), FileWatcherError> {
        let mut watched = self.watched.lock().unwrap();
        if !watched.contains(&path) {
            watched.push(path);
        }
        Ok(())
    }

    fn unwatch(&self, path: PathBuf) -> Result<(), FileWatcherError> {
        let mut watched = self.watched.lock().unwrap();
        watched.retain(|p| p != &path);
        Ok(())
    }

    fn watched_paths(&self) -> Vec<PathBuf> {
        self.watched.lock().unwrap().clone()
    }

    fn set_event_handler(&self, handler: Arc<dyn Fn(FileEvent) + Send + Sync>) {
        *self.event_handler.lock().unwrap() = Some(handler);
    }
}

// ============================================================================
// RecordedWebhookClient
// ============================================================================

#[derive(Debug, Clone)]
pub struct WebhookRequest {
    pub url: String,
    pub payload: Value,
    pub auth: WebhookAuth,
}

/// Failure configuration for webhook client
#[derive(Debug, Clone)]
pub enum WebhookBehavior {
    /// Always succeed with given status code
    AlwaysSucceed(u16),
    /// Fail N times, then succeed
    FailThenSucceed { fail_count: usize, error: WebhookError },
    /// Always fail with given error
    AlwaysFail(WebhookError),
    /// Custom response based on request
    Custom(Arc<dyn Fn(&WebhookRequest) -> Result<WebhookResponse, WebhookError> + Send + Sync>),
}

/// Recorded webhook client for testing
///
/// Records all requests and provides configurable responses.
#[derive(Clone)]
pub struct RecordedWebhookClient {
    requests: Arc<Mutex<Vec<WebhookRequest>>>,
    behavior: Arc<Mutex<WebhookBehavior>>,
    call_count: Arc<Mutex<usize>>,
}

impl RecordedWebhookClient {
    pub fn new() -> Self {
        Self {
            requests: Arc::new(Mutex::new(Vec::new())),
            behavior: Arc::new(Mutex::new(WebhookBehavior::AlwaysSucceed(200))),
            call_count: Arc::new(Mutex::new(0)),
        }
    }

    /// Always succeed with 200 OK
    pub fn success() -> Self {
        Self::new()
    }

    /// Fail N times, then succeed
    pub fn fail_then_succeed(fail_count: usize, error: WebhookError) -> Self {
        let mut client = Self::new();
        client.set_behavior(WebhookBehavior::FailThenSucceed { fail_count, error });
        client
    }

    /// Always fail with given error
    pub fn always_fail(error: WebhookError) -> Self {
        let mut client = Self::new();
        client.set_behavior(WebhookBehavior::AlwaysFail(error));
        client
    }

    /// Set the behavior for subsequent calls
    pub fn set_behavior(&mut self, behavior: WebhookBehavior) {
        *self.behavior.lock().unwrap() = behavior;
    }

    /// Get all recorded requests
    pub fn requests(&self) -> Vec<WebhookRequest> {
        self.requests.lock().unwrap().clone()
    }

    /// Get number of calls made
    pub fn call_count(&self) -> usize {
        *self.call_count.lock().unwrap()
    }

    /// Clear recorded requests
    pub fn clear(&self) {
        self.requests.lock().unwrap().clear();
        *self.call_count.lock().unwrap() = 0;
    }

    /// Record a request and determine response
    fn record_and_respond(
        &self,
        url: &str,
        payload: &Value,
        auth: &WebhookAuth,
    ) -> Result<WebhookResponse, WebhookError> {
        // Record request
        let request = WebhookRequest {
            url: url.to_string(),
            payload: payload.clone(),
            auth: auth.clone(),
        };
        self.requests.lock().unwrap().push(request.clone());

        // Increment call count
        let mut count = self.call_count.lock().unwrap();
        *count += 1;
        let current_count = *count;
        drop(count);

        // Determine response based on behavior
        let behavior = self.behavior.lock().unwrap().clone();
        match behavior {
            WebhookBehavior::AlwaysSucceed(status) => {
                Ok(WebhookResponse {
                    status,
                    body: Some("OK".to_string()),
                    duration_ms: 10,
                })
            }
            WebhookBehavior::FailThenSucceed { fail_count, error } => {
                if current_count <= fail_count {
                    Err(error)
                } else {
                    Ok(WebhookResponse {
                        status: 200,
                        body: Some("OK".to_string()),
                        duration_ms: 10,
                    })
                }
            }
            WebhookBehavior::AlwaysFail(error) => Err(error),
            WebhookBehavior::Custom(func) => func(&request),
        }
    }
}

impl Default for RecordedWebhookClient {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl WebhookClient for RecordedWebhookClient {
    async fn send(
        &self,
        url: &str,
        payload: &Value,
        auth: &WebhookAuth,
    ) -> Result<WebhookResponse, WebhookError> {
        self.record_and_respond(url, payload, auth)
    }

    async fn test(
        &self,
        url: &str,
        auth: &WebhookAuth,
    ) -> Result<WebhookResponse, WebhookError> {
        self.record_and_respond(url, &Value::Null, auth)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_credential_store() {
        let store = InMemoryCredentialStore::new();

        // Store
        store.store("key1", "value1").unwrap();
        assert!(store.exists("key1").unwrap());

        // Retrieve
        let value = store.retrieve("key1").unwrap();
        assert_eq!(value, Some("value1".to_string()));

        // Delete
        assert!(store.delete("key1").unwrap());
        assert!(!store.exists("key1").unwrap());
    }

    #[test]
    fn test_credential_store_with_entries() {
        let store = InMemoryCredentialStore::with_entries(vec![
            ("key1", "value1"),
            ("key2", "value2"),
        ]);

        assert_eq!(store.keys().len(), 2);
        assert_eq!(store.retrieve("key1").unwrap(), Some("value1".to_string()));
        assert_eq!(store.retrieve("key2").unwrap(), Some("value2".to_string()));
    }

    #[test]
    fn test_file_watcher() {
        let watcher = ManualFileWatcher::new();
        let path1 = PathBuf::from("/test/path1");
        let path2 = PathBuf::from("/test/path2");

        // Watch
        watcher.watch(path1.clone()).unwrap();
        assert!(watcher.is_watching(&path1));
        assert!(!watcher.is_watching(&path2));

        let paths = watcher.watched_paths();
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0], path1);

        // Unwatch
        watcher.unwatch(path1.clone()).unwrap();
        assert!(!watcher.is_watching(&path1));
        assert_eq!(watcher.watched_paths().len(), 0);
    }

    #[test]
    fn test_file_watcher_event_handler() {
        let watcher = ManualFileWatcher::new();
        let path = PathBuf::from("/test/path");

        // Set up event handler to capture events
        let received_events = Arc::new(Mutex::new(Vec::new()));
        let events_clone = Arc::clone(&received_events);

        watcher.set_event_handler(Arc::new(move |event: FileEvent| {
            events_clone.lock().unwrap().push(event.path.clone());
        }));

        // Simulate event
        watcher.simulate_event(path.clone());

        // Verify event was received
        let events = received_events.lock().unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], path);
    }

    #[test]
    fn test_file_watcher_event_handler_multiple_events() {
        let watcher = ManualFileWatcher::new();
        let path1 = PathBuf::from("/test/path1");
        let path2 = PathBuf::from("/test/path2");

        let received_events = Arc::new(Mutex::new(Vec::new()));
        let events_clone = Arc::clone(&received_events);

        watcher.set_event_handler(Arc::new(move |event: FileEvent| {
            events_clone.lock().unwrap().push(event.path.clone());
        }));

        // Simulate multiple events
        watcher.simulate_event(path1.clone());
        watcher.simulate_event(path2.clone());

        // Verify all events were received
        let events = received_events.lock().unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0], path1);
        assert_eq!(events[1], path2);
    }

    #[test]
    fn test_file_watcher_no_handler() {
        let watcher = ManualFileWatcher::new();
        let path = PathBuf::from("/test/path");

        // Simulate event without handler (should not panic)
        watcher.simulate_event(path);
    }

    #[tokio::test]
    async fn test_webhook_client_success() {
        let client = RecordedWebhookClient::success();

        let response = client.send(
            "https://example.com/webhook",
            &serde_json::json!({"test": true}),
            &WebhookAuth::None,
        ).await.unwrap();

        assert_eq!(response.status, 200);
        assert_eq!(client.call_count(), 1);
        assert_eq!(client.requests().len(), 1);
    }

    #[tokio::test]
    async fn test_webhook_client_fail_then_succeed() {
        let client = RecordedWebhookClient::fail_then_succeed(
            2,
            WebhookError::NetworkError("Connection refused".to_string()),
        );

        // First two calls fail
        let result1 = client.send("https://example.com/webhook", &Value::Null, &WebhookAuth::None).await;
        assert!(result1.is_err());

        let result2 = client.send("https://example.com/webhook", &Value::Null, &WebhookAuth::None).await;
        assert!(result2.is_err());

        // Third call succeeds
        let result3 = client.send("https://example.com/webhook", &Value::Null, &WebhookAuth::None).await;
        assert!(result3.is_ok());

        assert_eq!(client.call_count(), 3);
    }

    #[tokio::test]
    async fn test_webhook_client_always_fail() {
        let client = RecordedWebhookClient::always_fail(
            WebhookError::Timeout,
        );

        let result = client.send("https://example.com/webhook", &Value::Null, &WebhookAuth::None).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            WebhookError::Timeout => {},
            _ => panic!("Expected timeout error"),
        }
    }
}
