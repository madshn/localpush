# LocalPush Backend

Rust backend using Tauri 2.0 with trait-based dependency injection for guaranteed file→webhook delivery.

**Role:** Watch files, store delivery state in SQLite (WAL mode), handle webhook retries with exponential backoff, inject testable dependencies via traits.

---

## Architecture Principles

### 1. Trait-Based DI

**All external dependencies are behind traits.** This enables:
- Production implementations (Keychain, FSEvents, HTTP)
- Mock implementations (in-memory for tests)
- 100% testable without external services

```rust
// traits/webhook_client.rs
pub trait WebhookClient: Send + Sync {
    async fn deliver(&self, req: WebhookRequest) -> Result<WebhookResponse, WebhookError>;
}

// production/webhook_client.rs
pub struct ReqwestWebhookClient { /* ... */ }
impl WebhookClient for ReqwestWebhookClient { /* ... */ }

// mocks/mod.rs
pub struct MockWebhookClient { /* ... */ }
impl WebhookClient for MockWebhookClient { /* ... */ }
```

### 2. SQLite WAL for Guaranteed Delivery

Delivery state lives in SQLite with WAL (Write-Ahead Logging):
- Survives crashes
- Atomic writes prevent corruption
- Background retry loop re-attempts failed deliveries

### 3. Async Tokio Runtime

All I/O is async:
```rust
pub async fn deliver(&self, req: WebhookRequest) -> Result<WebhookResponse, WebhookError> {
    // Non-blocking HTTP call
    self.client.post(&self.url).json(&req).send().await?
}
```

---

## Project Layout

```
src-tauri/src/
├── main.rs                   # Tauri entry + window setup
├── lib.rs                    # Library exports, setup_app()
├── state.rs                  # AppState (DI container)
├── ledger.rs                 # SQLite delivery ledger
├── traits/
│   ├── mod.rs               # Export all traits
│   ├── credential_store.rs  # Store/retrieve secrets
│   ├── file_watcher.rs      # Watch file changes
│   ├── webhook_client.rs    # Send HTTP requests
│   └── delivery_ledger.rs   # Delivery state CRUD
├── production/
│   ├── mod.rs               # Export all impls
│   ├── credential_store.rs  # macOS Keychain impl
│   ├── file_watcher.rs      # FSEvents (notify-rs) impl
│   └── webhook_client.rs    # Reqwest HTTP impl
├── mocks/
│   └── mod.rs               # All mock impls for testing
├── sources/
│   ├── mod.rs               # Source trait + registry
│   └── claude_stats.rs      # Parse ~/.claude/stats-cache.json
└── commands/
    └── mod.rs               # Tauri command handlers
```

---

## Key Traits & Implementations

### CredentialStore

```rust
pub trait CredentialStore: Send + Sync {
    async fn set(&self, key: &str, value: &str) -> Result<(), CredentialError>;
    async fn get(&self, key: &str) -> Result<Option<String>, CredentialError>;
    async fn delete(&self, key: &str) -> Result<(), CredentialError>;
}
```

**Production:** `KeyringCredentialStore` (macOS Keychain)
**Mock:** `InMemoryCredentialStore` (HashMap)

### FileWatcher

```rust
pub trait FileWatcher: Send + Sync {
    async fn watch(&self, path: &Path) -> Result<(), FileWatcherError>;
    async fn unwatch(&self, path: &Path) -> Result<(), FileWatcherError>;
}
```

**Production:** `NotifyFileWatcher` (notify-rs with 300ms debounce)
**Mock:** `ManualFileWatcher` (events you manually inject in tests)

### WebhookClient

```rust
pub trait WebhookClient: Send + Sync {
    async fn deliver(&self, req: WebhookRequest) -> Result<WebhookResponse, WebhookError>;
}
```

**Production:** `ReqwestWebhookClient` (HTTP, 25s timeout)
**Mock:** `RecordingWebhookClient` (saves requests, lets you control response)

### DeliveryLedger

```rust
pub trait DeliveryLedgerTrait: Send + Sync {
    async fn enqueue(&self, entry: DeliveryEntry) -> Result<String, DeliveryError>;
    async fn get_pending(&self) -> Result<Vec<DeliveryEntry>, DeliveryError>;
    async fn mark_delivered(&self, id: &str) -> Result<(), DeliveryError>;
    async fn mark_failed(&self, id: &str, retries: u32) -> Result<(), DeliveryError>;
}
```

