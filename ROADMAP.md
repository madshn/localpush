# Roadmap: LocalPush

**Created:** 2026-02-04
**Current Phase:** 1
**Status:** IN_PROGRESS

---

## Phase Model

| Phase | Goal | Gate |
|-------|------|------|
| 1: Validation | Proof of life | External signal: installs, active sources, webhook deliveries |
| 2: Growth | Scale + features | Business decision |
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
| REQ-010 | Proof-of-life instrumentation | [ ] | Analytics for installs, source activations |
| REQ-011 | Scheduled push cadence | [x] | Per-binding on_change/daily/weekly delivery modes |
| REQ-012 | BUG: Apple Photos source broken | [ ] | Source does not work — needs investigation and fix |
| REQ-013 | Dashboard kanban view | [ ] | 3-column kanban layout (see Stitch prototype) |
| REQ-014 | Make.com + Zapier connectors | [ ] | Same pattern as n8n connector (endpoint discovery) |

### Scope Boundaries

**In Scope:**
- Core delivery pipeline (sources → targets)
- macOS menu bar app
- n8n + ntfy targets
- Claude Code + Apple sources
- Homebrew distribution

**Out of Scope (Phase 2+):**
- Windows/Linux support
- Home Assistant target
- Local AI privacy guardian
- Streaming push resolution (<5s)

### Work Log

| Date | What | Outcome |
|------|------|---------|
| 2026-02-04 | Initial scaffold | v0.1 created |
| 2026-02-05 | v0.1.0-v0.1.3 releases | Menu bar app, delivery pipeline |
| 2026-02-06 | v0.2 multi-source architecture | Targets, bindings, 5 sources |
| 2026-02-08 | E2E verification | Real data flowing to n8n |
| 2026-02-08 | Bob factory adoption | Factory standards applied |
| 2026-02-10 | UX overhaul + scheduled push cadence | Tailwind v4, Radix UI, pipeline cards, per-binding delivery modes |

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
| REQ-020 | Push resolution options | [ ] | Streaming, near-real-time |
| REQ-021 | Windows/Linux support | [ ] | |
| REQ-022 | Local AI privacy guardian | [ ] | Apple Intelligence/Ollama |
| REQ-023 | Performance optimization | [ ] | |

---

## Phase ∞: Sustain

**Status:** FUTURE
**Trigger:** Business decision after Phase 2

### Maintenance Scope

- Security updates
- Dependency updates
- Bug fixes
- Minor enhancements

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
| 0.3 | 2026-02-10 | Added REQ-011–014, pulled Make/Zapier into Phase 1 |
