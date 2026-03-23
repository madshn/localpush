# LocalPush 0.5.0

This release fixes the source-side issues identified in LocalPush issue `#16` and tightens delivery behavior for aggregate-style pushes.

## What changed

- Claude stats now includes real `tool_calls` data in the payload. We now count actual Claude `tool_use` events instead of emitting a placeholder `0`.
- Desktop activity aggregation is fixed. The background worker and the source now share the same state, so scheduled and manual pushes read the real buffered session data.
- Desktop activity no longer emits empty overnight pushes on a fixed cadence. If there is nothing meaningful to report, LocalPush now skips the push.
- LocalPush now applies a stronger cross-source delivery principle for scheduled and manual pushes: no push means nothing happened. Payloads can now be skipped when they contain no meaningful signal or when nothing has changed since the last successful push.

## Expected receiving-end impact

- `claude_stats` payloads should now show non-zero `tool_calls` when Claude sessions used tools.
- `desktop_activity` should stop producing repeated empty `sessions: []` payloads during idle periods.
- Aggregate-style sources should be quieter overall, with fewer no-op deliveries and fewer duplicate payloads that only differ by timestamps.

## Version

- App version: `0.5.0`

## Build note

- This build was packaged and installed as a macOS `.app` bundle and verified running locally as `0.5.0`.
- Updater signing was not produced in this shell because `TAURI_SIGNING_PRIVATE_KEY` was not available during the build, but the release app itself built and installed successfully.