**Production:** `SqliteLedger` (WAL mode, persistent)
**Mock:** `InMemoryLedger` (HashMap)

---

## AppState (Dependency Injection Container)

`state.rs` holds all dependencies:

```rust
pub struct AppState {
    pub credential_store: Arc<dyn CredentialStore>,
    pub file_watcher: Arc<dyn FileWatcher>,
    pub webhook_client: Arc<dyn WebhookClient>,
    pub delivery_ledger: Arc<dyn DeliveryLedger>,
}

impl AppState {
    pub fn new_production(app_handle: &AppHandle) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            credential_store: Arc::new(KeyringCredentialStore::new()?),
            file_watcher: Arc::new(NotifyFileWatcher::new()?),
            webhook_client: Arc::new(ReqwestWebhookClient::new()?),
            delivery_ledger: Arc::new(SqliteLedger::new(app_handle)?),
        })
    }

    pub fn new_mock() -> Self {
        Self {
            credential_store: Arc::new(InMemoryCredentialStore::new()),
            file_watcher: Arc::new(ManualFileWatcher::new()),
            webhook_client: Arc::new(RecordingWebhookClient::new()),
            delivery_ledger: Arc::new(InMemoryLedger::new()),
        }
    }
}
```

**In Tauri commands:** Extract `AppState` from `State<Arc<AppState>>`.

---

## Tauri Commands

All commands in `commands/mod.rs`:

```rust
#[tauri::command]
pub async fn get_delivery_status(
    state: State<'_, Arc<AppState>>,
) -> Result<DeliveryStatusResponse, String> {
    let pending = state.delivery_ledger
        .get_pending()
        .await
        .map_err(|e| format!("Ledger error: {}", e))?;

    Ok(DeliveryStatusResponse {
        pending_count: pending.len(),
        // ...
    })
}
```

**Pattern:**
1. Extract `State<Arc<AppState>>`
2. Call trait methods (abstracted behind deps)
3. Return `Result<T, String>` (Tauri serializes this to frontend)

---

## Testing Strategy

### Unit Tests

Write in trait or implementation files using `#[cfg(test)]`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_webhook_success() {
        let client = RecordingWebhookClient::new();
        let req = WebhookRequest { /* ... */ };

        let result = client.deliver(req).await;

        assert!(result.is_ok());
        assert_eq!(client.recorded_requests.lock().unwrap().len(), 1);
    }
}
```

### Integration Tests

Create mock `AppState` and test command behavior:

```rust
#[tokio::test]
async fn test_get_delivery_status() {
    let state = Arc::new(AppState::new_mock());

    // Inject test data
    state.delivery_ledger.enqueue(/* ... */).await.unwrap();

    // Test command
    let result = get_delivery_status(State::new(state)).await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap().pending_count, 1);
}
```

### Run All Tests

```bash
cargo test                    # All tests
cargo test --lib             # Library tests only
cargo test webhook            # Tests matching "webhook"
cargo test -- --nocapture    # Show println! output
```

---

## Error Handling

Use `thiserror` for ergonomic error types:

```rust
#[derive(Debug, thiserror::Error)]
pub enum WebhookError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Timeout")]
    Timeout,

    #[error("Invalid auth")]
    InvalidAuth,
}
```

Commands convert errors to `String`:

```rust
pub async fn my_command(...) -> Result<T, String> {
    let result = something().await
        .map_err(|e| format!("Error: {}", e))?;
    Ok(result)
}
```

---

## Logging

Uses `tracing` crate:

```rust
use tracing::{info, warn, error, debug};

tracing::info!("Webhook delivered: {}", delivery_id);
tracing::warn!("Retry #{}", retries);
tracing::error!("Failed: {}", err);
```

**Enable with environment:**
```bash
RUST_LOG=localpush=debug cargo test
RUST_LOG=localpush::ledger=trace npm run tauri dev
```

---

## Async Patterns

### Spawning Background Tasks

Don't block in commands. Use `tokio::spawn`:

```rust
#[tauri::command]
pub async fn trigger_delivery(
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let state_clone = state.inner().clone();

    tokio::spawn(async move {
        // Background work
        state_clone.webhook_client.deliver(req).await.ok();
    });

    Ok(())
}
```

### Waiting for Async Results

Use channels if you need to wait:

```rust
let (tx, mut rx) = tokio::sync::mpsc::channel(1);

