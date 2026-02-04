# LocalPush

macOS menu bar app that watches local files and delivers them to webhooks with **guaranteed delivery** (WAL pattern) and **radical transparency** (users see their real data before enabling).

**Problem:** Metrick lost 7 days of Claude Code token data due to cron timing. LocalPush solves this with event-driven, crash-safe delivery.

---

## Architecture

```
┌─────────────────────────────────────────────┐
│  LocalPush                                  │
├─────────────────────────────────────────────┤
│  Menu Bar UI (React)  ◀─ Tauri IPC ─▶      │
│                                             │
│  Rust Backend + SQLite (WAL mode)           │
│  ├─ Traits (DI) — All deps injectable      │
│  ├─ Production (Keychain, FSEvents, HTTP)  │
│  └─ Mocks (In-memory for tests)            │
└─────────────────────────────────────────────┘
```

### Tech Stack

| Layer | Technology | Notes |
|-------|-----------|-------|
| **Frontend** | React 18 + TypeScript + Vite | Menu bar UI, transparent window |
| **IPC** | Tauri 2.0 | Type-safe command bridge |
| **Backend** | Rust (Tokio async) | Trait-based DI, fully testable |
| **Storage** | SQLite + WAL | Guaranteed delivery, crash-safe |
| **Deps** | Keychain, FSEvents, Reqwest | macOS native integrations |

---

## Key Design Principles

1. **Guaranteed Delivery** — WAL (Write-Ahead Logging) survives crashes. No data loss.
2. **Radical Transparency** — Users preview their real data before enabling any source.
3. **Trait-Based DI** — All external dependencies behind traits → 100% testable without mocks.
4. **Type Safety** — TypeScript strict + Rust everywhere. No string types for state.
5. **Async by Default** — Tokio runtime handles all blocking operations.

---

## Project Structure

```
~/dev/localpush/
├── src/                    # Frontend (React)
│   ├── CLAUDE.md           # Frontend instructions
│   ├── components/         # UI components
│   ├── api/hooks/          # Tauri IPC hooks
│   └── App.tsx             # Main UI entry
├── src-tauri/              # Backend (Rust)
│   ├── CLAUDE.md           # Backend instructions
│   ├── src/
│   │   ├── traits/         # DI trait definitions
│   │   ├── production/     # Real implementations
│   │   ├── mocks/          # Test doubles
│   │   ├── sources/        # File source parsers
│   │   ├── ledger.rs       # SQLite delivery ledger
│   │   ├── commands/       # Tauri commands
│   │   ├── state.rs        # App state + DI
│   │   └── main.rs         # Tauri entry
│   └── Cargo.toml          # Dependencies
├── PLAN.md                 # Implementation plan
└── package.json            # npm scripts
```

---

## Verification Gates (Pre-Commit)

Every change must pass:

```bash
# Frontend
npm run lint               # ESLint strict
npm run typecheck          # TypeScript strict
npm test                   # Vitest

# Backend
cargo test                 # Rust unit + integration tests
cargo clippy -- -D warnings  # Rust linting

# Integration
npm run check              # Runs all above

# Build
cargo build --release      # Final sanity check
```

**Golden Rule:** If verification fails, the change doesn't ship.

---

## Development Workflow

### Adding a New Feature

1. **Start in Backend (src-tauri/)**
   - Add trait method to `traits/*.rs` if new capability
   - Implement in `production/*.rs`
   - Add mock in `mocks/*.rs`
   - Write Rust tests
   - Ensure `cargo test` passes

2. **Add Tauri Command** (in `commands/mod.rs`)
   - Expose backend as Tauri command
   - Use `AppState` for dependency injection
   - Return `Result<T, String>` for error handling

3. **Add Frontend Hook** (in `src/api/hooks/`)
   - Use `useQuery` for read operations
   - Wrap Tauri command invocation
   - Handle loading/error states

4. **Add UI Component** (in `src/components/`)
   - Use hook from step 3
   - Render loading/error/success states
   - Integrate into App.tsx if needed

5. **Test Integration**
   - Run full verification suite
   - Manual smoke test in dev mode: `npm run tauri dev`

