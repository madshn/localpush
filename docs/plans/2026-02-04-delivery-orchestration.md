# Delivery Orchestration Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Wire the five isolated components (FileWatcher, Sources, Ledger, WebhookClient, Commands) into a working delivery pipeline.

**Architecture:** Single background worker polls the ledger every 5 seconds. A SourceManager bridges file events to ledger enqueue. A Config table persists webhook URL and enabled sources across restarts. For MVP: one webhook target, one source (Claude Code Stats).

**Tech Stack:** Rust, Tauri 2.0, SQLite (rusqlite), tokio (async runtime), notify (FSEvents)

---

## Pre-Requisite: Fix Compilation Bugs

Before any new code, fix existing naming mismatches that prevent compilation.

### Bug 1: Mock names in state.rs

**File:** `src-tauri/src/state.rs:38`

Current (broken):
```rust
use crate::mocks::{MockCredentialStore, MockFileWatcher, MockWebhookClient, InMemoryLedger};
```

Fix:
```rust
use crate::mocks::{InMemoryCredentialStore, ManualFileWatcher, RecordedWebhookClient, InMemoryLedger};
```

And update lines 41-43:
```rust
credentials: Arc::new(InMemoryCredentialStore::new()),
file_watcher: Arc::new(ManualFileWatcher::new()),
webhook_client: Arc::new(RecordedWebhookClient::new()),
```

### Bug 2: Missing async_trait dependency

`mocks/mod.rs:8` uses `async_trait::async_trait` but `async_trait` isn't in Cargo.toml.

Fix: Add to `[dependencies]` in `src-tauri/Cargo.toml`:
```toml
async-trait = "0.1"
```

### Bug 3: InMemoryLedger won't work as mock

`state.rs:44` does `InMemoryLedger::new()` but `InMemoryLedger` is re-exported as `DeliveryLedger` which requires `open_in_memory()`, not `new()`.

Fix: Change to `InMemoryLedger::open_in_memory().unwrap()`

---

## Task 0: Fix Compilation Bugs

**Files:**
- Modify: `src-tauri/src/state.rs:36-46`
- Modify: `src-tauri/Cargo.toml`

**Step 1:** Fix state.rs mock imports and constructor calls as described above.

**Step 2:** Add `async-trait = "0.1"` to Cargo.toml dependencies.

**Step 3:** Commit.

```bash
git add src-tauri/src/state.rs src-tauri/Cargo.toml
git commit -m "fix: resolve mock naming mismatches and missing async-trait dep"
```

---

## Task 1: Config Table

App settings persisted in SQLite. Simple key/value store.

**Files:**
- Create: `src-tauri/src/config.rs`
- Modify: `src-tauri/src/lib.rs` (add `mod config;`)
- Modify: `src-tauri/src/ledger.rs` (create config table alongside ledger table)

### What Config Stores (MVP)

| Key | Example Value | Purpose |
|-----|---------------|---------|
| `webhook_url` | `https://flow.example.com/webhook/abc` | Where to send deliveries |
| `webhook_auth_json` | `{"type":"header","name":"X-Api-Key","value":"secret"}` | Auth config (serialized WebhookAuth) |
| `source.claude-stats.enabled` | `true` | Source enable/disable state |

### Step 1: Write the failing test

```rust
// In config.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_and_get() {
        let config = AppConfig::open_in_memory().unwrap();
        config.set("key", "value").unwrap();
        assert_eq!(config.get("key").unwrap(), Some("value".to_string()));
    }

    #[test]
    fn test_get_missing_key() {
        let config = AppConfig::open_in_memory().unwrap();
        assert_eq!(config.get("missing").unwrap(), None);
    }

    #[test]
    fn test_delete() {
        let config = AppConfig::open_in_memory().unwrap();
        config.set("key", "value").unwrap();
        config.delete("key").unwrap();
        assert_eq!(config.get("key").unwrap(), None);
    }

    #[test]
    fn test_set_overwrites() {
        let config = AppConfig::open_in_memory().unwrap();
        config.set("key", "v1").unwrap();
        config.set("key", "v2").unwrap();
        assert_eq!(config.get("key").unwrap(), Some("v2".to_string()));
    }

    #[test]
    fn test_get_bool() {
        let config = AppConfig::open_in_memory().unwrap();
        config.set("enabled", "true").unwrap();
        assert_eq!(config.get_bool("enabled").unwrap(), true);
        assert_eq!(config.get_bool("missing").unwrap(), false);
    }
}
```

