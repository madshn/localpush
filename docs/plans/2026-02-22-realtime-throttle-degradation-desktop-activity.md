# Plan: Real-Time Throttle, Target Degradation, Desktop Activity Source

**Date:** 2026-02-22
**Status:** Approved, pending implementation
**Branch:** To be created on pickup

---

## Context

LocalPush (pid 26805) is consuming ~23% CPU. Root cause: `claude-sessions` watches `~/.claude/projects/` recursively, firing a full parse+enqueue on **every file change** (~1 every 5-6s during active coding). Compounded by expired Google Sheets OAuth token, all deliveries fail and enter a tight retry loop (50+ entries cycling every 5s). Three features address this:

1. **90-second coalescing** — buffer file events, then deliver staggered across targets
2. **Graceful target degradation** — pause broken targets, queue messages, show reconnect CTA
3. **Desktop activity source** — track active computer sessions via IOKit HIDIdleTime

---

## Feature 1: Real-Time Coalescing + Staggered Delivery

### What changes
Instead of parsing+enqueuing on every file event, buffer events for 90 seconds per source, then parse once and enqueue with staggered `available_at` offsets so targets are hit one at a time (10s apart).

### Files to modify

| File | Change |
|------|--------|
| `src-tauri/src/source_manager.rs` | Add coalescing state + timer per source. Replace immediate enqueue in `handle_file_event()` with timer-based coalescing. Add `flush_source()` method. |
| `src-tauri/src/lib.rs` | Wire coalescing flush after SourceManager setup. No structural changes. |
| `src-tauri/src/ledger.rs` | Add `enqueue_targeted_at()` method that accepts a custom `available_at` timestamp. |
| `src-tauri/src/traits/delivery_ledger.rs` | Add `enqueue_targeted_at()` to trait. |

### Design

**Coalescing in SourceManager:**
- New field: `coalesce_timers: Mutex<HashMap<String, JoinHandle<()>>>` — one timer per source
- `handle_file_event()` becomes: cancel any existing timer for this source, start a new 90s timer
- When timer fires: parse source once, resolve all on_change bindings, call `enqueue_targeted_at()` with staggered offsets
- SourceManager needs `Arc<BindingStore>` added as a field (currently only has ledger)

**Staggered available_at:**
- For N target bindings, set `available_at = now + (i * 10)` seconds for the i-th target
- `claim_batch()` already filters `WHERE available_at <= ?1` — staggered entries become eligible 10s apart naturally
- Delivery worker unchanged — it already respects `available_at`

**Backward compatibility:**
- Manual "Push Now" bypasses coalescing (calls `parse_and_filter()` directly — no change needed)
- Scheduled worker bypasses coalescing (uses its own timer — no change needed)
- Tests that call `handle_file_event()` directly still work — they just trigger a 90s timer

### Implementation sequence

1. Add `enqueue_targeted_at(event_type, payload, endpoint_id, available_at)` to `DeliveryLedgerTrait` + `DeliveryLedger`
2. Add `binding_store: Arc<BindingStore>` field to `SourceManager`
3. Add coalescing state to `SourceManager` (`coalesce_timers` HashMap)
4. Modify `handle_file_event()` to start/reset 90s timer instead of immediate enqueue
5. Implement `flush_source()` that parses, resolves bindings, enqueues with staggered offsets
6. Wire `binding_store` into SourceManager construction in `lib.rs`/`state.rs`
7. Unit tests for coalescing timer behavior and staggered timestamps

---

## Feature 2: Graceful Target Degradation

### What changes
When a target has delivery issues (expired token, auth failure, connection failure), the system pauses deliveries to that target and queues messages instead of retrying endlessly. The UI shows a warning with queued message count and a reconnect CTA. On reconnect, queued messages replay.

### Files to create

| File | Purpose |
|------|---------|
| `src-tauri/src/target_health.rs` | `TargetHealthTracker` — per-target health state machine |

### Files to modify