---

## Testing Strategy

| Layer | Tool | Pattern |
|-------|------|---------|
| **Traits** | Rust unit tests | `#[test]` in trait file |
| **Production** | Rust integration | Create mock dependencies, test real impl |
| **Commands** | Rust integration | Mock all traits, test command behavior |
| **Frontend** | Vitest + mockIPC | Mock tauri invoke, test React components |
| **E2E** | Playwright (manual) | Full app smoke tests |

**Key Rule:** Never test implementation details. Test behavior and contracts.

---

## Debugging

### Logs (Rust Backend)

```bash
RUST_LOG=localpush=debug npm run tauri dev
```

Logs go to console + `~/.local/share/LocalPush/` on macOS.

### Logs (Frontend)

Check browser dev tools:
- `npm run tauri dev` → press F12 in app window

### SQLite Ledger

```bash
# Inspect ledger database
sqlite3 ~/Library/Application\ Support/LocalPush/ledger.db

# View delivery queue
SELECT id, file_path, webhook_url, status, retries FROM delivery_queue ORDER BY created_at DESC;

# View delivery history
SELECT * FROM delivery_history ORDER BY completed_at DESC LIMIT 10;
```

---

## Integration Points

### File Sources

Sources (in `src-tauri/src/sources/`) parse local files and expose as `Source`:

```rust
pub trait Source: Send + Sync {
    async fn get_entries(&self) -> Result<Vec<Entry>, SourceError>;
    async fn watch(&self) -> Result<(), SourceError>;
}
```

**Examples:**
- `claude_stats.rs` — Parse `~/.claude/stats-cache.json`
- Add more in `sources/mod.rs`

### Webhook Delivery

`traits/webhook_client.rs` defines HTTP contract:

```rust
pub async fn deliver(&self, req: WebhookRequest) -> Result<WebhookResponse, WebhookError>
```

Supports all auth types: None, Header, Bearer, Basic.

---

## Dependencies & Versions

| Crate | Version | Why |
|-------|---------|-----|
| `tauri` | 2.x | Menu bar + IPC |
| `tokio` | 1.x | Async runtime |
| `rusqlite` | 0.32 | SQLite driver |
| `notify-debouncer-full` | 0.4 | File watching (300ms debounce) |
| `keyring` | 3.x | macOS Keychain |
| `reqwest` | 0.12 | HTTP client (async) |

**If updating:** Verify `cargo test && cargo clippy` still pass.

---

## Common Tasks

### Add a New Webhook Auth Type

1. Add variant to `WebhookAuth` enum in `traits/webhook_client.rs`
2. Update `ReqwestWebhookClient` to handle it in `production/webhook_client.rs`
3. Add test case to mock and production
4. Run `cargo test webhook`

### Add a New Source

1. Create `sources/my_source.rs` implementing `Source` trait
2. Add to `sources/mod.rs` pub exports
3. Create mock in `mocks/mod.rs`
4. Wire into `AppState` in `state.rs`
5. Add Tauri command in `commands/mod.rs`
6. Run full verification

### Debug Delivery Failure

1. Check logs: `RUST_LOG=localpush::ledger=debug`
2. Inspect SQLite: `SELECT * FROM delivery_queue WHERE status != 'delivered';`
3. Add test case reproducing failure
4. Fix in production impl
5. Verify with `cargo test`

---

## References

- **Detailed Plan:** `PLAN.md`
- **Frontend Instructions:** `src/CLAUDE.md`
- **Backend Instructions:** `src-tauri/CLAUDE.md`
- **Tauri Docs:** https://tauri.app/en/develop/
- **SQLite WAL:** https://www.sqlite.org/wal.html

---

## Getting Started (First Time)

```bash
cd /Users/madsnissen/dev/localpush

# Install dependencies
npm install
cargo fetch --manifest-path src-tauri/Cargo.toml

# Verify setup
npm run check  # Should pass all gates

# Start dev mode
npm run tauri dev  # Opens app in menu bar
```

Press Ctrl+C to stop. App state is preserved in SQLite.