### Step 2: Implement AppConfig

```rust
pub struct AppConfig {
    conn: rusqlite::Connection,
}

impl AppConfig {
    pub fn open(conn: &rusqlite::Connection) -> Result<(), LedgerError> {
        // Create config table in existing DB
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS app_config (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at INTEGER NOT NULL
            );"
        ).map_err(|e| LedgerError::DatabaseError(e.to_string()))
    }

    pub fn open_in_memory() -> Result<Self, LedgerError> { ... }
    pub fn get(&self, key: &str) -> Result<Option<String>, LedgerError> { ... }
    pub fn set(&self, key: &str, value: &str) -> Result<(), LedgerError> { ... }
    pub fn delete(&self, key: &str) -> Result<(), LedgerError> { ... }
    pub fn get_bool(&self, key: &str) -> Result<bool, LedgerError> { ... }
}
```

### Step 3: Run tests, commit

```bash
cargo test config
git add src-tauri/src/config.rs src-tauri/src/lib.rs
git commit -m "feat(config): add SQLite-backed app config table"
```

---

## Task 2: Delivery Worker

Background tokio task that claims entries from the ledger and sends via webhook.

**Files:**
- Create: `src-tauri/src/delivery_worker.rs`
- Modify: `src-tauri/src/lib.rs` (add `pub mod delivery_worker;`)

### Design

```
loop {
    sleep(5s)
    config = read webhook_url + webhook_auth from AppConfig
    if no webhook configured → skip
    entries = ledger.claim_batch(10)
    for entry in entries:
        result = webhook_client.send(url, entry.payload, auth)
        if success → ledger.mark_delivered(entry.event_id)
        if failure → ledger.mark_failed(entry.event_id, error)
}
```

### Step 1: Write the failing test

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::mocks::*;

    #[tokio::test]
    async fn test_worker_delivers_pending_entry() {
        let ledger = Arc::new(DeliveryLedger::open_in_memory().unwrap());
        let webhook = Arc::new(RecordedWebhookClient::success());

        // Enqueue an entry
        let event_id = ledger.enqueue("test.event", serde_json::json!({"hello": "world"})).unwrap();

        // Run one tick
        let config = WorkerConfig {
            webhook_url: "https://example.com/hook".to_string(),
            webhook_auth: WebhookAuth::None,
        };
        process_batch(&ledger, &webhook, &config, 10).await;

        // Verify delivered
        assert_eq!(webhook.call_count(), 1);
        let delivered = ledger.get_by_status(DeliveryStatus::Delivered).unwrap();
        assert_eq!(delivered.len(), 1);
    }

    #[tokio::test]
    async fn test_worker_marks_failed_on_error() {
        let ledger = Arc::new(DeliveryLedger::open_in_memory().unwrap());
        let webhook = Arc::new(RecordedWebhookClient::always_fail(
            WebhookError::NetworkError("refused".to_string())
        ));

        ledger.enqueue("test.event", serde_json::json!({})).unwrap();

        let config = WorkerConfig {
            webhook_url: "https://example.com/hook".to_string(),
            webhook_auth: WebhookAuth::None,
        };
        process_batch(&ledger, &webhook, &config, 10).await;

        let failed = ledger.get_by_status(DeliveryStatus::Failed).unwrap();
        assert_eq!(failed.len(), 1);
    }

    #[tokio::test]
    async fn test_worker_skips_when_no_config() {
        // No webhook_url configured → worker does nothing
        let ledger = Arc::new(DeliveryLedger::open_in_memory().unwrap());
        let webhook = Arc::new(RecordedWebhookClient::success());

        ledger.enqueue("test.event", serde_json::json!({})).unwrap();

        // No config passed → skip
        let result = try_process_batch(&ledger, &webhook, None, 10).await;
        assert_eq!(webhook.call_count(), 0);
    }
}
```

### Step 2: Implement delivery worker

Core function (testable, no tokio::spawn):

```rust
pub struct WorkerConfig {
    pub webhook_url: String,
    pub webhook_auth: WebhookAuth,
}

