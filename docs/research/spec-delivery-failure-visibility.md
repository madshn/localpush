# Spec: Delivery Failure Visibility & Recovery Guidance

**Created:** 2026-02-11
**Status:** Proposed
**Type:** Core UX Feature
**Priority:** Critical — directly impacts guaranteed delivery promise
**Trigger:** Daily digest push to Metrick fired at 00:01, got 403, exhausted 5 retries, landed in DLQ. User didn't notice until manually querying SQLite 7 hours later.

---

## Problem

LocalPush's core promise is **guaranteed delivery with radical transparency**. Today, neither holds when things go wrong:

1. **Failures are silent.** A delivery can exhaust all retries and land in the dead letter queue without the user ever knowing. The tray icon doesn't change. No notification fires. The Activity tab shows it, but only if you happen to open the app and scroll.

2. **Errors are opaque.** The current error display is `HTTP error: 403` — a raw technical string. The user has no idea *why* it failed, *what to do*, or *what data was lost*.

3. **Data timeline gaps are invisible.** When a scheduled daily push fails, the downstream system (e.g., Metrick KPI dashboard) has a hole in its timeline. The user has no way to know this hole exists, what date range it covers, or how to fill it.

---

## Incident That Exposed This

```
Source:    claude-stats (daily digest)
Binding:   → Metrick KPI ingest webhook
Schedule:  daily @ 00:01
Trigger:   2026-02-11 00:01:40
Result:    HTTP 403, 5/5 retries exhausted → DLQ
Root cause: Empty Authorization header (credential not stored)
Discovery: Manual SQLite query at ~07:00
```

The payload contained 14 days of daily breakdowns. It's still sitting in the ledger and can be replayed — but nothing in the app told the user to do so.

---

## Design Principles

From the UX Constitution:

| Principle | Application Here |
|-----------|-----------------|
| "Is my data flowing?" | Failures must break the green → red signal chain immediately |
| Colors tell the story | Red tray icon, red banner in popup, red card in activity |
| Friendly helper tone | "Your daily stats didn't reach Metrick" not "HTTP 403 Forbidden" |
| Zero cognitive load (popup) | Popup shows *that* something failed + link to investigate |
| Managed complexity (full window) | Full window explains *why*, *what's at stake*, and *how to fix* |

---

## Feature: Failure Alert System

### 1. Tray Icon State Change

When any delivery enters `failed` or `dlq` status:

- Tray icon turns **red** (or shows red badge dot)
- Stays red until all failures are resolved (retried successfully or dismissed)
- Existing states remain: green (healthy), yellow (pending), grey (idle)

### 2. macOS Notification

On DLQ (all retries exhausted), fire a native macOS notification:

```
Title:    LocalPush: Delivery failed
Body:     Your daily Claude Stats didn't reach Metrick KPI Ingest.
          Tap to open LocalPush and fix it.
Action:   Opens full window → Activity tab, scrolled to the failed entry
```

**Rules:**
- Only notify on DLQ, not on individual retry failures (avoid noise)
- One notification per DLQ entry, not per retry
- Respect macOS notification settings (user can mute in System Settings)

### 3. Tray Popup: Failure Banner

When failures exist, the popup shows a degraded state:

```
┌─────────────────────────────────┐
│  ⚠ 1 delivery needs attention  │  ← red/yellow banner
│                                 │
│  ● Claude Stats     ✓ 19:47    │  ← last successful delivery
│    ↳ Daily push failed at 00:01 │  ← inline failure note
│  ● Apple Podcasts   ✓ 17:32    │
│                                 │
│  [ Open LocalPush ]             │
└─────────────────────────────────┘
```

---

## Feature: Error Diagnosis & Guidance

### Intelligent Error Classification

Replace raw HTTP errors with diagnosed, actionable messages. The backend classifies errors into categories and attaches structured metadata.

