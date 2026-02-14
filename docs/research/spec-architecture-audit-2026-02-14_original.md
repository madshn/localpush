# macOS LocalPush Architecture Analysis and Improvement Portfolio

## Summary

This repo has a solid foundation (clear Rust trait boundaries, strong Rust test coverage, CI on macOS), but there are several correctness, privacy, and contract-consistency issues that should be addressed before scaling features.

Validation run in this analysis:

- `npm test -- --run`: 30/30 tests passed, but with React `act(...)` warnings and hidden runtime errors in tests.
- `Cargo.toml`: 161/161 tests passed (unit + integration).

## Bulk-Approval Categories

### 1) Critical Correctness and Data-Loss Prevention (Approve Now)

**Fix DLQ replay command mismatch.**
Evidence: `useDlqActions.ts` (line 20) calls `replay_delivery` with `entryId`, but entry-based replay command is `replay_delivery_by_id` at `mod.rs` (line 1234) while `replay_delivery` expects payload inputs at `mod.rs` (line 954).

**Fix DLQ dismiss behavior (currently likely non-functional).**
Evidence: `dismiss_dlq_entry` calls `mark_delivered` (`mod.rs` (line 1223)), but `mark_delivered` updates only `status='in_flight'` rows (`ledger.rs` (line 231)).

**Remove LIMIT 100 lookup fragility for entry-id operations.**
Evidence: `get_by_status` hard-caps results (`ledger.rs` (line 299)), while lookup-by-id logic scans status lists (`mod.rs` (line 1111), `mod.rs` (line 1211), `mod.rs` (line 1243)).

**Ensure scheduled deliveries honor property filtering/privacy toggles.**
Evidence: scheduler directly parses raw source payload (`scheduled_worker.rs` (line 145)) instead of using filtered path (`parse_and_filter` exists in `source_manager.rs` (line 229)).

**Fix file-event routing for directory-backed sources (Claude Sessions likely unreliable).**
Evidence: exact path map lookup (`source_manager.rs` (line 190)) + non-recursive watch (`file_watcher.rs` (line 75)) + directory watch source (`claude_sessions.rs` (line 511)).

**Prevent silent drop of targeted deliveries when binding is missing.**
Evidence: missing targeted binding returns empty target list (`delivery_worker.rs` (line 96)), and empty target list is marked delivered (`delivery_worker.rs` (line 196)).

### 2) IPC/Data Contract Consistency (Approve Now)

**Unify delivery status casing and DTO shape across frontend.**
Evidence: backend emits snake_case (`mod.rs` (line 33)), but one hook expects camelCase (`useDeliveryStatus.ts` (line 7)).

**Unify delivery queue DTO shape across hooks.**
Evidence: `useDeliveryQueue` expects camelCase fields (`useDeliveryQueue.ts` (line 7)), while backend returns snake_case (`mod.rs` (line 51)) and `useActivityLog` already consumes snake_case (`useActivityLog.ts` (line 7)).

**Normalize source ID conventions (hyphen vs underscore drift).**
Evidence: canonical source IDs are hyphenated (`claude_stats.rs` (line 272), `apple_notes.rs` (line 124)), but frontend maps underscore IDs (`SourceCard.tsx` (line 5), `useActivityLog.ts` (line 42)).

**Fix target type derivation in dashboard cards.**
Evidence: parsing target type via underscore split (`DashboardPipelineRow.tsx` (line 99)) conflicts with hyphenated target IDs generated in backend (`mod.rs` (line 408)).

### 3) Privacy and Security Hardening (Security/Product Review, then Approve)

**Revisit default outbound webhook + auto-enabled source on first launch.**
Evidence: default webhook is set automatically (`state.rs` (line 47)) and first launch auto-enables Claude stats (`state.rs` (line 290)), while worker falls back to legacy webhook when no bindings exist (`delivery_worker.rs` (line 138)).

**Remove Google OAuth client secret from frontend bundle path.**
Evidence: frontend reads and uses `VITE_GOOGLE_CLIENT_SECRET` (`GoogleSheetsConnect.tsx` (line 9)).