/// Process one batch of deliveries. Testable standalone function.
pub async fn process_batch(
    ledger: &dyn DeliveryLedgerTrait,
    webhook: &dyn WebhookClient,
    config: &WorkerConfig,
    batch_size: usize,
) {
    let entries = match ledger.claim_batch(batch_size) {
        Ok(entries) => entries,
        Err(e) => { tracing::error!("Failed to claim batch: {}", e); return; }
    };

    for entry in entries {
        let result = webhook.send(&config.webhook_url, &entry.payload, &config.webhook_auth).await;
        match result {
            Ok(_response) => {
                let _ = ledger.mark_delivered(&entry.event_id);
            }
            Err(e) => {
                let _ = ledger.mark_failed(&entry.event_id, &e.to_string());
            }
        }
    }
}

/// Spawn the background delivery loop. Returns a JoinHandle for shutdown.
pub fn spawn_worker(
    ledger: Arc<dyn DeliveryLedgerTrait>,
    webhook: Arc<dyn WebhookClient>,
    config_db: Arc<AppConfig>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(5));
        loop {
            interval.tick().await;
            // Read config each tick (allows runtime changes)
            if let Some(worker_config) = read_worker_config(&config_db) {
                process_batch(&*ledger, &*webhook, &worker_config, 10).await;
            }
        }
    })
}
```

### Step 3: Run tests, commit

```bash
cargo test delivery_worker
git add src-tauri/src/delivery_worker.rs src-tauri/src/lib.rs
git commit -m "feat(worker): add background delivery worker with batch processing"
```

---

## Task 3: Source Manager

Registry of sources. Bridges file events to ledger enqueue.

**Files:**
- Create: `src-tauri/src/source_manager.rs`
- Modify: `src-tauri/src/lib.rs` (add `pub mod source_manager;`)

### Design

```rust
pub struct SourceManager {
    sources: HashMap<String, Arc<dyn Source>>,      // registered sources
    enabled: HashSet<String>,                        // currently enabled
    path_to_source: HashMap<PathBuf, String>,        // watch_path → source_id lookup
    ledger: Arc<dyn DeliveryLedgerTrait>,
    file_watcher: Arc<dyn FileWatcher>,
    config: Arc<AppConfig>,
}
```

### Step 1: Write tests

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_register_source() {
        let mgr = SourceManager::new_test();
        mgr.register(Arc::new(ClaudeStatsSource::new_with_path("/tmp/fake.json")));
        assert!(mgr.get_source("claude-stats").is_some());
    }

    #[test]
    fn test_enable_source_starts_watching() {
        let mgr = SourceManager::new_test();
        mgr.register(Arc::new(ClaudeStatsSource::new_with_path("/tmp/fake.json")));
        mgr.enable("claude-stats").unwrap();
        assert!(mgr.is_enabled("claude-stats"));
        // File watcher should now be watching the path
        assert!(mgr.file_watcher.watched_paths().contains(&PathBuf::from("/tmp/fake.json")));
    }

    #[test]
    fn test_disable_source_stops_watching() {
        let mgr = SourceManager::new_test();
        mgr.register(Arc::new(ClaudeStatsSource::new_with_path("/tmp/fake.json")));
        mgr.enable("claude-stats").unwrap();
        mgr.disable("claude-stats").unwrap();
        assert!(!mgr.is_enabled("claude-stats"));
        assert!(mgr.file_watcher.watched_paths().is_empty());
    }

    #[test]
    fn test_handle_file_event_enqueues_to_ledger() {
        let mgr = SourceManager::new_test_with_temp_stats();
        mgr.enable("claude-stats").unwrap();
        mgr.handle_file_event(&PathBuf::from("/tmp/fake-stats.json")).unwrap();
        let stats = mgr.ledger.get_stats().unwrap();
        assert_eq!(stats.pending, 1);
    }
}
```

