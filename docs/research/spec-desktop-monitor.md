# Feature Request: Desktop Presence Monitor

**Date:** 2026-02-12
**Origin:** Goldratt's Cycle architecture (Entourage Activity Feed)
**Priority:** Phase 2
**Related:** REQ-020 (Desktop Session Logs), `docs/research/spec-desktop-session-logs.md`

---

## Problem

The Entourage's human utilization tracking relies on Claude Code session data via LocalPush. This captures when the human is actively working in a terminal — but misses:

- Time spent reviewing code output (reading, not typing)
- Time spent planning in Notion, email, or other apps
- Time spent in meetings or calls related to Associate work
- Idle gaps between sessions (human thinking vs. human away)

Without this, the utilization math treats all non-session time as idle, understating true human engagement and making the leverage ratio artificially high.

## Proposed Solution

A lightweight **desktop presence monitor** as a LocalPush source that tracks keyboard/mouse activity to produce human-on-keyboard session data.

### What It Captures

| Signal | Method | Privacy Level |
|--------|--------|---------------|
| Active/idle state | Keyboard + mouse event detection (boolean, not keystrokes) | Low (no content) |
| Active application name | `NSWorkspace.frontmostApplication` | Medium (app names only) |
| Session boundaries | Active → idle transition (configurable threshold, default 5min) | Low |
| Daily active time | Sum of active periods | Low |

### What It Does NOT Capture

- Keystrokes or key combinations
- Mouse coordinates or click targets
- Screen content or screenshots
- URLs or document names
- Clipboard content

### Output Schema

Pushed daily (or on-demand), similar to Claude Stats source:

```json
{
  "source": "desktop-monitor",
  "date": "2026-02-12",
  "sessions": [
    {
      "started_at": "2026-02-12T09:03:00+01:00",
      "ended_at": "2026-02-12T12:47:00+01:00",
      "active_seconds": 12840,
      "idle_seconds": 2700,
      "top_apps": [
        { "name": "Terminal", "seconds": 5400 },
        { "name": "Arc", "seconds": 3200 },
        { "name": "Notion", "seconds": 2100 }
      ]
    },
    {
      "started_at": "2026-02-12T20:15:00+01:00",
      "ended_at": "2026-02-12T23:42:00+01:00",
      "active_seconds": 10620,
      "idle_seconds": 2700,
      "top_apps": [
        { "name": "Terminal", "seconds": 6300 },
        { "name": "Arc", "seconds": 2800 }
      ]
    }
  ],
  "summary": {
    "total_active_seconds": 23460,
    "total_idle_seconds": 5400,
    "session_count": 2,
    "top_apps": ["Terminal", "Arc", "Notion"]
  }
}
```

### Integration with Goldratt's Cycle

This data feeds the **Human Utilization** calculations in `entourage_activity`:

1. **Precise denominator** — Instead of the conservative 11h window assumption, use actual on-keyboard time as the denominator for utilization %
2. **Evening opus detection** — Active time >60min + 5+ Claude sessions = opus-tier attention (SP-1 heuristic)
3. **Idle analysis** — Distinguish "human thinking between sessions" from "human away" to refine the leverage ratio
4. **Pre-loaded work detection** — Identify when the human queues work before family time and returns later to process results

### Implementation Notes

- **macOS API:** `CGEventTap` for keyboard/mouse activity detection (requires Accessibility permission)
- **Alternative:** `IOHIDManager` for HID-level activity without Accessibility
- **App detection:** `NSWorkspace.shared.frontmostApplication` (no special permissions)
- **Idle threshold:** Configurable (default 5 minutes of no input = idle)
- **Battery impact:** Polling at 1s intervals is negligible; event-driven even better
- **Permissions:** Accessibility (System Preferences → Privacy → Accessibility). Same pattern as existing Accessibility-dependent apps

### Relationship to REQ-020

REQ-020 (Desktop Session Logs) reads historical data from `knowledgeC.db` (CoreDuet). This feature request is complementary:

- **REQ-020:** Retrospective analysis of past sessions from system database
- **Desktop Monitor:** Real-time active/idle tracking with richer data (top apps, precise boundaries)

Both could coexist — REQ-020 for historical backfill, Desktop Monitor for live tracking.

### Acceptance Criteria

1. Source appears in LocalPush source list as "Desktop Monitor"
2. Transparency preview shows real active/idle data before enabling
3. Daily push produces the schema above
4. On-demand "Push Now" works
5. Idle threshold is configurable in source settings
6. No keystrokes, URLs, or content are captured (verifiable in transparency preview)

---

## Decision Needed

- Phase 1 or Phase 2? Currently slotted as Phase 2 (REQ-020 dependency), but Goldratt's Cycle would benefit from having this data in Phase 1 for accurate human utilization.
