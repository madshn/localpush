# LocalPush

Codex-facing guide for `localpush`. This should stay aligned with [CLAUDE.md](CLAUDE.md), which is the canonical project operating guide.

Use this file for the structural model, guardrails, and Codex-specific lane. Use `CLAUDE.md` when you need the full version.

## Project Identity

| Field | Value |
|-------|-------|
| Repo | `madshn/localpush` |
| Classification | Build (product) |
| Path | `~/builds/localpush` |
| Owner | Bob |
| Shorthand | `lpush` |
| Domain | macOS menu bar app |
| Hosting | Local (Tauri / .dmg distribution) |
| Codex identity in this repo | Product Coordinator (`lpush`) |

## Architecture

macOS menu bar app that watches local files and delivers them to webhooks with guaranteed delivery (WAL pattern) and radical transparency (users see real data before enabling).

```
Sources (Southbound)          Bindings              Targets (Northbound)
────────────────────         ─────────             ───────────────────
claude-stats ──────┐                               ┌── n8n (webhook endpoints)
claude-sessions ───┤                               ├── ntfy (push topics)
codex-stats ───────┤── SourceBinding ──────────────┤
codex-sessions ────┤   (source→endpoint)           └── (future: Make, Zapier...)
apple-podcasts ────┤
apple-notes ───────┤
apple-photos ──────┤
cic-task-output ───┤
desktop-activity ──┘
                    │
              SourceManager          DeliveryWorker
              (parse + enqueue)      (poll ledger → resolve bindings → POST)
                    │                       │
                    └──── SQLite Ledger ─────┘
                          (WAL mode, crash-safe)
```

**Data flow:** Source fires → SourceManager parses → enqueues to Ledger → DeliveryWorker polls → resolves bindings → HTTP POST to target endpoints.

**Key constraint:** Sources are southbound (local data in), targets are northbound (data out). Bindings connect them. The delivery worker is the only component that touches the network for event delivery.

### Sources / Targets / Bindings

- **Adding a source:** Implement `Source` trait → register in SourceManager → auto-appears in UI. See `src-tauri/AGENTS.md`.
- **Adding a target:** Implement `Target` trait → add connect command → add frontend form → register startup restoration. See `src-tauri/AGENTS.md`.

### Delivery Guarantee Contract

| Guarantee | Implementation |
|-----------|---------------|
| Crash-safe | SQLite WAL mode — writes survive app crashes mid-delivery |
| Retry on failure | Failed deliveries stay in ledger, retried on next poll cycle (5s) |
| No silent drops | Every source event is ledgered before any delivery attempt |
| Binding resolution | Deliveries go only to configured bindings/targets |
| Visibility | Every delivery state (pending → delivering → delivered/failed) is queryable |

What "guaranteed" does **not** mean: network uptime, strict ordering, exactly-once delivery (targets must be idempotent).

Any feature that weakens these guarantees requires explicit user approval before shipping.

## Stack

| Layer | Technology | Notes |
|-------|-----------|-------|
| Frontend | React 18 + TypeScript + Vite | Menu bar UI, 420x680 window |
| IPC | Tauri 2.0 | Type-safe command bridge |
| Backend | Rust (Tokio async) | Trait-based DI, fully testable |
| Storage | SQLite + WAL | config.sqlite + ledger.sqlite |
| Credentials | Dev: file-based / Prod: Keychain | dev-credentials.json avoids Keychain prompts |
| Platform | FSEvents via notify-rs | 300ms debounce, macOS only |

**Key rule:** Use `tauri::async_runtime::spawn` (NOT `tokio::spawn`) for spawned work that needs Tauri context. Use `Mutex<Connection>` for rusqlite thread safety.

## Key Components

| Path | Purpose |
|------|---------|
| `src-tauri/src/sources/` | Source implementations (Claude, Codex, Apple, CiC task output, desktop activity) |
| `src-tauri/src/targets/` | Target implementations (n8n webhook, ntfy) |
| `src-tauri/src/delivery/` | DeliveryWorker — polls ledger, resolves bindings, POSTs |
| `src-tauri/src/ledger/` | SQLite WAL ledger — crash-safe event queue |
| `src/` | React frontend — menu bar UI, source/target/binding forms |
| `src/AGENTS.md` | Frontend implementation patterns |
| `src-tauri/AGENTS.md` | Backend trait details and examples |
| `.claude/agents/localpush-agent.md` | Implementation worker |
| `ROADMAP.md` | Phase-locked roadmap |

## Verification Gates

Every change must pass before claiming complete:

```bash
# Backend (from src-tauri/)
cargo test                    # Unit + integration tests
cargo clippy -- -D warnings   # Rust linting

# Frontend
npm run lint                  # ESLint strict
npm run typecheck             # TypeScript strict
npm test                      # Vitest

# Build
cargo build --release         # Final sanity check
```

**Golden Rule:** If verification fails, the change does not ship.

## Team Communication

| Question about | Ask |
|---------------|-----|
| Factory standards, cross-project patterns, phase transitions | `@bob` |
| Build/release pipeline, code signing, .dmg distribution | `@mira` |
| Delivery rates, active sources, install metrics | `@metrick` |
| Product strategy, competitive landscape, human escalation | `@aston` |
| Content, marketing, frontend UX | `@leah` |
| Revenue, clients, commercial | `@rex` |

## Messaging

### Three-Plane Model

The team messaging system operates on three data planes:

- `team_messages`: content and transcript plane
- `team_message_deliveries`: routing and ownership plane
- `team_message_reactions`: progress and protocol-state plane

A message without a delivery was posted but never routed.

### Done vs Complete

- `done` = this turn is answered, conversation remains open
- `complete` = conversation is terminated for routing purposes

Do not treat `done` as terminal.

### Conversation Rules

- `conversation_id` is server-assigned (from `post_team_message` RPC return)
- Fresh thread for new topics
- Reuse only for direct follow-ups via `--follow-up`
- When in doubt, start a new conversation

## Codex Lane

- Repo-local Codex WalkieTalkie: [.codex/skills/walkietalkie/SKILL.md](.codex/skills/walkietalkie/SKILL.md)
- Helper scripts: [.codex/](.codex/)
- Identity on bus: `lpush`

## Constraints

- Do not commit credentials, `dev-credentials.json`, or `.env*` files
- Do not weaken the WAL delivery guarantee without explicit user approval
- All network delivery goes through DeliveryWorker — sources never POST directly
- Use `tauri::async_runtime::spawn`, not `tokio::spawn`, for Tauri-context work
- Keep changes scoped to the issue you were asked to work on
- Escalate to `@bob` for cross-project changes, major dependency upgrades, or phase decisions

## Git

- Branch: `{type}/{issue-number}-{short-description}`
- Open a PR instead of pushing directly to `main`
- Conventional commits:

```
{type}({scope}): {subject}

Closes #{issue-number}

Co-Authored-By: Claude <noreply@anthropic.com>
```

Types: `feat`, `fix`, `style`, `refactor`, `chore`, `docs`, `test`

## Key References

- Canonical operating guide: [CLAUDE.md](CLAUDE.md)
- Roadmap: [ROADMAP.md](ROADMAP.md)
- Backend patterns: [src-tauri/AGENTS.md](src-tauri/AGENTS.md)
- Frontend patterns: [src/AGENTS.md](src/AGENTS.md)
- Factory manager: `~/team/bob/`
- GitHub: https://github.com/madshn/localpush
