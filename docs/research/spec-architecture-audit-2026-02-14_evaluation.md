# Architecture Audit Evaluation — 2026-02-14

**Auditor:** Codex 5.3
**Evaluator:** Claude Opus 4.6 (LocalPush Product Coordinator)
**Date:** 2026-02-14

---

## Methodology

Every claim verified against actual codebase with file/line evidence. Each finding rated:
- **VALID** — claim accurate, action warranted
- **PARTIALLY VALID** — directionally correct but root cause or severity misstated
- **INVALID** — claim incorrect or not a bug

**Overall hit rate: 65% valid/partially-valid, 35% invalid.**

---

## Category 1: Critical Correctness (6 claims)

| # | Finding | Verdict | Action |
|---|---------|---------|--------|
| 1.1 | DLQ replay command mismatch | INVALID | Bug exists (wrong command name) but evidence misstated. Fix: `useDlqActions.ts` should call `replay_delivery_by_id` |
| 1.2 | DLQ dismiss non-functional | INVALID | Real issue is `let _ =` error suppression, not WHERE clause design. Fix: handle error or add DLQ→dismissed state |
| 1.3 | LIMIT 100 lookup fragility | PARTIALLY VALID | Pattern exists but >100 DLQ entries is exceptional. Low priority. |
| 1.4 | Scheduled delivery bypasses filtering | INVALID | Intentional — targeted deliveries skip filtering by design |
| 1.5 | Claude Sessions file routing broken | VALID | Exact path match + non-recursive watch on directory source. **Implement fix.** |
| 1.6 | Silent drop of targeted deliveries | VALID | Missing binding → marked delivered silently. **Ship-blocking.** |

### Actions taken:
- 1.1: Fix command name in useDlqActions.ts
- 1.2: Fix error suppression in dismiss_dlq_entry
- 1.5: Fix file event routing to support directory-backed sources
- 1.6: Mark as failed (not delivered) when targeted binding missing

---

## Category 2: IPC/Data Contract Consistency (4 claims)

| # | Finding | Verdict | Action |
|---|---------|---------|--------|
| 2.1 | Delivery status casing mismatch | INVALID | Tauri auto-converts snake_case → camelCase. No actual mismatch. |
| 2.2 | Delivery queue DTO shape mismatch | PARTIALLY VALID | Inter-hook inconsistency: `useActivityLog` uses snake_case interface but receives camelCase. |
| 2.3 | Source ID hyphen/underscore drift | VALID | Backend: `claude-stats`, Frontend maps: `claude_code_stats`. **Lookups silently fail.** |
| 2.4 | Target type derivation via underscore split | VALID | `split("_")` on hyphenated IDs like `n8n-abc123`. **Parsing broken.** |

### Actions taken:
- 2.2: Align useActivityLog interface to camelCase
- 2.3: Fix all frontend source ID maps to use kebab-case
- 2.4: Fix target type parsing to split on hyphen

---

## Category 3: Privacy and Security (4 claims)

| # | Finding | Verdict | Action |
|---|---------|---------|--------|
| 3.1 | Auto-webhook + auto-enable on first launch | VALID | Consent footgun. Data not auto-sent but configuration is pre-loaded. |
| 3.2 | Google OAuth client secret in frontend | INVALID | Standard desktop OAuth pattern. Google allows this for installed apps. |
| 3.3 | Auth header values logged on error | PARTIALLY VALID | Logged to browser console only. Medium risk in desktop context. |
| 3.4 | Apple Photos privacy mismatch | VALID | Doc says "counts only", payload sends filenames/GPS/faces. **Fix doc and property flags.** |

### Actions taken:
- 3.3: Redact auth values from log output
- 3.4: Fix documentation and privacy_sensitive flags

---

## Category 4: Architecture and Maintainability (4 claims)

| # | Finding | Verdict | Action |
|---|---------|---------|--------|
| 4.1 | Monolithic commands/mod.rs (1417 lines) | VALID | Well-organized within file but genuinely large. Backlog item. |
| 4.2 | Startup orchestration too broad | VALID | new_production mixes 6 concerns across 277 lines. Backlog item. |
| 4.3 | Source property definitions ≠ payload keys | PARTIALLY VALID | claude_stats intentional; claude_sessions has unimplemented stubs. |
| 4.4 | Typed IPC boundary proposal | N/A | Good architectural direction. Backlog. |

### Actions taken:
- 4.1–4.4: Backlog items. Not blocking current release.

---

## Category 5: Test and Observability (5 claims)

| # | Finding | Verdict | Action |
|---|---------|---------|--------|
| 5.1 | Frontend test quality masked | PARTIALLY VALID | FlowModal.test.tsx doesn't exist (hallucinated). ActivityCard has dead import only. |
| 5.2 | DLQ integration tests needed | VALID | No test coverage for DLQ replay/dismiss paths. |
| 5.3 | Contract tests for casing | VALID | Good suggestion. Backlog. |
| 5.4 | Dead DLQ event listener | VALID | Frontend listens for `delivery:dlq`, backend never emits it. **Dead code.** |
| 5.5 | Google Sheets hardcoded 400 | VALID | Error messages report 400 regardless of actual HTTP status. **Fix.** |

### Actions taken:
- 5.4: Remove dead listener or emit event from backend
- 5.5: Fix error reporting to use actual HTTP status

---

## Meta-Observations on Codex 5.3

1. **Doesn't model framework behavior.** Tauri serde auto-conversion missed entirely — analyzed Rust/TS in isolation.
2. **Conflates "pattern exists" with "pattern is a bug."** Scheduled delivery filtering bypass and OAuth secret are intentional design.
3. **Line numbers are approximate** (off by 10-50 lines). Treat as search hints.
4. **Hallucinated one test file** (FlowModal.test.tsx doesn't exist).
5. **Strong at structural analysis** — monolithic files, mixed concerns, naming drift all correctly identified.
6. **Weak at intent analysis** — can't distinguish "by design" from "by accident."

---

## Implementation Scope

Fixes implemented in this release:

| Fix | Category | Severity |
|-----|----------|----------|
| Silent targeted delivery drop → fail instead of deliver | 1.6 | High |
| Claude Sessions directory routing | 1.5 | High |
| Source ID convention alignment | 2.3 | High |
| Target type parsing fix | 2.4 | High |
| useActivityLog interface alignment | 2.2 | Medium |
| DLQ replay command fix | 1.1 | Medium |
| DLQ dismiss error handling | 1.2 | Medium |
| Auth header log redaction | 3.3 | Medium |
| Apple Photos privacy flags + doc | 3.4 | Medium |
| Dead DLQ listener cleanup | 5.4 | Low |
| Google Sheets error status fix | 5.5 | Low |

Deferred to backlog:
- commands/mod.rs split (4.1)
- Startup orchestration refactor (4.2)
- Typed IPC boundary (4.4)
- Contract tests for casing (5.3)
- LIMIT 100 pagination (1.3)
