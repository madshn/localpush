# LocalPush

macOS menu bar app that watches local files and delivers them to webhooks with **guaranteed delivery** (WAL pattern) and **radical transparency** (users see their real data before enabling).

**Problem:** Metrick lost 7 days of Claude Code token data due to cron timing. LocalPush solves this with event-driven, crash-safe delivery.

**Branch:** `feature/v0.2-multi-source` (worktree at `.trees/v0.2`)

---

## v0.2 Architecture

v0.2 introduces **multi-source, multi-target delivery with per-binding routing**.

```
Sources (Southbound)          Bindings              Targets (Northbound)
────────────────────         ─────────             ───────────────────
claude-stats ──────┐                               ┌── n8n (webhook endpoints)
claude-sessions ───┤── SourceBinding ──────────────┤── ntfy (push topics)
apple-podcasts ────┤   (source→endpoint)           └── (future: Make, Zapier...)
apple-notes ───────┤
apple-photos ──────┘
                    │
              SourceManager          DeliveryWorker
              (parse + enqueue)      (poll ledger → resolve bindings → POST)
                    │                       │
                    └──── SQLite Ledger ─────┘
                          (WAL mode, crash-safe)
```

### Data Flow

1. **Source fires** → SourceManager parses → enqueues payload to Ledger
2. **DeliveryWorker polls** (every 5s) → picks up pending entries
3. **Binding resolution** → looks up bindings for source_id → gets target endpoints
4. **HTTP POST** → sends to each bound endpoint
5. **Fallback** → if no bindings, tries legacy global webhook (v0.1 compat)

### Tech Stack

| Layer | Technology | Notes |
|-------|-----------|-------|
| **Frontend** | React 18 + TypeScript + Vite | Menu bar UI, 420x680 window |
| **IPC** | Tauri 2.0 | Type-safe command bridge |
| **Backend** | Rust (Tokio async) | Trait-based DI, fully testable |
| **Storage** | SQLite + WAL | config.sqlite + ledger.sqlite |
| **Credentials** | Dev: file-based / Prod: Keychain | dev-credentials.json avoids Keychain prompts |
| **Deps** | Keychain, FSEvents, Reqwest | macOS native integrations |

---

## Key Design Principles

1. **Guaranteed Delivery** — WAL (Write-Ahead Logging) survives crashes. No data loss.
2. **Radical Transparency** — Users preview their real data before enabling any source.
3. **Trait-Based DI** — All external dependencies behind traits → 100% testable without mocks.
4. **Per-Binding Routing** — Each source binds to specific target endpoints (v0.2), with v0.1 global webhook fallback.
5. **Type Safety** — TypeScript strict + Rust everywhere. No string types for state.

---

## Project Structure

```
src-tauri/src/
├── main.rs                    # Tauri entry point, window setup
├── lib.rs                     # Library exports, setup_app()
├── state.rs                   # AppState (DI container)
├── config.rs                  # SQLite config store (app_config table)
├── ledger.rs                  # SQLite delivery ledger (WAL mode)
├── delivery_worker.rs         # Background worker: poll → resolve bindings → POST
├── bindings.rs                # Source-to-target binding store (persisted in config)
├── source_manager.rs          # Source registry + file event routing
├── target_manager.rs          # Target registry (in-memory, restored from config)
├── traits/
│   ├── mod.rs                 # Export all traits
│   ├── credential_store.rs    # Store/retrieve secrets
│   ├── file_watcher.rs        # Watch file changes
│   ├── webhook_client.rs      # Send HTTP requests
│   ├── delivery_ledger.rs     # Delivery state CRUD
│   └── target.rs              # Target trait (v0.2: test_connection, list_endpoints)
├── production/
│   ├── mod.rs                 # Export all impls
│   ├── credential_store.rs    # macOS Keychain impl
│   ├── dev_credential_store.rs # File-based dev credential store (no Keychain prompts)
│   ├── file_watcher.rs        # FSEvents (notify-rs) impl
│   └── webhook_client.rs      # Reqwest HTTP impl
├── mocks/
│   └── mod.rs                 # All mock impls for testing
├── sources/
│   ├── mod.rs                 # Source trait + registry
│   ├── claude_stats.rs        # Parse ~/.claude/stats-cache.json
│   ├── claude_sessions.rs     # Parse Claude Code session data
│   ├── apple_podcasts.rs      # Apple Podcasts library
│   ├── apple_notes.rs         # Apple Notes
│   └── apple_photos.rs        # Apple Photos metadata
└── targets/
    ├── mod.rs                 # Target module exports
    ├── n8n.rs                 # n8n target (discovers webhook endpoints via REST API)
    └── ntfy.rs                # ntfy push notification target

src/
├── App.tsx                    # Main UI entry with tab navigation
├── main.tsx                   # React entry point
├── styles.css                 # Minimal CSS
├── utils/logger.ts            # Frontend logging utility
├── api/hooks/
│   ├── useDeliveryStatus.ts   # Query delivery stats
│   ├── useSources.ts          # Query configured sources
│   ├── useDeliveryQueue.ts    # Query in-flight deliveries
│   ├── useTargets.ts          # Query connected targets
│   ├── useBindings.ts         # Query/mutate source-target bindings
│   └── useActivityLog.ts      # Query delivery activity log
└── components/
    ├── SourceList.tsx          # Source list with enable/bind/push controls
    ├── EndpointPicker.tsx      # Pick target endpoint for source binding
    ├── SecurityCoaching.tsx    # Educate users about data being sent
    ├── TransparencyPreview.tsx # Show real data before enabling
    ├── TargetSetup.tsx         # Add new targets (n8n, ntfy)
    ├── N8nConnect.tsx          # n8n instance connection form
    ├── NtfyConnect.tsx         # ntfy server connection form
    ├── ActivityLog.tsx         # Delivery activity log display
    ├── TrafficLight.tsx        # Status indicator (green/yellow/red)
    ├── StatusIndicator.tsx     # Overall app health
    ├── DeliveryQueue.tsx       # In-flight deliveries display
    └── SettingsPanel.tsx       # Settings and webhook config
```