tokio::spawn(async move {
    let result = state.webhook_client.deliver(req).await;
    tx.send(result).await.ok();
});

let result = rx.recv().await;
```

---

## Adding New Dependencies

### Add a Trait (traits/my_trait.rs)

```rust
pub trait MyTrait: Send + Sync {
    async fn do_something(&self) -> Result<T, MyError>;
}
```

### Add Production Impl (production/my_impl.rs)

```rust
pub struct MyImpl { /* ... */ }

impl MyTrait for MyImpl {
    async fn do_something(&self) -> Result<T, MyError> {
        // Real implementation
    }
}
```

### Add Mock (mocks/mod.rs)

```rust
pub struct MockMyImpl { /* ... */ }

impl MyTrait for MockMyImpl {
    async fn do_something(&self) -> Result<T, MyError> {
        Ok(/* test data */)
    }
}
```

### Add to AppState (state.rs)

```rust
pub struct AppState {
    // ...
    pub my_trait: Arc<dyn MyTrait>,
}

impl AppState {
    pub fn new_production(...) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            // ...
            my_trait: Arc::new(MyImpl::new()?),
        })
    }

    pub fn new_mock() -> Self {
        Self {
            // ...
            my_trait: Arc::new(MockMyImpl::new()),
        }
    }
}
```

---

## SQLite Ledger Details

### Schema

```sql
CREATE TABLE delivery_queue (
    id TEXT PRIMARY KEY,
    file_path TEXT NOT NULL,
    webhook_url TEXT NOT NULL,
    payload BLOB NOT NULL,
    status TEXT NOT NULL,  -- 'pending' | 'delivered' | 'failed'
    retries INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE delivery_history (
    id TEXT PRIMARY KEY,
    delivery_id TEXT,
    status_code INTEGER,
    error_message TEXT,
    completed_at TEXT NOT NULL
);
```

### WAL Mode

Enabled automatically in `ledger.rs`:

```rust
conn.execute("PRAGMA journal_mode = WAL", [])?;
conn.execute("PRAGMA synchronous = NORMAL", [])?;
```

Guarantees crash-safe writes.

---

## Key Files Reference

| File | Responsibility |
|------|-----------------|
| `main.rs` | Tauri entry point, window setup |
| `lib.rs` | Public API, `setup_app()` |
| `state.rs` | DI container, trait wiring |
| `ledger.rs` | SQLite operations + tests |
| `commands/mod.rs` | Tauri command handlers |
| `traits/*.rs` | Trait definitions (contracts) |
| `production/*.rs` | Real implementations |
| `mocks/*.rs` | Test doubles |
| `sources/*.rs` | File source parsers |

---

## Common Tasks

### Add a Tauri Command

1. Define in `commands/mod.rs` with `#[tauri::command]`
2. Extract `State<Arc<AppState>>`
3. Call trait methods on state
4. Return `Result<T, String>`
5. Register in `main.rs` with `invoke_handler`

### Test a Command

```rust
#[tokio::test]
async fn test_my_command() {
    let state = Arc::new(AppState::new_mock());
    let result = my_command(State::new(state)).await;
    assert!(result.is_ok());
}
```

### Add a Source

1. Create `sources/my_source.rs` implementing `Source` trait
2. Add to `sources/mod.rs`
3. Wire into `AppState` + commands
4. Test with mock implementations

---

## Debugging

### Print Logs

```rust
eprintln!("Debug: {:?}", value);  // stderr
tracing::debug!("Debug: {:?}", value);  // to log file
```

### SQL Inspection

```bash
sqlite3 ~/Library/Application\ Support/LocalPush/ledger.db
> .tables
> SELECT * FROM delivery_queue;
> SELECT * FROM delivery_history WHERE completed_at > datetime('now', '-1 hour');
```

### Clippy Warnings

Fix all before committing:
```bash
cargo clippy -- -D warnings
```

---

## References

- **Root Instructions:** `../CLAUDE.md`
- **Frontend Instructions:** `../src/CLAUDE.md`
- **Main Plan:** `../PLAN.md`
- **Tauri API:** https://docs.rs/tauri/
- **SQLite Docs:** https://www.sqlite.org/
- **Tokio Guide:** https://tokio.rs/