| HTTP Status | Category | User Message | Guidance |
|-------------|----------|-------------|----------|
| 401 | `auth_invalid` | "Authentication rejected by [target]" | "Check your API key in Settings > Targets > [target]. The current key may have expired or been revoked." |
| 403 | `auth_missing` | "Not authorized to reach [endpoint]" | "This webhook requires authentication. Go to the binding settings and add an API key or auth header." |
| 404 | `endpoint_gone` | "[endpoint] no longer exists" | "The webhook URL may have changed. Check your n8n workflow and update the endpoint in Sources > [source]." |
| 429 | `rate_limited` | "Target is rate-limiting requests" | "Too many requests to [target]. LocalPush will retry with backoff. No action needed unless this persists." |
| 500-599 | `target_error` | "[target] had an internal error" | "The problem is on [target]'s side. LocalPush will retry automatically. If it persists, check [target]'s logs." |
| Connection refused | `unreachable` | "Can't reach [target]" | "Is [target] running? Check the URL in Settings > Targets. LocalPush will keep retrying." |
| Timeout | `timeout` | "[target] didn't respond in time" | "The request took too long. This could be a network issue or [target] is overloaded. Will retry." |
| Empty auth header | `auth_not_configured` | "Authentication not set up for this binding" | "You configured an Authorization header but didn't save a credential. Open the binding config and enter your API key." |

### Error Display in Activity Card (Expanded)

When a failed/DLQ entry is expanded in the Activity tab, show a structured diagnosis instead of a raw error string:

```
┌──────────────────────────────────────────────────┐
│ ☠ Claude Stats · Daily push                      │
│   Gave up after 5 retries                  00:01 │
│                                                  │
│ ▼ Expanded:                                      │
│                                                  │
│   What happened                                  │
│   Not authorized to reach Metrick KPI Ingest.    │
│   The webhook returned 403 Forbidden.            │
│                                                  │
│   What to do                                     │
│   This webhook requires authentication. Open the │
│   binding settings and add your API key.          │
│   [ Go to Binding Settings ]                     │
│                                                  │
│   What's at risk                                 │
│   This was a daily digest covering Feb 10.       │
│   Your Metrick dashboard is missing this day.    │
│   The data is still here — replay it after       │
│   fixing auth to fill the gap.                   │
│                                                  │
│   Timeline                                       │
│   00:01:40  First attempt — 403 Forbidden        │
│   00:01:50  Retry 1/5 — 403 Forbidden            │
│   00:02:10  Retry 2/5 — 403 Forbidden            │
│   00:02:50  Retry 3/5 — 403 Forbidden            │
│   00:04:10  Retry 4/5 — 403 Forbidden            │
│   00:06:50  Retry 5/5 — 403 Forbidden            │
│   00:06:50  Moved to dead letter queue            │
│                                                  │
│   [ Replay ]  [ Retry ]  [ Dismiss ]             │
└──────────────────────────────────────────────────┘
```

---

## Feature: Data Timeline Awareness

### The Core Problem

LocalPush knows things downstream systems don't:
- It knows a daily digest was *supposed* to deliver at 00:01
- It knows the payload covers a specific date range
- It knows the downstream system now has a gap

This knowledge should be surfaced, not buried in a SQLite column.

### Timeline Gap Detection

For scheduled deliveries (`delivery_mode: daily` or `weekly`), track expected vs actual delivery:

```
Expected: 2026-02-11 00:01 → claude-stats → Metrick KPI Ingest
Actual:   DLQ (403)
Gap:      Feb 10 daily breakdown missing from Metrick
```

### Gap Indicator in Pipeline Card

On the Sources tab, the pipeline card for a source with a gap shows:

```
┌──────────────────────────────────────────┐
│ Claude Stats                        ● 🔴 │
│ Last delivered: Feb 10, 19:47 (manual)   │
│                                          │
│ ⚠ Missing: Daily digest for Feb 10      │
│   Failed at 00:01 — auth not configured  │
│   [ Fix & Replay ]                       │
└──────────────────────────────────────────┘
```

### Recovery Actions

| Action | What It Does | When Available |
|--------|-------------|----------------|
| **Replay** | Re-enqueue the original payload as a new delivery | Any failed/DLQ entry with payload |
| **Retry** | Reset the existing entry to pending (same delivery ID) | Failed/DLQ entries only |
| **Fix & Replay** | Deep-link to binding config, then replay after save | Auth/config errors |
| **Dismiss** | Acknowledge the failure, clear the alert state | Any failed/DLQ entry |

