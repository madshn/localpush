# Spec: Discovery Mode

**Created:** 2026-02-10
**Status:** Backlog (Phase 2+)
**Type:** Platform Feature
**Priority:** High — differentiator, unlocks long-tail sources

---

## Overview

Discovery Mode scans the Mac for SQLite databases inside application containers and suggests them as potential data sources. Instead of hardcoding every source, LocalPush becomes a **platform** that can push data from ANY installed app.

**User experience:** Open Settings > "Discover Sources" > See a list of installed apps with local databases > Pick one > Preview the data > Bind to a target.

---

## How It Works

### Database Discovery

Scan three locations where macOS apps store persistent data:

```bash
# 1. Standard Application Support
~/Library/Application Support/*/*.sqlite

# 2. Sandboxed Containers (App Store apps)
~/Library/Containers/*/Data/Library/Application Support/**/*.sqlite

# 3. Group Containers (shared app data — Notes, Podcasts, Reminders)
~/Library/Group Containers/*/**/*.sqlite
```

### App Identification

Map discovered databases back to their owning app:

1. Extract bundle ID from container path (e.g., `com.agiletortoise.Drafts-osx`)
2. Resolve app name via `mdfind "kMDItemCFBundleIdentifier == '{bundle_id}'"` or parse `Info.plist`
3. Get app icon from `/Applications/*.app/Contents/Resources/`

### Schema Inspection

For each discovered database:

1. `PRAGMA table_info()` on each table
2. Count rows per table
3. Identify timestamp columns (for "recent changes" filtering)
4. Identify text/VARCHAR columns (for preview)
5. Present a table browser where the user picks which tables/columns to push

---

## Known High-Value Candidates

| App | Bundle ID | Database Path | Gold Mine |
|-----|-----------|---------------|-----------|
| **Drafts** | `com.agiletortoise.Drafts-osx` | `~/Library/Containers/.../Application Support/*.sqlite` | Every text snippet, thought, draft |
| **Bear Notes** | `9K33E3U3T4.net.shinyfrog.bear` | `~/Library/Group Containers/.../database.sqlite` | Tagged notes, markdown content |
| **Things 3** | `com.culturedcode.ThingsMac` | `~/Library/Group Containers/.../Things Database.thingsdatabase/main.sqlite` | Tasks, projects, tags, dates |
| **Raycast** | `com.raycast.macos` | `~/Library/Application Support/com.raycast.macos/*.sqlite` | Snippets, clipboard history, extensions |
| **IINA / VLC** | `com.colliderli.iina` | `~/Library/Application Support/com.colliderli.iina/*.sqlite` | Playback history, resume positions |
| **Signal Desktop** | `org.whispersystems.signal-desktop` | `~/Library/Application Support/Signal/sql/db.sqlite` | Message history (SQLCipher encrypted — needs Keychain key) |

---

## Implementation Design

### Phase 1: Scanner

```rust
pub struct DiscoveryScanner {
    search_paths: Vec<PathBuf>,
}

impl DiscoveryScanner {
    pub fn scan(&self) -> Vec<DiscoveredDatabase> { ... }
}

pub struct DiscoveredDatabase {
    pub path: PathBuf,
    pub bundle_id: Option<String>,
    pub app_name: Option<String>,
    pub tables: Vec<TableInfo>,
    pub total_rows: u64,
    pub file_size_bytes: u64,
    pub last_modified: SystemTime,
}
```

### Phase 2: Dynamic Source

```rust
pub struct DynamicSource {
    pub id: String,
    pub name: String,
    pub db_path: PathBuf,
    pub query: String,          // User-configured SQL SELECT
    pub watch_path: PathBuf,    // Watch for -wal changes
    pub timestamp_column: Option<String>,  // For "recent only" filtering
}
```

### Phase 3: Template Library

Pre-built query templates for popular apps:

```toml
[template.things3]
name = "Things 3 Tasks"
query = "SELECT ZTITLE, ZSTATUS, ZDUEDATE FROM ZTHING WHERE ZTRASHED = 0"
timestamp = "ZMODIFICATIONDATE"

[template.bear]
name = "Bear Notes"
query = "SELECT ZTITLE, ZSUBTITLE, ZCREATIONDATE FROM ZSFNOTE WHERE ZTRASHED = 0"
timestamp = "ZMODIFICATIONDATE"
```

---

## Technical Notes

### File Watching Strategy

Watch the **container directory** (not individual `.sqlite` files) because:
- Apps create/delete `-wal` and `-shm` files during writes
- FSEvents is more reliable on directories
- Use 60-second debounce to avoid excessive triggers

### Bundle ID Resolution

```bash
# Find app path from bundle ID
mdfind "kMDItemCFBundleIdentifier == 'com.culturedcode.ThingsMac'"

# Or scan /Applications manually
plutil -p /Applications/Things3.app/Contents/Info.plist | grep CFBundleIdentifier
```

### Copy-on-Write Safety

Never query an app's live database directly during active writes:

```rust
// 1. Copy .sqlite + .sqlite-wal + .sqlite-shm to temp dir
// 2. Open the copy in read-only mode
// 3. Run queries against the copy
// 4. Delete the copy
```

This prevents SQLite lock contention and data corruption.

### Permissions

- Most `~/Library/Containers/` paths require **Full Disk Access**
- Group Containers are generally accessible without FDA
- Signal Desktop DB requires SQLCipher decryption (key in macOS Keychain)

---

## UX Flow

```
Settings > Discover Sources
  │
  ├── [Scanning...] (searches 3 container paths)
  │
  ├── Found 47 databases from 23 apps
  │   ├── Things 3 (main.sqlite — 2,847 tasks)
  │   ├── Bear (database.sqlite — 412 notes)
  │   ├── Drafts (Drafts.sqlite — 1,203 drafts)
  │   ├── Raycast (clipboard.sqlite — 890 items)
  │   └── [Show all...]
  │
  └── Click "Things 3"
      ├── Tables: ZTHING (2,847 rows), ZPROJECT (23 rows), ZTAG (45 rows)
      ├── Preview: [shows 5 sample rows with Radical Transparency]
      ├── Template: "Things 3 Tasks" [Use Template] or [Custom Query]
      └── [Add as Source] → enters standard enable flow
```

---

## Risks

1. **Schema instability** — App updates can change table/column names. Templates need version pinning or graceful fallback.
2. **Sandbox permissions** — Some containers need FDA. Clear UX for permission requests.
3. **Data sensitivity** — Discovery exposes ALL local databases. Privacy warnings essential.
4. **SQLCipher** — Encrypted databases (Signal, 1Password) need special handling or should be flagged as unsupported.
5. **Performance** — Full filesystem scan could be slow. Cache results, scan lazily.

---

## References

- `mdfind` for bundle ID resolution: built-in macOS Spotlight CLI
- Container paths: Apple Developer Documentation on App Sandbox
- SQLite copy-on-write: standard forensic practice for live databases
- SQLCipher: https://www.zetetic.net/sqlcipher/