---

## Tauri Commands (v0.2)

| Command | Purpose |
|---------|---------|
| `get_delivery_status` | Overall delivery health |
| `get_sources` | List registered sources |
| `enable_source` / `disable_source` | Toggle source |
| `get_source_preview` | Radical transparency preview |
| `trigger_source_push` | Manual "Push Now" (parse + enqueue) |
| `connect_n8n_target` | Register n8n instance (URL + API key) |
| `connect_ntfy_target` | Register ntfy server |
| `list_targets` | List connected targets |
| `test_target_connection` | Test connectivity |
| `list_target_endpoints` | List endpoints for a target |
| `create_binding` / `remove_binding` | Source-to-endpoint binding |
| `get_source_bindings` / `list_all_bindings` | Query bindings |
| `add_webhook_target` / `test_webhook` | Legacy v0.1 webhook config |
| `get_webhook_config` | Legacy webhook config read |
| `get_setting` / `set_setting` | Generic config store |
| `retry_delivery` | Reset failed delivery to pending |
| `get_delivery_queue` | Full delivery queue |

---

## Verification Gates (Pre-Commit)

Every change must pass:

```bash
# Backend (from src-tauri/)
cargo test                    # 80 unit + 5 integration tests
cargo clippy -- -D warnings   # Rust linting

# Frontend
npm run lint                  # ESLint strict
npm run typecheck             # TypeScript strict
npm test                      # Vitest

# Build
cargo build --release         # Final sanity check
```

**Golden Rule:** If verification fails, the change doesn't ship.

---

## Development Setup

```bash
cd ~/dev/localpush/.trees/v0.2

# Kill any existing LocalPush instances (old prod app conflicts)
pkill -f LocalPush || true
# Free port 1420 if needed
lsof -ti:1420 | xargs kill -9 2>/dev/null || true

# Start dev server
npx tauri dev
```

App appears as menu bar icon → click to open 420x680 window.

### Dev Credential Store

In dev mode, credentials use `dev-credentials.json` (file-based) instead of macOS Keychain. This avoids Keychain permission prompts during development.

- Path: `~/Library/Application Support/com.localpush.app/dev-credentials.json`
- Config DB: `~/Library/Application Support/com.localpush.app/config.sqlite`
- Ledger DB: `~/Library/Application Support/com.localpush.app/ledger.sqlite`

---

## Testing Strategy

| Layer | Tool | Count | Pattern |
|-------|------|-------|---------|
| **Rust unit** | cargo test | 80 | `#[test]` / `#[tokio::test]` in modules |
| **Rust integration** | cargo test | 5 | `tests/integration_test.rs` |
| **Frontend** | Vitest | — | Mock tauri invoke, test React components |

Integration tests cover:
- Full pipeline: enable → event → deliver
- Retry on webhook failure
- Disabled source ignores events
- Orphan recovery + redelivery
- Multiple events batch delivery

---

## Known Issues

1. **UX: Enable checkbox confusing** — "I did not recognize it as a checkbox". Defer to Google Stitch for redesign.
2. **Old production LocalPush.app conflicts** — Kill before dev testing.
3. **Port 1420 may be held** — `lsof -ti:1420 | xargs kill -9`

---

## Key Decisions

- `tauri::async_runtime::spawn` (NOT `tokio::spawn`) for Tauri context
- `Mutex<Connection>` for rusqlite thread safety
- 22x22 PNG template icon for macOS menu bar
- Per-binding routing in delivery worker with legacy global webhook fallback
- Dev credential store (file-based) to avoid Keychain prompts in development
- Config persisted in SQLite `app_config` table (not flat files)
- Targets restored from config on startup (URL + type stored, API key in keychain/dev-creds)

---

## Adding a New Source

1. Create `sources/my_source.rs` implementing `Source` trait (see `claude_stats.rs` as template)
2. Add to `sources/mod.rs` pub exports
3. Register in `state.rs` `SourceManager` initialization
4. Run `cargo test` from `src-tauri/`

The source will automatically appear in the frontend SourceList and be available for binding.

## Adding a New Target Type

1. Create `targets/my_target.rs` implementing `Target` trait (see `n8n.rs` as template)
2. Add to `targets/mod.rs` pub exports
3. Add connect command in `commands/mod.rs`
4. Add frontend connect form component
5. Add startup restoration logic in `state.rs`

---

## References

- **Detailed Plan:** `PLAN.md`
- **Resume Prompt:** `RESUME.md`
- **Frontend Instructions:** `src/CLAUDE.md`
- **Backend Instructions:** `src-tauri/CLAUDE.md`
- **Tauri Docs:** https://tauri.app/en/develop/
- **SQLite WAL:** https://www.sqlite.org/wal.html
- **Vision Doc:** https://www.notion.so/ownbrain/LocalPush-Open-Source-File-Webhook-Bridge-2fbc84e67cc481b69522f87f17b9aed7