| File | Change |
|------|--------|
| `src-tauri/src/delivery_worker.rs` | Check target health before delivery. On failure, report to health tracker. Skip deliveries to degraded targets (mark as `target_paused`). |
| `src-tauri/src/ledger.rs` | Add `target_paused` status. Add `pause_target_deliveries()` and `resume_target_deliveries()` methods. Add `count_paused_for_target()`. |
| `src-tauri/src/traits/delivery_ledger.rs` | Add new methods to trait. Add `TargetPaused` to `DeliveryStatus` enum. |
| `src-tauri/src/target_manager.rs` | Add `health: Arc<TargetHealthTracker>` field. Expose health queries. |
| `src-tauri/src/commands/mod.rs` | Add `get_target_health`, `reconnect_target` commands. |
| `src-tauri/src/lib.rs` | Wire TargetHealthTracker into AppState and delivery worker. |
| `src/api/hooks/useTargets.ts` | Add `useTargetHealth()` hook. |
| `src/components/PipelineCard.tsx` | Show degraded target warning with queued count + reconnect CTA. |

### Design

**TargetHealthTracker (`target_health.rs`):**
```
State machine per target_id:
  Healthy → Degraded (on auth/token error: immediate; on transient error: after 3 consecutive failures)
  Degraded → Healthy (on successful reconnect_target command)
```

- `HashMap<String, TargetHealthState>` behind `Mutex`
- `report_failure(target_id, error)` — classifies error, transitions state
- `report_success(target_id)` — resets consecutive failure count
- `is_degraded(target_id) -> Option<DegradationInfo>` — returns reason + timestamp
- Auth errors (`TokenExpired`, `AuthFailed`) → immediate degradation
- Transient errors (`ConnectionFailed`, `DeliveryError`) → degrade after 3 consecutive failures
- `mark_reconnected(target_id)` — resets to Healthy

**Ledger changes:**
- New status value `target_paused` in `DeliveryStatus` enum
- `pause_target_deliveries(target_id)` — UPDATE all pending entries for this target to `target_paused`
- `resume_target_deliveries(target_id)` — UPDATE all `target_paused` entries for this target back to `pending`
- `count_paused_for_target(target_id) -> usize` — COUNT for UI display

**Delivery worker integration:**
- Before delivering to a target, check `health_tracker.is_degraded(target_id)`
- If degraded: mark entry as `target_paused`, skip delivery, continue
- On delivery failure: call `health_tracker.report_failure(target_id, error)`
- On delivery success: call `health_tracker.report_success(target_id)`

**Frontend:**
- `useTargetHealth()` hook polls `get_target_health` command
- PipelineCard shows warning row when any bound target is degraded:
  - Target icon (Google for GSheets, ntfy icon, etc.) + "Reconnect" button
  - "{N} deliveries queued" count
- Reconnect button calls `reconnect_target` command → triggers `test_connection()` → if OK, resumes queued deliveries

### Implementation sequence

1. Add `TargetPaused` to `DeliveryStatus` enum in traits
2. Create `target_health.rs` with `TargetHealthTracker`
3. Add ledger methods: `pause_target_deliveries`, `resume_target_deliveries`, `count_paused_for_target`
4. Integrate health tracking into delivery worker (`process_batch`)
5. Add `get_target_health` and `reconnect_target` Tauri commands
6. Wire health tracker into AppState and delivery worker spawn
7. Add `useTargetHealth()` frontend hook
8. Update PipelineCard with degraded target warning + reconnect CTA
9. Tests for state machine transitions, pause/resume ledger operations

---

## Feature 3: Desktop Activity Source

### What changes
New source that tracks active desktop sessions (keyboard/mouse activity). Uses macOS IOKit `HIDIdleTime` — no Accessibility permissions needed. Sessions end after 3 minutes of inactivity or on sleep/lockscreen.

### Files to create

| File | Purpose |
|------|---------|
| `src-tauri/src/sources/desktop_activity.rs` | Source implementation + session state machine |
| `src-tauri/src/desktop_activity_worker.rs` | Background polling loop (30s interval) |
| `src-tauri/src/iokit_idle.rs` | FFI wrapper for IOKit HIDIdleTime |

