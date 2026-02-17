# Spec: Desktop Session Logs Source

**Created:** 2026-02-10
**Status:** Backlog (Phase 2+)
**Type:** New Source
**Priority:** High-value — "memory.ai light"

---

## Overview

A source that daily-pushes desktop activity summaries: total active time, top applications by usage, and optionally screen on/off periods. Uses macOS CoreDuet `knowledgeC.db` to extract Screen Time-equivalent data without relying on the Screen Time UI.

**Use case:** Productivity analysis, work session tracking, daily activity journaling. Push a summary like:

```json
{
  "source": "desktop_sessions",
  "timestamp": "2026-02-10T23:59:00+01:00",
  "date": "2026-02-10",
  "summary": {
    "total_active_minutes": 487,
    "screen_on_minutes": 612,
    "first_activity": "08:23",
    "last_activity": "22:15",
    "active_periods": [
      {"start": "08:23", "end": "12:45", "minutes": 262},
      {"start": "14:10", "end": "18:30", "minutes": 260}
    ]
  },
  "top_apps": [
    {"name": "Claude Code", "bundle_id": "com.anthropic.claude-code", "minutes": 185},
    {"name": "Safari", "bundle_id": "com.apple.Safari", "minutes": 92},
    {"name": "VS Code", "bundle_id": "com.microsoft.VSCode", "minutes": 78},
    {"name": "Slack", "bundle_id": "com.tinyspeck.slackmacgap", "minutes": 45},
    {"name": "Terminal", "bundle_id": "com.apple.Terminal", "minutes": 33}
  ],
  "stats": {
    "total_apps_used": 18,
    "longest_focus_session_minutes": 95,
    "longest_focus_app": "Claude Code"
  }
}
```

---

## Data Source

### Primary Database

| Field | Value |
|-------|-------|
| **Path** | `~/Library/Application Support/Knowledge/knowledgeC.db` |
| **Alt path** | `/private/var/db/CoreDuet/Knowledge/knowledgeC.db` (requires root) |
| **Engine** | SQLite (read-only, WAL mode) |
| **Access** | Requires **Full Disk Access (FDA)** via TCC |

### Key Tables

| Table | Purpose |
|-------|---------|
| `ZOBJECT` | Core event log — app focus, display state, device state |
| `ZSTRUCTUREDMETADATA` | Additional context (Safari URLs, activity types) |

### Essential Stream Names (`ZSTREAMNAME`)

| Stream | What it captures |
|--------|-----------------|
| `/app/inFocus` | Which app window is currently active (bundle ID + duration) |
| `/app/activity` | Specific actions within apps (compose, read, etc.) |
| `/display/isBacklit` | Screen on/off (1/0) — total screen time |
| `/safari/history` | URLs visited (even private browsing traces) |
| `/device/isPluggedIn` | Power state changes |

### Timestamp Conversion

All timestamps use **Mac Absolute Time** (seconds since 2001-01-01 00:00:00 UTC).

```
Unix timestamp = ZSTARTDATE + 978307200
```

---

## Extraction Query

### App Usage (Top N by Duration)

```sql
SELECT
    ZOBJECT.ZVALUESTRING AS bundle_id,
    SUM(ZOBJECT.ZENDDATE - ZOBJECT.ZSTARTDATE) AS total_seconds,
    COUNT(*) AS session_count,
    MIN(DATETIME(ZOBJECT.ZSTARTDATE + 978307200, 'unixepoch', 'localtime')) AS first_use,
    MAX(DATETIME(ZOBJECT.ZENDDATE + 978307200, 'unixepoch', 'localtime')) AS last_use
FROM ZOBJECT
WHERE ZOBJECT.ZSTREAMNAME = '/app/inFocus'
  AND DATE(ZOBJECT.ZSTARTDATE + 978307200, 'unixepoch', 'localtime') = DATE('now', 'localtime')
  AND ZOBJECT.ZVALUESTRING IS NOT NULL
GROUP BY ZOBJECT.ZVALUESTRING
ORDER BY total_seconds DESC
LIMIT 20;
```

### Screen On Time

```sql
SELECT
    SUM(ZOBJECT.ZENDDATE - ZOBJECT.ZSTARTDATE) AS screen_on_seconds
FROM ZOBJECT
WHERE ZOBJECT.ZSTREAMNAME = '/display/isBacklit'
  AND ZOBJECT.ZVALUEINTEGER = 1
  AND DATE(ZOBJECT.ZSTARTDATE + 978307200, 'unixepoch', 'localtime') = DATE('now', 'localtime');
```

### Active Periods (Gaps > 30 min = new period)

Compute in Rust: sort `/app/inFocus` events by start time, merge overlapping ranges, split on gaps > 30 minutes.

---

## Implementation Design

### Source Struct