### Replay Confirmation

Before replaying, show what will happen:

```
┌───────────────────────────────────────────┐
│  Replay daily digest for Claude Stats?    │
│                                           │
│  This will send the Feb 10 daily          │
│  breakdown to Metrick KPI Ingest.         │
│                                           │
│  Payload: 14 daily entries (Jan 29–Feb 11)│
│  Target:  Metrick KPI Ingest webhook      │
│                                           │
│  ⚠ Make sure authentication is fixed     │
│    before replaying, or it will fail      │
│    again.                                 │
│                                           │
│  [ Cancel ]              [ Replay Now ]   │
└───────────────────────────────────────────┘
```

---

## Backend Changes

### 1. Error Classification (Rust)

Add an `ErrorDiagnosis` struct returned alongside `last_error`:

```rust
struct ErrorDiagnosis {
    category: ErrorCategory,      // auth_missing, endpoint_gone, etc.
    user_message: String,         // "Not authorized to reach Metrick KPI Ingest"
    guidance: String,             // "Open the binding settings and add your API key"
    risk_summary: Option<String>, // "Your Metrick dashboard is missing Feb 10"
    retry_log: Vec<RetryAttempt>, // Timestamped attempt history
}
```

### 2. Retry History

Currently, only `retry_count` and `last_error` are stored. Add a `retry_log` column (JSON array) to track each attempt:

```json
[
  {"at": 1739228500, "error": "HTTP 403", "attempt": 1},
  {"at": 1739228510, "error": "HTTP 403", "attempt": 2}
]
```

### 3. Notification Trigger

When `mark_failed` transitions an entry to DLQ status, emit a Tauri event that the frontend (or a native notification handler) picks up:

```rust
if new_status == DeliveryStatus::Dlq {
    app_handle.emit("delivery:dlq", DlqEvent {
        entry_id, source_id, endpoint_name, error_diagnosis
    });
}
```

### 4. Tray Icon Update

Expose a command or event that the tray icon manager listens to. When any entry is in `failed`/`dlq` state, set tray icon to red variant.

---

## Scope & Phasing

### Phase 1 (Ship Now)

Minimum viable failure visibility. No silent failures.

1. **Tray icon turns red** when DLQ entries exist
2. **macOS notification** on DLQ
3. **Error classification** for the top 5 HTTP status codes (401, 403, 404, 5xx, connection error)
4. **Structured error display** in ActivityCard expanded view (what happened + what to do)
5. **Replay confirmation dialog** with payload summary and pre-flight auth check

### Phase 2 (Fast Follow)

Richer guidance and timeline awareness.

1. **Retry history timeline** in expanded ActivityCard
2. **Timeline gap detection** for scheduled deliveries
3. **Gap indicator on pipeline cards**
4. **Fix & Replay flow** (deep-link to binding config → auto-replay after save)
5. **Tray popup failure banner** (degraded state)

### Phase 3 (Polish)

1. **Pre-flight validation** — before a scheduled push fires, check that auth is configured and target is reachable. Warn proactively instead of failing at 00:01.
2. **Failure digest notification** — batch multiple failures into one notification instead of N
3. **"Dismiss" with reason** — track why user dismissed (fixed, won't fix, not my problem)

---

## Open Questions

1. **Dismiss semantics** — Does dismissing a DLQ entry mean "I've handled this externally" or "I don't care"? Should dismissed entries affect tray icon color?
2. **Notification frequency** — If 3 bindings all fail on the same push, is that 1 notification or 3?
3. **Replay idempotency** — Some downstream systems may not handle duplicate payloads gracefully. Should replay include a `X-LocalPush-Replay: true` header so targets can detect replays?
4. **Auth pre-check depth** — How much validation can we do before firing a scheduled push? A HEAD request to the webhook? Or just check that the credential store has a non-empty value?

---

## Success Criteria

- A user sleeping through a 00:01 failure wakes up to a red tray icon and a notification
- Opening the app shows exactly what failed, why, and how to fix it
- After fixing auth, the user can replay the failed payload and fill the timeline gap
- Total time from "notice failure" to "data timeline restored": under 2 minutes
