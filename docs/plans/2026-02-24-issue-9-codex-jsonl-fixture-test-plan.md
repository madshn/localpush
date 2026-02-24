# Plan: Codex JSONL Real-Fixture Validation (Issue #9)

**Date:** 2026-02-24
**Issue:** #9 (`feat: Codex session JSONL as new data source`)
**Status:** Draft for approval
**Branch:** `codex/issue-9-codex-jsonl-source`

---

## Goal

Implement Codex JSONL sources (`codex-sessions`, `codex-stats`) with a hard validation gate:

- Parser and aggregation tests must use a **frozen fixture derived from real Codex logs for Sunday, 2026-02-23**
- Expected outputs must be verified and committed
- Semantics differences vs Claude sources must be explicitly encoded in schema metadata

This prevents synthetic test fixtures from masking parsing or counting errors.

---

## Why This Matters

Codex JSONL is the authoritative source for token/session usage in this workflow. If parsing logic is wrong, KPI tracking will be wrong. A verified fixture from real logs is the fastest way to catch:

- field shape drift
- token usage field interpretation mistakes
- duplicate event double-counting
- timestamp boundary mistakes (local day vs UTC day)
- missing/partial line behavior

---

## Scope Split (Mirrors Claude Sources)

Two new sources will be implemented, matching the existing Claude pattern:

- `codex-sessions` (log-shaped, text/context heavy)
- `codex-stats` (number-shaped, KPI oriented)

Both sources must be validated against the same frozen `2026-02-23` fixture set.

---

## Real Fixture Requirement (Hard Gate)

Implementation is **not considered done** until all of the following exist and pass:

1. Frozen sanitized fixture derived from real Codex logs on `2026-02-23`
2. Verification manifest with hand-checked totals and semantics notes
3. Golden expected outputs for both `codex-sessions` and `codex-stats`
4. Unit tests asserting parser/aggregation output matches the goldens

---

## Day Boundary Rule (Must Be Explicit)

The fixture and expected outputs must declare which day boundary is used:

- **Local calendar day:** `2026-02-23` in the developer's local timezone (recommended for KPI parity with current UX)
- **OR UTC day:** `2026-02-23T00:00:00Z` to `2026-02-23T23:59:59Z`

The manifest must record:

- `day_boundary.mode`: `"local"` or `"utc"`
- `day_boundary.timezone`: e.g. `"Europe/Oslo"` (if local)
- exact inclusive/exclusive range used

No implicit assumptions.

---

## Proposed Fixture Layout

Store fixtures under `src-tauri` so source tests can load them without external dependencies:

```text
src-tauri/tests/fixtures/codex/
  2026-02-23/
    README.md
    manifest.json
    raw/
      sessions/
        <hashed-session-id-1>.jsonl
        <hashed-session-id-2>.jsonl
        ...
    expected/
      codex-sessions.json
      codex-stats.json
```

### Notes

- `raw/sessions/*.jsonl` are sanitized copies of real Codex JSONL files containing events in scope
- Session filenames may be pseudonymized, but internal linking keys must remain consistent after sanitization
- `expected/*.json` are golden outputs generated once and then reviewed

---

## Sanitization Rules (Preserve Semantics, Remove Sensitive Text)

Sanitization must not alter numeric truth.

### Allowed transformations

- Replace prompt/content text with placeholders
- Replace repo/project paths with stable pseudonyms
- Replace user-specific IDs with deterministic pseudonyms
- Remove unrelated metadata fields not needed for parsing tests

### Forbidden transformations

- Changing timestamps (except if explicitly redacted and re-baselined consistently)
- Changing token counts
- Changing event ordering
- Removing duplicate events that are intentionally part of the fixture
- Changing model names (unless model names are mapped consistently and documented)

### Deterministic pseudonymization

If IDs/paths are redacted, use deterministic mapping to preserve joins/dedup behavior:

- same input -> same output across all files
- mapping strategy documented in `README.md`

---

## Verification Manifest (`manifest.json`)

The manifest is the source of truth for what was verified by hand from real logs.

### Manifest schema (v1)

