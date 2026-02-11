# Roadmap: LocalPush

**Created:** 2026-02-04
**Current Phase:** 1
**Status:** IN_PROGRESS

---

## Vision

> Your Mac stores incredible data about your life — token usage, listening history, browsing patterns, financial transactions, screen time, photos, messages, health metrics. But it's all trapped in SQLite databases and proprietary formats. LocalPush unlocks it with **guaranteed delivery** and **radical transparency**.

**Two non-negotiable principles:**

1. **Guaranteed Delivery (WAL)** — Every capture writes to local ledger BEFORE transmission. Survives crashes, reboots, network outages, app reinstalls.
2. **Radical Transparency** — Before enabling any source, users see THEIR real data. Not samples, not descriptions. Plus ongoing payload inspection anytime.

**At full vision:** 30+ local data sources across 6 tiers, 7 webhook target platforms, push resolution from streaming (<5s) to weekly, and a local AI privacy guardian.

**Reference:** [Vision Document (Notion)](https://www.notion.so/ownbrain/LocalPush-Open-Source-File-Webhook-Bridge-2fbc84e67cc481b69522f87f17b9aed7)

---

## Phase Model

| Phase | Goal | Gate |
|-------|------|------|
| 1: Validation | Proof of life | External signal: installs, active sources, webhook deliveries |
| 2: Growth | Multi-source, multi-target scale | Business decision |
| 3: Real-Time | Streaming events + mobile alerts | User demand signal |
| 4: Intelligence | Local AI privacy guardian | Phase 3 mature + Apple Intelligence/Ollama viable |
| ∞: Sustain | Maintenance | Ongoing |

---

## Phase 1: Validation (ACTIVE)

**Status:** IN_PROGRESS
**Exit Criteria:** 10+ installs, 3+ active sources in use, confirmed webhook deliveries from non-developer users

### Requirements

| ID | Requirement | Status | Notes |
|----|-------------|--------|-------|
| REQ-001 | End-to-end delivery pipeline | [x] | Source → Ledger → Worker → Target |
| REQ-002 | Claude Code Statistics source | [x] | Enabled, bound, verified with real data |
| REQ-003 | n8n target with endpoint discovery | [x] | Connected, credentials persisted |
| REQ-004 | Per-binding routing (v0.2) | [x] | With v0.1 legacy fallback |
| REQ-005 | Push Now manual trigger | [x] | Parse + enqueue, worker picks up ≤5s |
| REQ-006 | Enable remaining sources | [ ] | claude-sessions, apple-podcasts, notes, photos |
| REQ-007 | UX improvements | [x] | Tailwind v4, Radix tabs, pipeline cards, activity log |
| REQ-008 | Homebrew Cask distribution | [x] | brew tap madshn/localpush |
| REQ-009 | Auto-update via GitHub Releases | [x] | tauri-plugin-updater configured |
| REQ-010 | Proof-of-life instrumentation | [ ] | Passive only: Homebrew tap + GitHub Release download counts. "Feedback" menu item opens GitHub Issues in browser. No telemetry SDK. Self-hosted dogfooding idea deferred to Phase 2 (REQ-034). |
| REQ-011 | Scheduled push cadence | [x] | Per-binding on_change/daily/weekly delivery modes |
| REQ-012 | BUG: Apple Photos source broken | [ ] | Source does not work — needs investigation and fix |
| REQ-013 | Dashboard kanban view | [ ] | 3-column kanban layout (see Stitch prototype) |
| REQ-014 | Make.com + Zapier connectors | [ ] | Same pattern as n8n connector (endpoint discovery) |
| REQ-015 | Delivery failure visibility & recovery | [ ] | Red tray icon on DLQ, macOS notification, error diagnosis with actionable guidance, data timeline gap awareness, replay confirmation. See `docs/research/spec-delivery-failure-visibility.md` |
| REQ-016 | Google Sheets target | [ ] | OAuth2 connect → pick spreadsheet → auto-create worksheet per source → flatten payloads to rows with proper formatting. First non-webhook target type. |
| REQ-017 | Apple Developer ID code signing | [ ] | $99/yr subscription → Developer ID cert → notarize builds → eliminates Gatekeeper warnings + Keychain prompts. Required for entitlements support. |

### Scope Boundaries

**In Scope:**
- Core delivery pipeline (sources → targets)
- macOS menu bar app
- n8n + ntfy targets
- Claude Code + Apple sources
- Homebrew distribution

**Out of Scope (Phase 2+):**
- Tier 2-6 sources (see Source Tiers below)
- Additional targets: Pipedream, Home Assistant, Custom (see Target Matrix)
- Push resolution tiers beyond event-driven (see Push Resolution)
- Local AI privacy guardian
- Windows/Linux support
- Event filtering, quiet hours, weekend mode
- Transform engine

### Work Log

| Date | What | Outcome |
|------|------|---------|
| 2026-02-04 | Initial scaffold | v0.1 created |
| 2026-02-05 | v0.1.0-v0.1.3 releases | Menu bar app, delivery pipeline |
| 2026-02-06 | v0.2 multi-source architecture | Targets, bindings, 5 sources |
| 2026-02-08 | E2E verification | Real data flowing to n8n |
| 2026-02-08 | Bob factory adoption | Factory standards applied |
| 2026-02-10 | UX overhaul + scheduled push cadence | Tailwind v4, Radix UI, pipeline cards, per-binding delivery modes |
| 2026-02-11 | v0.2.1 bug fix release | 13 bug fixes, DLQ notifications, tray indicator, entitlements research |

---
<!-- PHASE_GATE: Do not proceed until Phase 1 exit criteria met -->
---

## Phase 2: Growth (LOCKED)

**Status:** BLOCKED
**Prerequisite:** Phase 1 exit criteria achieved
**Exit Criteria:** Business decision — continue/pivot/sunset

### Requirements

| ID | Requirement | Status | Notes |
|----|-------------|--------|-------|
| REQ-020 | Desktop Session Logs source | [ ] | Daily push of active periods + top apps from CoreDuet knowledgeC.db. Requires FDA. See `docs/research/spec-desktop-session-logs.md` |
| REQ-021 | iMessage History source (chat.db) | [ ] | Metadata-only: conversation counts, contacts, message frequency. Requires FDA. Privacy-sensitive. |
| REQ-022 | Apple Mail metadata source | [ ] | Inbox stats, sender frequency, unread counts via AppleScript. No body content. |
| REQ-023 | Safari History + Reading List source | [ ] | Browsing stats, top domains, reading list count. TCC-protected since Big Sur. |
| REQ-024 | Tier 2 sources: Chrome History, Notion Local Cache, Git Repos | [ ] | High-value data unlock |
| REQ-025 | Tier 3 sources: Arc Browser, Downloads, Reminders, Calendar, Screenshots | [ ] | "Conscious attention" signals |
| REQ-026 | Additional targets: Pipedream (API Key) | [ ] | Mainstream automation platform |
| REQ-027 | Home Assistant target (Webhook ID) | [ ] | Smart home triggers |
| REQ-028 | Custom target (Bearer/Header/Basic auth, manual URL) | [ ] | Any REST endpoint |
| REQ-029 | Event filtering + quiet hours + weekend mode | [ ] | User control over delivery timing |
| REQ-030 | Transform engine | [ ] | Normalize payloads across sources |
| REQ-031 | Performance optimization | [ ] | |
| REQ-032 | Discovery Mode | [ ] | Auto-scan `~/Library/Containers`, `Group Containers`, `Application Support` for SQLite DBs. Suggest sources from ANY installed app (Drafts, Bear, Things 3, IINA, Raycast, etc.). See `docs/research/spec-discovery-mode.md` |
| REQ-033 | Third-party app sources: Drafts, Bear Notes, Things 3 | [ ] | Sandboxed container SQLite access. High-value "trapped data" candidates. |
| REQ-034 | Self-hosted telemetry dogfooding | [ ] | Optional built-in `localpush-telemetry` source that pushes daily heartbeat (app version, enabled sources, delivery stats) to user's own webhook. Opt-in only. |

---
<!-- PHASE_GATE: Do not proceed until Phase 2 exit criteria met -->
---

## Phase 3: Real-Time (LOCKED)

**Status:** BLOCKED
**Prerequisite:** Phase 2 exit criteria achieved
**Exit Criteria:** User demand signal for streaming + mobile alerts

### Requirements

| ID | Requirement | Status | Notes |
|----|-------------|--------|-------|
| REQ-040 | Streaming push resolution (<5s) | [ ] | Live alerts, status changes |
| REQ-041 | Near-real-time resolution (30-60s) | [ ] | Activity tracking, file saves |
| REQ-042 | Hourly resolution | [ ] | Periodic summaries, health metrics |
| REQ-043 | Tier 4 sources: VS Code/Cursor, Terminal history, Homebrew, Docker, Raycast/Alfred | [ ] | Development environment |
| REQ-044 | Tier 5 sources: Contacts, FaceTime/Phone (metadata only) | [ ] | Relationships — privacy-sensitive, explicit opt-in |
| REQ-045 | Tier 6 sources: Apple Music, Apple TV, Apple Books, Siri Analytics | [ ] | Media consumption |
| REQ-046 | Mobile push alerts via ntfy streaming | [ ] | Claude errors → phone notification |

---
<!-- PHASE_GATE: Do not proceed until Phase 3 exit criteria met -->
---

## Phase 4: Intelligence (LOCKED)

**Status:** BLOCKED
**Prerequisite:** Phase 3 mature + Apple Intelligence/Ollama viable
**Exit Criteria:** Functional local AI triage for at least one source

### Requirements

| ID | Requirement | Status | Notes |
|----|-------------|--------|-------|
| REQ-040 | Local AI privacy guardian | [ ] | Apple Intelligence / Ollama / MLX |
| REQ-041 | Intelligent data classification | [ ] | Understand what data *means*, not just what it *contains* |
| REQ-042 | Anomaly detection before transmission | [ ] | Flag unusual patterns, suggest rules |
| REQ-043 | Plain-language explanations | [ ] | "Banking and medical sites detected — exclude?" |
| REQ-044 | Auto-rule suggestions from observed patterns | [ ] | Learn user preferences over time |

---

## Phase ∞: Sustain

**Status:** FUTURE
**Trigger:** Business decision after active phases

### Maintenance Scope

- Security updates
- Dependency updates
- Bug fixes
- Minor enhancements
- Windows/Linux support (if demand warrants)

---

## Reference: Source Tiers

| Tier | Theme | Sources | Phase |
|------|-------|---------|-------|
| **1: MVP** | Core validation | Claude Code Stats, Claude Sessions, Apple Podcasts, Apple Finance | 1 |
| **2: High-Value** | Broad data unlock | Chrome/Safari History, Screen Time, Notion Local Cache, Git Repos | 2 |
| **3: Engagement** | "Conscious attention" signals | Arc Browser, Safari Reading List, Downloads, Apple Notes, Reminders, Calendar, Screenshots | 2 |
| **4: Development** | Dev environment | VS Code/Cursor, Terminal history, Homebrew, Docker, Raycast/Alfred | 3 |
| **5: Relationships** | People (metadata only, opt-in) | Contacts, Mail, Messages, FaceTime/Phone logs | 3 |
| **6: Media** | Consumption | Apple Music, Apple TV, Apple Books, Siri Analytics | 3 |

---

## Reference: Target Matrix

| Platform | Auth | Discovery | Phase | Status |
|----------|------|-----------|-------|--------|
| **n8n** (172k stars) | API Key | List workflows with webhooks | 1 | Done |
| **ntfy** | Topic + Token | Manual topic | 1 | Done |
| **Make** | API Token | Discover scenarios | 2 | Planned |
| **Zapier** | OAuth2 | Find Zaps with webhook triggers | 2 | Planned |
| **Google Sheets** | OAuth2 | Pick spreadsheet, auto-create worksheets | 1 | Planned |
| **Pipedream** | API Key | List HTTP triggers | 2 | Planned |
| **Home Assistant** | Webhook ID | Manual or Nabu Casa | 2 | Planned |
| **Custom** | Bearer/Header/Basic | Manual URL | 2 | Planned |

---

## Reference: Push Resolution Tiers

| Resolution | Latency | Use Case | Example | Phase |
|------------|---------|----------|---------|-------|
| **Streaming** | <5s | Live alerts, status changes | Claude session errors → ntfy push | 3 |
| **Near-real-time** | 30-60s | Activity tracking | File saves, command execution | 3 |
| **Hourly** | 1 hour | Periodic summaries | Aggregated stats, health metrics | 3 |
| **Daily** | End of day | Historical metrics | Token totals, listening history | 2 |
| **Weekly** | End of week | Trend data | Cost reports, weekly summaries | 2 |

---

## Reference: Key Use Cases

| Use Case | Flow | Phase |
|----------|------|-------|
| **AI Cost Tracking** | Claude Stats → n8n → Dashboard | 1 |
| **Mobile Alerts** | Claude Sessions → ntfy → Phone | 3 |
| **Podcast Analytics** | Apple Podcasts → Zapier → Notion | 2 |
| **Personal Finance** | Apple Finance → Make → Budget | 2 |
| **Digital Wellness** | Screen Time → Home Assistant | 2 |
| **Activity Recall** | All Sources → Aston Recall ("What did I work on Tuesday?") | 3+ |

---

## Agentic Instructions

```
PHASE CHECK PROTOCOL:

Before starting work on any requirement:
1. Check current phase status (IN_PROGRESS vs BLOCKED)
2. Verify requirement belongs to active phase
3. If requirement is in LOCKED phase → STOP and flag

If working on Phase 2+ requirement while Phase 1 incomplete:
→ WARNING: "This requirement belongs to Phase [N] which is LOCKED"
→ ASK: "Phase 1 exit criteria not met. Continue anyway?"

Phase transition:
→ Only human can unlock next phase
→ Update status: IN_PROGRESS → COMPLETE
→ Update next phase: BLOCKED → IN_PROGRESS
```

---

## Version History

| Version | Date | Change |
|---------|------|--------|
| 0.1 | 2026-02-04 | Initial roadmap |
| 0.2 | 2026-02-08 | Updated with factory adoption, v0.2 status |
| 0.3 | 2026-02-09 | Enriched from vision document: source tiers, target matrix, push resolution, phases 3-4, use cases |
| 0.4 | 2026-02-10 | Added REQ-011–014, pulled Make/Zapier into Phase 1 |
| 0.5 | 2026-02-11 | Added REQ-015: delivery failure visibility & recovery (triggered by silent 00:01 DLQ incident) |
| 0.6 | 2026-02-11 | Added REQ-016 (Google Sheets target), REQ-017 (Apple Developer ID signing) |