### Step 2: Implement SourceManager

Key methods:
- `register(source: Arc<dyn Source>)` - Add source to registry
- `enable(source_id: &str)` - Start watching, persist to config
- `disable(source_id: &str)` - Stop watching, persist to config
- `handle_file_event(path: &Path)` - Lookup source by path, call parse(), enqueue
- `restore_enabled()` - On startup, re-enable from config
- `get_source(id: &str)` - Lookup for commands
- `list_sources()` - List all registered with enabled state

### Step 3: Add `new_with_path` constructor to ClaudeStatsSource

For testing, allow custom stats path:
```rust
impl ClaudeStatsSource {
    pub fn new_with_path(path: impl Into<PathBuf>) -> Self {
        Self { stats_path: path.into() }
    }
}
```

### Step 4: Run tests, commit

```bash
cargo test source_manager
git add src-tauri/src/source_manager.rs src-tauri/src/sources/claude_stats.rs src-tauri/src/lib.rs
git commit -m "feat(sources): add SourceManager with enable/disable and file event handling"
```

---

## Task 4: Event Bridge (FileWatcher → SourceManager)

Connect FSEvents notifications to SourceManager.

**Files:**
- Modify: `src-tauri/src/production/file_watcher.rs`
- Modify: `src-tauri/src/traits/file_watcher.rs` (add `set_event_handler`)

### Design Decision

Add an optional event callback to the FileWatcher trait:

```rust
pub trait FileWatcher: Send + Sync {
    fn watch(&self, path: PathBuf) -> Result<(), FileWatcherError>;
    fn unwatch(&self, path: PathBuf) -> Result<(), FileWatcherError>;
    fn watched_paths(&self) -> Vec<PathBuf>;
    fn set_event_handler(&self, handler: Arc<dyn Fn(FileEvent) + Send + Sync>);
}
```

### Step 1: Add `set_event_handler` to trait

In `traits/file_watcher.rs`, add the method with a default no-op.

### Step 2: Implement in FsEventsWatcher

Store `Arc<dyn Fn(FileEvent) + Send + Sync>` behind a Mutex. In the event handler thread, call it instead of just logging.

### Step 3: Implement in ManualFileWatcher (mock)

Store handler, add a `simulate_event(path)` method for testing.

### Step 4: Run tests, commit

```bash
cargo test file_watcher
git add src-tauri/src/traits/file_watcher.rs src-tauri/src/production/file_watcher.rs src-tauri/src/mocks/mod.rs
git commit -m "feat(watcher): add event handler callback to FileWatcher trait"
```

---

## Task 5: Wire Commands

Make the Tauri commands call real implementations.