```json
{
  "manifest_version": 1,
  "source_family": "codex",
  "fixture_date": "2026-02-23",
  "captured_at": "2026-02-24T00:00:00Z",
  "captured_by": "madsnissen",
  "day_boundary": {
    "mode": "local",
    "timezone": "Europe/Oslo",
    "start_inclusive": "2026-02-23T00:00:00+01:00",
    "end_exclusive": "2026-02-24T00:00:00+01:00"
  },
  "sanitization": {
    "text_redacted": true,
    "paths_pseudonymized": true,
    "ids_pseudonymized": false,
    "models_preserved": true,
    "token_counts_preserved": true,
    "timestamps_preserved": true
  },
  "input_files": {
    "session_file_count": 0,
    "jsonl_line_count_total": 0
  },
  "verification": {
    "sessions_in_scope": 0,
    "sessions_out_of_scope_excluded": 0,
    "malformed_lines_skipped": 0,
    "duplicate_events_detected": 0,
    "duplicate_events_collapsed": 0,
    "first_event_timestamp": null,
    "last_event_timestamp": null,
    "models_used": [],
    "token_totals": {
      "input": 0,
      "output": 0,
      "total": 0,
      "cache_read": 0,
      "cache_creation": 0
    },
    "notes": [
      "Describe any Codex-specific semantics differences observed in raw logs"
    ]
  },
  "expected_outputs": {
    "codex_sessions": "expected/codex-sessions.json",
    "codex_stats": "expected/codex-stats.json"
  }
}
```

### Manifest rules

- All counts/totals in `verification` are manually checked against the raw fixture
- If a metric is unavailable in Codex, set to `0` and document under `notes` and source semantics metadata
- `token_totals.total` must equal `input + output` unless a documented exception applies

---

## Golden Outputs (`expected/*.json`)

Two golden files are required.

### 1) `expected/codex-sessions.json`

Purpose: validate log/session parsing and normalized session payload shape.

Requirements:

- Must use the same top-level shape as `claude-sessions` where possible:
  - `source`
  - `timestamp`
  - `sessions`
  - `summary`
- Must include semantics metadata to clarify meaning/resolution differences:
  - `schema_version`
  - `source_family`
  - `source_type` (`"sessions"`)
  - `token_count_basis`
  - `message_count_basis`
  - `duration_basis`
  - `dedupe_basis`
  - `unsupported_metrics`
- Per-session `tokens` object must keep normalized keys:
  - `input`
  - `output`
  - `cache_read`
  - `cache_creation`

### 2) `expected/codex-stats.json`

Purpose: validate KPI aggregation behavior from the same fixture.

Requirements:

- Number-first structure suitable for downstream KPI ingestion
- Explicit aggregation scope/date window in metadata
- Derived/native flags for each major metric group or top-level metadata
- Reuse field names from `claude-stats` only when meanings match

---

## Test Cases (Required)

### A. Real fixture contract tests (must use 2026-02-23 fixture)

1. `codex_sessions_parses_real_fixture_2026_02_23`
- Loads `raw/` fixture files
- Runs `codex-sessions` parser
- Compares normalized output to `expected/codex-sessions.json`

2. `codex_stats_aggregates_real_fixture_2026_02_23`
- Loads same `raw/` fixture files
- Runs `codex-stats` aggregation
- Compares output to `expected/codex-stats.json`

3. `codex_fixture_manifest_verification_totals_match_goldens`
- Loads `manifest.json`
- Verifies:
  - session count matches golden(s)
  - token totals match goldens
  - model list matches goldens
  - time bounds match goldens

4. `codex_sessions_emits_semantics_metadata`
- Asserts required semantics metadata keys exist
- Asserts unsupported/missing metrics are declared explicitly

5. `codex_stats_emits_semantics_and_aggregation_scope`
- Asserts stats payload includes aggregation scope and derivation metadata

### B. Synthetic edge-case tests (still needed, but secondary)

These do **not** replace the real fixture tests.

1. malformed line handling (skip and count)
2. partial/truncated write handling
3. duplicate event dedupe behavior
4. missing token fields
5. mixed-model session
6. no sessions in date window

---

## Receiver Compatibility Test (Recommended)

Add a normalization contract test (backend-only) that loads representative payloads for:

- `claude-sessions`
- `claude-stats`
- `codex-sessions`
- `codex-stats`

and asserts a shared parser can:

- read common metrics (`source_family`, `source_type`, token totals, session counts)
- branch on semantics metadata when source-specific logic is needed

This is not required for issue #9 MVP, but strongly recommended before downstream Metrick integration.

---

## Implementation Sequence (Updated with Real-Fixture Gate)

1. Capture real Codex logs for `2026-02-23` and create sanitized fixture
2. Create `manifest.json` with hand-verified totals and semantics notes
3. Draft and review `expected/codex-sessions.json`
4. Draft and review `expected/codex-stats.json`
5. Implement `codex-sessions`
6. Make `codex-sessions` pass real-fixture tests
7. Implement `codex-stats`
8. Make `codex-stats` pass real-fixture tests
9. Add synthetic edge-case coverage

---

## Definition of Done (Issue #9)

Issue #9 is not complete until:

- `codex-sessions` and `codex-stats` are implemented
- both pass real-fixture tests using frozen `2026-02-23` Codex logs
- semantics differences vs Claude sources are explicit in schema metadata
- receiver-facing parsing can distinguish differences without ambiguity

