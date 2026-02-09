# LocalPush Resume Prompt

**Last Updated:** 2026-02-09
**Project Path:** `~/dev/localpush`
**Branch:** `main`
**GitHub:** https://github.com/madshn/localpush
**Vision Doc:** https://www.notion.so/ownbrain/LocalPush-Open-Source-File-Webhook-Bridge-2fbc84e67cc481b69522f87f17b9aed7
**Status:** v0.2 merged to main — multi-source, multi-target, per-binding routing

---

## Resume Prompt

Copy and paste this to continue work:

```
Resume LocalPush at ~/dev/localpush (branch: main). Read CLAUDE.md for full architecture and PC identity.

STATUS: v0.2 merged to main. Full delivery pipeline verified end-to-end. 80 unit + 5 integration tests passing.

WHAT'S WORKING:
- Multi-source, multi-target architecture with per-binding routing
- Claude Code Statistics source: enabled, bound to n8n endpoint
- n8n target restored on restart (credentials in dev-credentials.json)
- Binding: claude-stats → n8n webhook "LocalPush Ingestion Test" at flow.rightaim.ai
- "Push Now" button triggers source parse + enqueue (delivery worker picks up ≤5s)
- Delivery worker: per-binding routing with legacy global webhook fallback
- Window: 420x680, resizable, min 360x400
- Dev credential store (file-based, no Keychain prompts)

SOURCES AVAILABLE:
- Claude Code Statistics (claude-stats) — enabled, bound, tested
- Claude Code Sessions (claude-sessions) — registered, not enabled
- Apple Podcasts, Apple Notes, Apple Photos — registered, not enabled

WHAT'S NEXT:
- Enable and test remaining sources (claude-sessions, apple-podcasts, etc.)
- UX improvements (checkbox discoverability, onboarding flow)
- See ROADMAP.md for phase-locked deliverables
```

---

## Current State (2026-02-09)

### What Works (Verified E2E)

- **Full delivery pipeline:** Source → Parse → Ledger → DeliveryWorker → Binding Lookup → HTTP POST
- **n8n target:** Connected (n8n-e2480372), credentials persisted in dev-credentials.json
- **Binding:** claude-stats → W9fgsdFjC3Fo4dvR ("LocalPush Ingestion Test") at https://flow.rightaim.ai/webhook/localpush-ingest
- **Push Now:** Manual trigger works — parse + enqueue → delivery worker picks up within 5s
- **Target restoration:** n8n targets restore from config on app restart
- **Tests:** 80 unit + 5 integration all passing
- **Factory adoption:** PC identity, ROADMAP.md, .claude/ agents and commands

### Sources

| Source | Status | Notes |
|--------|--------|-------|
| Claude Code Statistics (`claude-stats`) | Enabled, bound | Pushing real data to n8n |
| Claude Code Sessions (`claude-sessions`) | Registered | Not enabled |
| Apple Podcasts (`apple-podcasts`) | Registered | Not enabled |
| Apple Notes (`apple-notes`) | Registered | Not enabled |
| Apple Photos (`apple-photos`) | Registered | Not enabled |

### Targets

| Target | Status | Notes |
|--------|--------|-------|
| n8n (`n8n-e2480372`) | Connected | flow.rightaim.ai, API key in dev-creds |
| ntfy | Available | Not connected |

---

## Key Files

| File | Purpose |
|------|---------|
| `CLAUDE.md` | PC identity + technical docs |
| `ROADMAP.md` | Phase-locked roadmap |
| `src-tauri/src/delivery_worker.rs` | Background worker with per-binding routing |
| `src-tauri/src/bindings.rs` | Source-to-target binding persistence |
| `src-tauri/src/config.rs` | SQLite config store |
| `src-tauri/src/source_manager.rs` | Source registry + orchestration |
| `src-tauri/src/target_manager.rs` | Target registry (in-memory) |
| `src-tauri/src/targets/n8n.rs` | n8n target (API discovery of webhook endpoints) |
| `src-tauri/src/state.rs` | AppState DI container + startup restoration |
| `src-tauri/src/commands/mod.rs` | All Tauri commands (22 commands) |
| `src/components/SourceList.tsx` | Main source interaction UI |
| `src/components/EndpointPicker.tsx` | Target endpoint selection for binding |

---

## Verification

```bash
cd ~/dev/localpush/src-tauri
cargo test                    # 80 + 5 tests
cargo clippy -- -D warnings   # Clean

# Dev server
cd ~/dev/localpush
npx tauri dev
```

---

## Release History

| Version | Date | Changes |
|---------|------|---------|
| v0.1.0 | 2026-02-05 | Initial release — crash fixes, signing key |
| v0.1.1 | 2026-02-05 | Tray positioning, blur dismiss, toggle |
| v0.1.2 | 2026-02-05 | PNG decode fix (include_image macro) |
| v0.1.3 | 2026-02-05 | 22x22 icon size for menu bar compatibility |
| v0.2.0 | 2026-02-09 | Multi-source, multi-target, per-binding routing — merged to main |