**Files:**
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/state.rs` (add SourceManager + AppConfig to AppState)

### Step 1: Expand AppState

```rust
pub struct AppState {
    pub credentials: Arc<dyn CredentialStore>,
    pub file_watcher: Arc<dyn FileWatcher>,
    pub webhook_client: Arc<dyn WebhookClient>,
    pub ledger: Arc<dyn DeliveryLedgerTrait>,
    pub source_manager: Arc<SourceManager>,  // NEW
    pub config: Arc<AppConfig>,               // NEW
}
```

### Step 2: Wire each command

| Command | Current | Wire To |
|---------|---------|---------|
| `enable_source` | logs only | `source_manager.enable(source_id)` |
| `disable_source` | logs only | `source_manager.disable(source_id)` |
| `get_source_preview` | hardcoded JSON | `source_manager.get_source(id)?.preview()` |
| `get_sources` | hardcoded list | `source_manager.list_sources()` |
| `add_webhook_target` | stores auth only | Store URL + auth in config |
| `test_webhook` | works | No change needed |
| `get_delivery_status` | works | No change needed |
| `get_delivery_queue` | works | No change needed |

### Step 3: Run tests, commit

```bash
cargo test commands
git add src-tauri/src/commands/mod.rs src-tauri/src/state.rs
git commit -m "feat(commands): wire Tauri commands to real implementations"
```

---

## Task 6: Startup Orchestration

Connect everything on app launch.

**Files:**
- Modify: `src-tauri/src/lib.rs` (`setup_app` function)

### Step 1: Update setup_app

```rust
pub fn setup_app(app: &App) -> Result<(), Box<dyn std::error::Error>> {
    // 1. Init logging
    // 2. Open database (ledger + config share same SQLite file)
    // 3. Create AppConfig from same connection
    // 4. Create SourceManager, register built-in sources
    // 5. Recover orphaned in-flight entries
    // 6. Restore previously enabled sources
    // 7. Set up file event handler (watcher → source_manager)
    // 8. Spawn delivery worker
    // 9. Build AppState, manage via Tauri
    // 10. Setup tray
}
```

### Step 2: Integration test

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_full_pipeline_mock() {
        // 1. Create all mock components
        // 2. Register source with known test data
        // 3. Enable source → verify watcher started
        // 4. Simulate file event → verify ledger has entry
        // 5. Run one worker tick → verify webhook called
        // 6. Verify ledger entry marked delivered
    }
}
```

### Step 3: Commit

```bash
git add src-tauri/src/lib.rs
git commit -m "feat(startup): orchestrate full delivery pipeline on app launch"
```

---

## Task 7: End-to-End Integration Test

Full pipeline test with all mocks.

**Files:**
- Create: `src-tauri/tests/integration_test.rs`

### Test Scenario

```
1. Write fake stats-cache.json to temp dir
2. Create SourceManager with ClaudeStatsSource pointing at temp file
3. Enable source → watcher starts
4. Simulate file event on that path
5. Verify: ledger now has 1 pending entry
6. Run one delivery worker tick
7. Verify: webhook client received 1 call with correct payload
8. Verify: ledger entry is now "delivered"
9. Simulate file change again
10. Run another tick
11. Verify: 2 total deliveries
```

### Step 1: Write test, step 2: Make it pass, step 3: Commit

```bash
cargo test --test integration_test
git add src-tauri/tests/integration_test.rs
git commit -m "test: add end-to-end delivery pipeline integration test"
```

---

## Dependency Graph

```
Task 0 (fix bugs)
  └─► Task 1 (config table)
       └─► Task 2 (delivery worker) ←── uses config for webhook URL
       └─► Task 3 (source manager) ←── uses config for enabled state
            └─► Task 4 (event bridge) ←── connects watcher to source manager
                 └─► Task 5 (wire commands) ←── uses source manager + config
                      └─► Task 6 (startup orchestration)
                           └─► Task 7 (integration test)
```

Tasks 2 and 3 can run in **parallel** after Task 1 completes.

---

## Parallel Dispatch Strategy

| Phase | Tasks | Agents | Model |
|-------|-------|--------|-------|
| **Phase 1** | Task 0 (fix bugs) | 1 agent | haiku |
| **Phase 2** | Task 1 (config) | 1 agent | sonnet |
| **Phase 3** | Task 2 (worker) + Task 3 (source mgr) | 2 agents parallel | sonnet |
| **Phase 4** | Task 4 (event bridge) | 1 agent | sonnet |
| **Phase 5** | Task 5 (wire commands) + Task 6 (startup) | 1 agent sequential | sonnet |
| **Phase 6** | Task 7 (integration test) | 1 agent | sonnet |

**Estimated agents:** 7 dispatches across 6 phases.

---

## Verification

After all tasks complete:

```bash
cargo test                     # All unit + integration tests
cargo clippy -- -D warnings    # No warnings
cargo build --release          # Clean build
```

Full pipeline smoke test:
1. Launch app (`npm run tauri dev`)
2. Add webhook target (n8n URL + Header Auth)
3. Enable Claude Code Stats source
4. Verify: preview shows real data
5. Wait for stats-cache.json to change
6. Verify: delivery appears in queue
7. Verify: n8n receives the webhook
