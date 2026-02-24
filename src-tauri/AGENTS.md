# LocalPush Backend (Codex Guide)

Backend guidance for `src-tauri/` (Rust + Tauri). This mirrors the portable parts of `src-tauri/CLAUDE.md`.

## Role

The backend is responsible for:

- Watching local files/events
- Persisting delivery state in SQLite (WAL)
- Retrying webhook deliveries
- Exposing Tauri commands to the frontend
- Preserving testability through trait-based dependency injection

## Architecture Principles

### 1) Trait-Based Dependency Injection

All external integrations should be behind traits (credential store, file watcher, webhook client, ledger, etc.).

Benefits:

- Production implementations for real integrations
- Mock implementations for deterministic tests
- Reduced coupling in command handlers and workers

### 2) SQLite WAL for Delivery Durability

Delivery state belongs in a WAL-backed ledger:

- Crash resilience
- Atomic writes
- Clear retry state tracking

Do not introduce shortcuts that bypass the ledger for delivery-critical events.

### 3) Async I/O

- Keep I/O async (HTTP, watchers, long-running tasks).
- Ensure spawned tasks that need Tauri context use `tauri::async_runtime::spawn`.

## Project Structure (Mental Model)

Common areas in `src-tauri/src/`:

- `main.rs` / `lib.rs`: app setup and entry points
- `state.rs`: app dependency container (`AppState`)
- `ledger.rs`: persistent delivery ledger
- `traits/`: interfaces for external dependencies
- `production/`: real implementations
- `mocks/`: test implementations
- `sources/`: source registry and source implementations
- `commands/`: Tauri command handlers

## AppState / Command Pattern

Preferred command pattern:

1. Extract `State<Arc<AppState>>`
2. Call trait-backed dependency methods
3. Map backend errors to `String` for Tauri response serialization
4. Return small, explicit response structs

This keeps command handlers thin and testable.

## Testing Strategy

### Unit Tests

- Test trait implementations in isolation.
- Use mocks/fakes for external boundaries.
- Prefer deterministic tests over timing-sensitive integration behavior where possible.

### Integration-Level Command Tests

- Build a mock `AppState`
- Seed mock ledger / dependencies
- Call command functions directly
- Assert serialized response behavior and state transitions

## Reliability Rules (Important)

- Ledger before delivery attempt (no silent drops).
- Failed deliveries must remain visible and retryable.
- Preserve at-least-once semantics; do not assume exactly-once.
- Make retry/backoff behavior explicit and inspectable.

## macOS / Platform Notes

- Credentials: dev may use file-based storage; production uses Keychain.
- File watcher implementation uses macOS/FSEvents via `notify-rs`.
- Tauri-specific runtime and thread-safety constraints apply (especially around rusqlite and spawned work).

## Extending the System

### Adding a Source

- Implement the `Source` trait
- Register it in the source registry/manager
- Ensure enqueue path writes to ledger
- Add/verify frontend visibility and settings UX

### Adding a Target

- Implement the `Target` trait
- Add connection/setup command(s)
- Register startup restoration
- Verify binding resolution and delivery worker integration

## Verification

Run the relevant backend checks from `src-tauri/`:

```bash
cargo test
cargo clippy -- -D warnings
```

Run additional checks (`cargo build --release`) when touching build/runtime-sensitive paths.
