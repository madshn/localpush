# LocalPush Resume Prompt

**Last Updated:** 2026-02-05
**Project Path:** `~/dev/localpush`
**GitHub:** https://github.com/madshn/localpush
**Vision Doc:** https://www.notion.so/ownbrain/LocalPush-Open-Source-File-Webhook-Bridge-2fbc84e67cc481b69522f87f17b9aed7
**Status:** v0.1.3 released — core infrastructure complete, ready for feature development

---

## Resume Prompt

Copy and paste this to continue work:

```
Resume LocalPush build at ~/dev/localpush. Read RESUME.md for context.

STATUS: v0.1.3 released and working. Menu bar app with arrow icon, popover UI, delivery pipeline verified end-to-end.

WHAT'S BUILT:
- Tauri 2.0 macOS menu bar app (Rust + React)
- SQLite WAL delivery ledger with guaranteed delivery (5-state machine)
- Claude Code Stats source (watches ~/.claude/stats-cache.json)
- Webhook delivery to n8n (tested, working)
- 42 tests passing (37 Rust + 5 integration)
- Homebrew Cask distribution (brew tap madshn/localpush)
- Auto-update via GitHub Releases

WHAT'S NEXT (from vision doc):
1. More northbound targets: ntfy (mobile push), Make, Zapier, Home Assistant
2. More southbound sources: Apple Podcasts, Apple Finance, Screen Time, Browser History
3. Push resolution options: Streaming (<5s), Near-real-time (30-60s), Hourly, Daily, Weekly
4. Radical transparency: Pre-enable preview showing YOUR real data before connecting
5. Future: Local AI privacy guardian (Apple Intelligence/Ollama)

ARCHITECTURE:
- src-tauri/src/traits/ — All abstractions (CredentialStore, FileWatcher, WebhookClient, DeliveryLedger)
- src-tauri/src/production/ — Real implementations (Keychain, FSEvents, Reqwest)
- src-tauri/src/sources/ — Data source plugins (claude_stats.rs is the template)
- src-tauri/src/source_manager.rs — Source registry + file event routing
- src/components/ — React UI (StatusIndicator, SourceList, DeliveryQueue, SettingsPanel)

KEY DECISIONS:
- Trait-based DI for 100% testable Rust
- SQLite WAL for crash-safe guaranteed delivery
- tauri::async_runtime::spawn (NOT tokio::spawn) for Tauri context
- 22x22 PNG template icon for macOS menu bar
- Mutex<Connection> for rusqlite thread safety

LEARNINGS: See learnings/ directory for patterns discovered during development.
```

---

## Current State (2026-02-05)

### What Works
- Full trait-based architecture (CredentialStore, FileWatcher, WebhookClient, DeliveryLedger)
- SQLite WAL ledger with 5-state machine (Pending → InFlight → Delivered/Failed/DLQ)
- Production implementations: Keychain, FSEvents, Reqwest webhook client
- Claude Code Stats source plugin (parses ~/.claude/stats-cache.json)
- Source manager with enable/disable and file event routing
- All Tauri commands wired (12 commands registered)
- Frontend: StatusIndicator, SourceList, DeliveryQueue, SettingsPanel
- Logging: tracing with daily file rotation + stdout
- Auto-update: tauri-plugin-updater configured with GitHub Releases
- n8n test endpoint: https://flow.rightaim.ai/webhook/localpush-ingest
- Homebrew tap: https://github.com/madshn/homebrew-localpush
- Menu bar popover: positions below tray icon, toggles on click, dismisses on blur

### Release History
| Version | Date | Changes |
|---------|------|---------|
| v0.1.0 | 2026-02-05 | Initial release — crash fixes, signing key |
| v0.1.1 | 2026-02-05 | Tray positioning, blur dismiss, toggle |
| v0.1.2 | 2026-02-05 | PNG decode fix (include_image macro) |
| v0.1.3 | 2026-02-05 | 22x22 icon size for menu bar compatibility |

---

## Vision Summary

**Core Principles:**
1. **Guaranteed Delivery** — WAL pattern ensures no data loss (survives crashes, reboots, network outages)
2. **Radical Transparency** — See YOUR real data before enabling any source

**Northbound Targets (7):**
- n8n (MVP ✓), ntfy, Make, Zapier, Pipedream, Home Assistant, Custom

**Southbound Sources (30+ planned, 6 tiers):**
- Tier 1 MVP: Claude Code Stats ✓, Claude Sessions, Apple Podcasts, Apple Finance
- Tier 2: Browser History, Screen Time, Notion Local Cache, Git Repos
- Tier 3: Arc Browser, Safari Reading List, Downloads, Notes, Reminders, Calendar, Screenshots
- Tier 4-6: Dev tools, Relationships (metadata), Media consumption

**Push Resolutions:**
- Streaming (<5s), Near-real-time (30-60s), Hourly, Daily, Weekly

**Future:** Local AI privacy guardian using Apple Intelligence/Ollama for intelligent data triage

---

## Key Files

| File | Purpose |
|------|---------|
| `src-tauri/src/lib.rs` | App setup, tray, auto-update |
| `src-tauri/src/traits/` | All trait abstractions |
| `src-tauri/src/production/` | Real implementations |
| `src-tauri/src/sources/claude_stats.rs` | Template for new sources |
| `src-tauri/src/source_manager.rs` | Source registry |
| `src-tauri/src/ledger.rs` | SQLite WAL delivery ledger |
| `src/App.tsx` | React frontend entry |
| `src/components/` | UI components |

---

## Verification

```bash
cd ~/dev/localpush
./scripts/verify.sh        # Full verification
npm run tauri dev          # Launch dev build
```
