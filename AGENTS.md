# LocalPush (Codex Project Guide)

Codex-facing project guidance for LocalPush. This is a selective mirror of the `CLAUDE.md` hierarchy with Claude-specific persona/process instructions removed.

## Purpose

LocalPush is a macOS menu bar app that watches local files and delivers them to webhooks with guaranteed delivery (SQLite WAL-backed ledger) and transparent payload previews before users enable a source.

Use this file for repo-level product context and invariants. Use subdirectory guides for implementation patterns:

- `src/AGENTS.md` (frontend)
- `src-tauri/AGENTS.md` (backend)

## Product Goal (North Star)

Prove that local-first data push with guaranteed delivery creates real user adoption, especially for non-developer automation users.

## Product Priorities

- Preserve delivery guarantees over convenience.
- Preserve radical transparency (show real payloads before enablement).
- Optimize for proof of life and daily use before polish.
- Ship measurable improvements and instrument outcomes.

## Delivery Guarantee Contract (Do Not Weaken Silently)

- Crash-safe writes: delivery state is persisted via SQLite WAL.
- Retry on failure: failed deliveries remain in the ledger and are retried.
- No silent drops: events are ledgered before delivery attempts.
- Binding resolution fallback: if no bindings exist, legacy global webhook fallback may apply (v0.1 compatibility).
- Visibility: pending/delivering/delivered/failed states are queryable.

What "guaranteed" does not mean:

- Not guaranteed network uptime.
- Not strict ordering.
- Not exactly-once delivery (targets should be idempotent).

Any feature that weakens these guarantees should be called out explicitly before shipping.

## Architecture Summary

Flow:

1. Source fires.
2. Source manager parses and enqueues payload to ledger.
3. Delivery worker polls pending entries.
4. Bindings resolve source to one or more target endpoints.
5. HTTP POST executes delivery.
6. If no bindings exist, legacy global webhook fallback may be used.

Key constraint:

- Sources are southbound (local data in).
- Targets are northbound (data out).
- Bindings connect them.
- Delivery worker is the only component that should touch the network path for event delivery.

## macOS / Tauri Constraints

- Use `tauri::async_runtime::spawn` (not `tokio::spawn`) for spawned tasks that require Tauri context.
- Use proper synchronization around rusqlite connections (`Mutex<Connection>` pattern).
- Credential behavior differs by environment:
  - Dev: file-based credentials (`dev-credentials.json`)
  - Prod: macOS Keychain
- File watching uses FSEvents via `notify-rs` with debounce.

## Extension Model (Sources / Targets)

Adding a source is a bounded task:

- Implement `Source` trait
- Register in source manager/registry
- Ensure UI exposes it

Adding a target is a bounded task:

- Implement `Target` trait
- Add connect command(s)
- Add frontend setup form
- Register startup restoration

See `src-tauri/AGENTS.md` for backend patterns and `src/AGENTS.md` for frontend UI/IPC patterns.

## Verification Gates

Run the relevant checks before claiming completion.

Backend (from `src-tauri/`):

```bash
cargo test
cargo clippy -- -D warnings
```

Frontend:

```bash
npm run lint
npm run typecheck
npm test
```

Build sanity:

```bash
cargo build --release
```

If a requested change only touches docs or a narrow area, run the smallest relevant verification set and state what you did not run.

## Roadmap / Ownership Context

- Product roadmap lives in `ROADMAP.md`.
- Prefer staying within LocalPush product boundaries (sources, targets, bindings, delivery pipeline, UI, tests, docs).
- Escalate/call out cross-project coupling, major framework upgrades, or changes that alter product phase assumptions.

## Porting Note

Until sync automation exists, `CLAUDE.md` files may evolve independently. When updating repo guidance, prefer changing both the Claude and Codex mirrors (or move shared material into tool-neutral docs and link from both).