```rust
pub struct DesktopSessions {
    db_path: PathBuf,  // ~/Library/Application Support/Knowledge/knowledgeC.db
}
```

### Delivery Mode

**Scheduled only** — not on_change. The knowledgeC.db changes constantly (hundreds of writes per hour). Use the scheduled delivery worker:

```
delivery_mode: "daily"
schedule_time: "23:55"  // End of day summary
```

### Watch Path

Watch the **directory** (not the DB file) since WAL writes create frequent events:

```
~/Library/Application Support/Knowledge/
```

But given scheduled-only delivery, watching is optional. The scheduler will trigger `parse()` at the configured time regardless.

### Privacy Considerations

| Data | Default | User Toggle |
|------|---------|-------------|
| Bundle IDs | Included | Always on (non-sensitive) |
| App names (resolved from bundle ID) | Included | Toggle: `resolve_app_names` |
| Total minutes per app | Included | Always on |
| Safari URLs/titles | **Excluded** | Toggle: `include_browsing` (privacy-sensitive) |
| Active periods (start/end times) | Included | Toggle: `include_active_periods` |
| Window titles | **Excluded** | Toggle: `include_window_titles` (privacy-sensitive) |

### Bundle ID to App Name Resolution

Use `NSWorkspace` or parse `/Applications/*.app/Contents/Info.plist` to resolve `com.apple.Safari` to "Safari". Cache the mapping.

---

## Technical Hurdles

1. **Full Disk Access (FDA)** — Required. First-run UX must guide user to System Settings > Privacy > Full Disk Access and add LocalPush. Without FDA, the DB read fails silently or returns permission error.

2. **WAL Lock** — Open DB in read-only mode with `PRAGMA journal_mode=WAL; PRAGMA synchronous=OFF;` to avoid contention with macOS writes.

3. **Binary Plists in ZSTRUCTUREDMETADATA** — Some metadata is stored as BLOB (binary plist). Need `plist` crate or `plutil` subprocess to decode. For the MVP (app usage only), this can be skipped.

4. **Biome Migration** — Apple is slowly migrating from `knowledgeC.db` to the newer **Biome** system (`.segb` files). As of macOS Sequoia/Tahoe (2025-2026), both systems coexist. Monitor for deprecation.

5. **Mac Absolute Time** — All timestamps need +978307200 offset. Use `chrono` with the offset constant.

---

## References

### Apple Official

- [FSEvents API](https://developer.apple.com/documentation/coreservices/file_system_events) — File system monitoring
- [Endpoint Security Framework](https://developer.apple.com/documentation/endpointsecurity) — System event monitoring
- [Security-Scoped Bookmarks](https://developer.apple.com/documentation/foundation/nsurl/1417051-startaccessingsecurityscopedreso) — Persistent folder access

### Community Schema References

- [KnowledgeC Forensics (Belkasoft)](https://belkasoft.com/knowledgec-database-forensics-with-belkasoft) — ZOBJECT column breakdown
- [Mac4n6 - Knowledge is Power](http://www.mac4n6.com/blog/2018/8/5/knowledge-is-power-using-the-knowledgecdb-database-on-macos-and-ios-to-determine-precise-user-and-application-usage) — Definitive reverse-engineering of CoreDuet schema
- [DoubleBlak - KnowledgeC](https://www.doubleblak.com/blogPost.php?k=knowledgec) — Full ZSTREAMNAME reference

### Permissions

- [TCC Database (HackTricks)](https://book.hacktricks.xyz/macos-hardening/macos-security-and-privilege-escalation/macos-security-protections/macos-tcc) — Full Disk Access mechanics

### Libraries

- [FMDB (Swift)](https://github.com/ccgus/fmdb) — Thread-safe SQLite wrapper
- [biplist (Python)](https://pypi.org/project/biplist/) — Binary plist parsing (if needed for metadata BLOBs)

### Constants

| Constant | Value |
|----------|-------|
| Mac Epoch Offset | `978307200` |
| User DB Path | `~/Library/Application Support/Knowledge/knowledgeC.db` |
| System DB Path | `/private/var/db/CoreDuet/Knowledge/knowledgeC.db` |

---

## Future: Biome System

Apple's newer Biome system stores data in `.segb` (segmented binary) files at:

```
~/Library/Biome/streams/
```

The `biome` CLI utility can dump these. As `knowledgeC.db` is deprecated, a Biome adapter may be needed. Track this as a separate research item.

---

## Scope for LocalPush

### MVP (Phase 2)

- Daily push of yesterday's activity summary
- Top 5 apps by active minutes
- Total active time + screen on time
- Active periods (work blocks)
- No browsing data (privacy default)

### Extended (Phase 3)

- Configurable push frequency (daily/weekly)
- Safari browsing summary (opt-in)
- Week-over-week trends
- Focus session detection (sustained single-app usage > 25 min)
- Category grouping (Development, Communication, Entertainment)