### Files to modify

| File | Change |
|------|--------|
| `src-tauri/src/state.rs` | Register desktop-activity source, spawn worker |
| `src-tauri/src/lib.rs` | Spawn desktop activity worker alongside delivery/scheduled workers |
| `src-tauri/src/sources/mod.rs` | Add pub export for desktop_activity |
| `src-tauri/Cargo.toml` | Add `core-foundation-sys` dep, link IOKit framework |
| `src-tauri/build.rs` | Add `println!("cargo:rustc-link-lib=framework=IOKit")` |

### Design

**IOKit FFI (`iokit_idle.rs`):**
```rust
// Uses IOServiceGetMatchingService + IORegistryEntryCreateCFProperty
// Property: "HIDIdleTime" from IOHIDSystem (nanoseconds since last input)
pub fn get_idle_seconds() -> Result<f64, String>
```
- No permissions needed — HIDIdleTime is a system property
- Returns seconds since last keyboard/mouse input
- Uses `core-foundation-sys` crate for CF type handling

**Source implementation (`desktop_activity.rs`):**
- Source trait: `id() = "desktop-activity"`, `name() = "Desktop Activity"`, `watch_path() = None` (first non-file source)
- Internal `SessionStore` persisted in AppConfig (JSON blob):
  ```json
  { "current_session": { "start": 1708000000, "last_active": 1708003600 },
    "completed_sessions": [...] }
  ```
- `parse()` returns completed sessions as payload
- `preview()` returns current session + recent completed sessions

**State machine:**
```
Inactive → Active (idle < 180s, i.e., user is active)
Active → Idle (idle >= 180s, 3 minutes inactivity)
Idle → finalize session → Inactive
Inactive → Active (new session when activity resumes)
```

**Worker (`desktop_activity_worker.rs`):**
- Polls every 30 seconds via `tokio::time::interval`
- Reads `get_idle_seconds()`
- Updates state machine
- On session end: stores completed session, enqueues to ledger if source is enabled + has bindings
- Also detects sleep/wake via `NSWorkspace` notifications (optional enhancement)

**Payload format:**
```json
{
  "type": "desktop_session",
  "start_timestamp": 1708000000,
  "end_timestamp": 1708003600,
  "duration_minutes": 60,
  "idle_threshold_seconds": 180
}
```

### Implementation sequence

1. Add `core-foundation-sys` to Cargo.toml, IOKit framework link in build.rs
2. Create `iokit_idle.rs` — FFI wrapper for HIDIdleTime
3. Create `desktop_activity.rs` — Source trait impl + SessionStore + state machine
4. Create `desktop_activity_worker.rs` — polling loop
5. Register source in `state.rs`, spawn worker in `lib.rs`
6. Add pub export in `sources/mod.rs`
7. Test: verify IOKit returns reasonable idle time, test state machine transitions

---

## Implementation Order

Execute features in this order (each builds on the previous):

1. **Feature 2 (Target Degradation)** — immediately stops the CPU burn by pausing the broken Google Sheets target
2. **Feature 1 (Coalescing)** — prevents future burst storms; depends on stable delivery pipeline
3. **Feature 3 (Desktop Activity)** — independent new source; can be done last

## Verification

### Backend
```bash
cd src-tauri
cargo test                    # All tests pass
cargo clippy -- -D warnings   # No warnings
```

### Frontend
```bash
npm run lint
npm run typecheck
npm test
```

### Manual smoke test
```bash
npx tauri dev
```
1. Verify degraded Google Sheets target shows warning + queued count in pipeline
2. Click "Reconnect" after re-authenticating — verify queued deliveries replay
3. Enable claude-sessions source — verify events coalesce (90s buffer visible in logs)
4. Verify staggered delivery (10s apart in delivery worker logs)
5. Enable desktop-activity source — verify sessions appear after keyboard activity + 3min idle