**Stop logging sensitive binding values and lower production log verbosity.**
Evidence: full params logging includes `authHeaderValue` (`useBindings.ts` (line 59), `useBindings.ts` (line 81)), logger defaults to debug (`logger.ts` (line 10)).

**Align Apple Photos behavior with stated data minimization policy.**
Evidence: comment claims "counts only" and no details (`apple_photos.rs` (line 50)), but payload includes recent-photo metadata including filename/geo/faces (`apple_photos.rs` (line 418), `apple_photos.rs` (line 23)).

### 4) Architecture and Maintainability (Approve, medium effort)

**Split monolithic command surface by domain.**
Evidence: `mod.rs` is 1417 lines.

**Introduce a typed IPC boundary (single contract package for Rust <-> TS).**
Rationale: current direct invoke usage is widespread and drift-prone.

**Refactor startup orchestration into dedicated components.**
Evidence: `AppState::new_production` is broad and mixes config migration, credential restore, target/source registry, and defaults (`state.rs` (line 30) onward).

**Make source property definitions and payload keys schema-aligned.**
Evidence: declared properties don't match emitted top-level payload keys in Claude sources (`claude_stats.rs` (line 458), `claude_stats.rs` (line 341); `claude_sessions.rs` (line 611), `claude_sessions.rs` (line 547)).

### 5) Test and Observability Hardening (Approve Now)

**Fix frontend test quality issues currently masked by passing status.**
Evidence: `act(...)` warnings in FlowModal tests (`FlowModal.test.tsx` (line 63)) and runtime errors during ActivityCard tests (`ActivityCard.test.tsx` (line 46) with hook usage at `useErrorDiagnosis.ts` (line 35)).

**Add integration tests for DLQ replay/dismiss and entry lookup behavior beyond 100 records.**

**Add contract tests to enforce snake_case/camelCase mapping decisions.**

**Emit DLQ event or remove dead listener path.**
Evidence: frontend listens for `delivery:dlq` (`App.tsx` (line 62)), but backend does not emit this event.

**Correct misleading Google Sheets append status reporting.**
Evidence: hardcoded 400 in error message (`google_sheets.rs` (line 289), `google_sheets.rs` (line 336)).

## Public API / Interface Changes (Proposed)

- Add ledger interface: `get_by_id(entry_id)` and explicit DLQ state transition method (`dismiss_dlq_entry` should not rely on `mark_delivered` semantics).
- Deprecate ambiguous replay paths by making entry-based replay first-class (frontend always calls `replay_delivery_by_id` for activity items).
- Standardize IPC DTO casing (recommended: snake_case over wire, mapped once in a shared TS adapter).
- Define canonical source ID format (kebab-case) and canonical payload `source_id` field.
- Add an internal event contract for delivery lifecycle notifications (`delivery:dlq`, maybe `delivery:recovered`).

## Test Cases and Acceptance Criteria

1. DLQ replay from failed-entry card succeeds and enqueues a new pending entry by entry_id.
2. DLQ dismiss changes state deterministically and does not rely on prior `in_flight` state.
3. Lookup by old entry_id works with >100 entries in each status bucket.
4. Scheduled deliveries respect source property toggles identically to manual/file-triggered deliveries.
5. Claude Sessions triggers from nested file changes under `~/.claude/projects/**`.
6. No targeted event is silently marked delivered when endpoint binding is missing.
7. Frontend contract tests fail if backend DTO casing drifts.
8. No auth secrets appear in frontend logs under normal and error paths.
9. Google Sheets error messages report real HTTP status.
10. Frontend tests pass without React `act(...)` warnings or hidden runtime errors.

## Assumptions and Defaults Used

- Canonical source/target IDs should be stable machine IDs (kebab-case), while human labels are derived separately.
- Privacy defaults should prefer least data collection and explicit user opt-in, especially for Apple Photos and first-launch behavior.
- Legacy webhook fallback should be treated as migration-only behavior and gated explicitly.
- Existing Rust test strategy is strong and should remain the backbone; frontend needs stronger integration/contract coverage.
