# Apple Podcasts Source Plugin - Implementation Plan

**Date:** 2026-02-06
**Target App:** LocalPush (Tauri 2.0 macOS)
**Template:** Claude Stats source (`~/dev/localpush/src-tauri/src/sources/claude_stats.rs`)

---

## Executive Summary

Apple Podcasts stores listening history in a SQLite database located in Group Containers. This plugin will watch the database file and extract recent listening activity (episode plays, completions, timestamps) to send via webhooks.

**Key Challenge:** The database is protected by macOS TCC (Transparency, Consent, and Control) permissions starting with macOS Sequoia. Users must grant LocalPush Full Disk Access to read Group Containers.

---

## 1. Database Location and Access

### File Path

```
~/Library/Group Containers/243LU875E5.groups.com.apple.podcasts/Documents/MTLibrary.sqlite
```

**Components:**
- `243LU875E5` — Apple's team identifier (same for all users)
- `MTLibrary.sqlite` — Main Core Data SQLite database
- Companion files: `MTLibrary.sqlite-wal`, `MTLibrary.sqlite-shm` (WAL mode)

### Access Requirements

**macOS Permissions:**
- **Full Disk Access** required (TCC protection on Group Containers)
- User must manually enable in System Settings → Privacy & Security → Full Disk Access → [Add LocalPush]
- No programmatic bypass available

**Database Locking:**
- Apple Podcasts app keeps the database open in WAL mode
- **Multiple readers allowed** — LocalPush can query while Podcasts app is running
- WAL mode ensures read consistency without locking conflicts

**Reference:** [macOS TCC Protection](https://book.hacktricks.wiki/en/macos-hardening/macos-security-and-privilege-escalation/macos-security-protections/macos-tcc/index.html)

---

## 2. Database Schema

### Core Tables

Based on research and the [Si Jobling blog post](https://sijobling.com/blog/recently-played-episodes-data-from-apple-podcasts/):

#### `ZMTEPISODE` (Episode Data)

| Column | Type | Description |
|--------|------|-------------|
| `Z_PK` | INTEGER | Primary key |
| `ZPODCAST` | INTEGER | Foreign key to ZMTPODCAST.Z_PK |
| `ZTITLE` | TEXT | Episode title |
| `ZLASTDATEPLAYED` | REAL | Last played timestamp (Core Data epoch) |
| `ZWEBPAGEURL` | TEXT | Episode webpage URL |
| `ZENCLOSUREURL` | TEXT | MP3/audio file URL |
| `ZDURATION` | REAL | Episode duration in seconds (estimated) |
| `ZPLAYSTATE` | INTEGER | Playback state (0=unplayed, 2=played, etc.) |
| `ZPLAYCOUNT` | INTEGER | Number of times played (likely exists) |
| `ZBYTESIZE` | INTEGER | File size in bytes (likely exists) |

#### `ZMTPODCAST` (Podcast/Show Data)

| Column | Type | Description |
|--------|------|-------------|
| `Z_PK` | INTEGER | Primary key |
| `ZTITLE` | TEXT | Podcast title |
| `ZFEEDURL` | TEXT | RSS feed URL |
| `ZWEBPAGEURL` | TEXT | Podcast homepage URL |
| `ZAUTHOR` | TEXT | Podcast author/creator (likely exists) |
| `ZIMAGEUUID` | TEXT | Artwork identifier (likely exists) |

**Core Data Epoch:** Apple uses `978307200` as the offset (seconds between Unix epoch `1970-01-01` and Apple epoch `2001-01-01`).

```rust
// Convert Core Data timestamp to Unix timestamp
let unix_timestamp = core_data_timestamp + 978307200.0;
```

**Note:** Schema is inferred from available research. Exact column names may need verification by inspecting the actual database with DB Browser for SQLite.

---

## 3. SQL Queries

### Recent Listening History

```sql
SELECT
    e.ZTITLE AS episode_title,
    e.ZLASTDATEPLAYED AS last_played_raw,
    (e.ZLASTDATEPLAYED + 978307200) AS last_played_unix,
    e.ZWEBPAGEURL AS episode_url,
    e.ZENCLOSUREURL AS audio_url,
    e.ZPLAYSTATE AS play_state,
    e.ZPLAYCOUNT AS play_count,
    p.ZTITLE AS podcast_title,
    p.ZFEEDURL AS feed_url,
    p.ZAUTHOR AS podcast_author
FROM ZMTEPISODE e
INNER JOIN ZMTPODCAST p ON e.ZPODCAST = p.Z_PK
WHERE e.ZLASTDATEPLAYED IS NOT NULL
ORDER BY e.ZLASTDATEPLAYED DESC
LIMIT 50;
```

**Filters:**
- `WHERE e.ZLASTDATEPLAYED IS NOT NULL` — Only episodes that have been played
- `ORDER BY e.ZLASTDATEPLAYED DESC` — Most recent first
- `LIMIT 50` — Reasonable payload size for webhook

### Today's Listening Activity

```sql
SELECT
    COUNT(*) AS episodes_played,
    SUM(CASE WHEN e.ZPLAYSTATE = 2 THEN 1 ELSE 0 END) AS episodes_completed,
    COUNT(DISTINCT e.ZPODCAST) AS unique_shows
FROM ZMTEPISODE e
WHERE e.ZLASTDATEPLAYED >= :today_start_raw;
```

**Variables:**
- `:today_start_raw` — Core Data timestamp for start of today (calculated in Rust)

```rust
// Calculate start of today in Core Data epoch
let now = chrono::Local::now();
let today_start = now.date_naive().and_hms_opt(0, 0, 0).unwrap();
let today_start_unix = today_start.and_utc().timestamp() as f64;
let today_start_raw = today_start_unix - 978307200.0;
```

---

## 4. Rust Implementation Plan

### File Structure

```
src-tauri/src/sources/
├── mod.rs              (add apple_podcasts module)
├── claude_stats.rs     (existing template)
└── apple_podcasts.rs   (new source)
```

### Dependencies (Cargo.toml)

```toml
[dependencies]
rusqlite = { version = "0.32", features = ["bundled"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
chrono = "0.4"
tracing = "0.1"
```

**Note:** `bundled` feature includes libsqlite3 to avoid linking issues on macOS.

### Struct Definition

```rust
use super::{PreviewField, Source, SourceError, SourcePreview};
use chrono::{DateTime, Utc, NaiveDate};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::{debug, info, warn};

/// Apple Podcasts listening history source
pub struct ApplePodcastsSource {
    db_path: PathBuf,
}

impl ApplePodcastsSource {
    pub fn new() -> Result<Self, SourceError> {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .map_err(|_| {
                SourceError::ParseError("Could not determine home directory".to_string())
            })?;

        let db_path = PathBuf::from(home)
            .join("Library")
            .join("Group Containers")
            .join("243LU875E5.groups.com.apple.podcasts")
            .join("Documents")
            .join("MTLibrary.sqlite");

        Ok(Self { db_path })
    }

    pub fn new_with_path(path: impl Into<PathBuf>) -> Self {
        Self {
            db_path: path.into(),
        }
    }

    /// Open read-only connection to the database
    fn open_connection(&self) -> Result<Connection, SourceError> {
        if !self.db_path.exists() {
            return Err(SourceError::FileNotFound(self.db_path.clone()));
        }

        // Open in read-only mode to avoid lock conflicts
        let conn = Connection::open_with_flags(
            &self.db_path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )?;

        Ok(conn)
    }

    /// Convert Core Data timestamp to Unix timestamp
    fn core_data_to_unix(raw: f64) -> f64 {
        raw + 978307200.0
    }

    /// Get start of today in Core Data epoch
    fn today_start_raw() -> f64 {
        let now = chrono::Local::now();
        let today_start = now.date_naive().and_hms_opt(0, 0, 0).unwrap();
        let today_start_unix = today_start.and_utc().timestamp() as f64;
        today_start_unix - 978307200.0
    }
}
```

### Payload Structs

```rust
/// Structured payload sent to webhooks
#[derive(Debug, Serialize, Deserialize)]
pub struct ApplePodcastsPayload {
    pub recent_episodes: Vec<EpisodePlay>,
    pub today_summary: TodaySummary,
    pub metadata: PayloadMetadata,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EpisodePlay {
    pub episode_title: String,
    pub podcast_title: String,
    pub podcast_author: Option<String>,
    pub last_played_at: DateTime<Utc>,
    pub episode_url: Option<String>,
    pub audio_url: Option<String>,
    pub feed_url: Option<String>,
    pub play_state: i32,
    pub play_count: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TodaySummary {
    pub episodes_played: i32,
    pub episodes_completed: i32,
    pub unique_shows: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PayloadMetadata {
    pub source: String,
    pub generated_at: DateTime<Utc>,
    pub database_path: String,
}
```

### Source Trait Implementation

```rust
impl Source for ApplePodcastsSource {
    fn id(&self) -> &str {
        "apple-podcasts"
    }

    fn name(&self) -> &str {
        "Apple Podcasts Listening History"
    }

    fn watch_path(&self) -> Option<PathBuf> {
        // Watch the main database file
        // WAL changes will trigger file modification events
        Some(self.db_path.clone())
    }

    fn parse(&self) -> Result<serde_json::Value, SourceError> {
        let conn = self.open_connection()?;

        // Query recent episodes
        let mut stmt = conn.prepare(
            "SELECT
                e.ZTITLE, e.ZLASTDATEPLAYED, e.ZWEBPAGEURL, e.ZENCLOSUREURL,
                e.ZPLAYSTATE, e.ZPLAYCOUNT,
                p.ZTITLE, p.ZFEEDURL, p.ZAUTHOR
            FROM ZMTEPISODE e
            INNER JOIN ZMTPODCAST p ON e.ZPODCAST = p.Z_PK
            WHERE e.ZLASTDATEPLAYED IS NOT NULL
            ORDER BY e.ZLASTDATEPLAYED DESC
            LIMIT 50"
        )?;

        let recent_episodes: Vec<EpisodePlay> = stmt
            .query_map([], |row| {
                let raw_timestamp: f64 = row.get(1)?;
                let unix_timestamp = Self::core_data_to_unix(raw_timestamp);
                let datetime = DateTime::from_timestamp(unix_timestamp as i64, 0)
                    .unwrap_or_else(|| DateTime::from_timestamp(0, 0).unwrap());

                Ok(EpisodePlay {
                    episode_title: row.get(0)?,
                    last_played_at: datetime,
                    episode_url: row.get(2)?,
                    audio_url: row.get(3)?,
                    play_state: row.get(4)?,
                    play_count: row.get(5).ok(),
                    podcast_title: row.get(6)?,
                    feed_url: row.get(7)?,
                    podcast_author: row.get(8).ok(),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        // Query today's summary
        let today_start = Self::today_start_raw();
        let today_summary: TodaySummary = conn.query_row(
            "SELECT
                COUNT(*) AS episodes_played,
                SUM(CASE WHEN e.ZPLAYSTATE = 2 THEN 1 ELSE 0 END) AS episodes_completed,
                COUNT(DISTINCT e.ZPODCAST) AS unique_shows
            FROM ZMTEPISODE e
            WHERE e.ZLASTDATEPLAYED >= ?1",
            [today_start],
            |row| {
                Ok(TodaySummary {
                    episodes_played: row.get(0)?,
                    episodes_completed: row.get(1).unwrap_or(0),
                    unique_shows: row.get(2)?,
                })
            },
        )?;

        let payload = ApplePodcastsPayload {
            recent_episodes,
            today_summary,
            metadata: PayloadMetadata {
                source: "localpush".to_string(),
                generated_at: Utc::now(),
                database_path: self.db_path.display().to_string(),
            },
        };

        serde_json::to_value(payload).map_err(SourceError::JsonError)
    }

    fn preview(&self) -> Result<SourcePreview, SourceError> {
        let conn = self.open_connection()?;

        // Get today's summary for preview
        let today_start = Self::today_start_raw();
        let (episodes_today, unique_shows): (i32, i32) = conn.query_row(
            "SELECT
                COUNT(*),
                COUNT(DISTINCT e.ZPODCAST)
            FROM ZMTEPISODE e
            WHERE e.ZLASTDATEPLAYED >= ?1",
            [today_start],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;

        // Get most recent episode
        let most_recent: Option<(String, String)> = conn
            .query_row(
                "SELECT e.ZTITLE, p.ZTITLE
                FROM ZMTEPISODE e
                INNER JOIN ZMTPODCAST p ON e.ZPODCAST = p.Z_PK
                WHERE e.ZLASTDATEPLAYED IS NOT NULL
                ORDER BY e.ZLASTDATEPLAYED DESC
                LIMIT 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .ok();

        let summary = if episodes_today > 0 {
            format!(
                "{} episodes from {} shows today",
                episodes_today, unique_shows
            )
        } else {
            "No listening activity today".to_string()
        };

        let mut fields = vec![
            PreviewField {
                label: "Episodes Today".to_string(),
                value: episodes_today.to_string(),
                sensitive: false,
            },
            PreviewField {
                label: "Shows Today".to_string(),
                value: unique_shows.to_string(),
                sensitive: false,
            },
        ];

        if let Some((episode, podcast)) = most_recent {
            fields.push(PreviewField {
                label: "Last Episode".to_string(),
                value: episode,
                sensitive: true, // Episode titles may reveal preferences
            });
            fields.push(PreviewField {
                label: "Podcast".to_string(),
                value: podcast,
                sensitive: true, // Podcast subscriptions reveal interests
            });
        }

        Ok(SourcePreview {
            title: self.name().to_string(),
            summary,
            fields,
            last_updated: Some(Utc::now()), // Real-time query
        })
    }
}
```

### Error Handling

```rust
impl From<rusqlite::Error> for SourceError {
    fn from(err: rusqlite::Error) -> Self {
        SourceError::ParseError(format!("SQLite error: {}", err))
    }
}
```

---

## 5. Sample JSON Payload

```json
{
  "recent_episodes": [
    {
      "episode_title": "The Future of AI",
      "podcast_title": "Tech Talk Daily",
      "podcast_author": "John Doe",
      "last_played_at": "2026-02-06T14:32:15Z",
      "episode_url": "https://example.com/episodes/123",
      "audio_url": "https://cdn.example.com/audio/123.mp3",
      "feed_url": "https://example.com/feed.xml",
      "play_state": 2,
      "play_count": 1
    },
    {
      "episode_title": "Interview with Expert",
      "podcast_title": "Deep Dives",
      "podcast_author": "Jane Smith",
      "last_played_at": "2026-02-05T19:45:00Z",
      "episode_url": null,
      "audio_url": "https://cdn.example.com/audio/456.mp3",
      "feed_url": "https://example.com/feed2.xml",
      "play_state": 0,
      "play_count": 2
    }
  ],
  "today_summary": {
    "episodes_played": 3,
    "episodes_completed": 1,
    "unique_shows": 2
  },
  "metadata": {
    "source": "localpush",
    "generated_at": "2026-02-06T15:00:00Z",
    "database_path": "/Users/username/Library/Group Containers/243LU875E5.groups.com.apple.podcasts/Documents/MTLibrary.sqlite"
  }
}
```

**Play State Values (estimated):**
- `0` — Unplayed
- `1` — In progress
- `2` — Played/Completed

---

## 6. Privacy and Transparency Assessment

### Data Sensitivity: HIGH

Apple Podcasts listening history reveals:
- **Interests and preferences** — Topics, genres, creators
- **Daily routines** — Listening times indicate commute/exercise patterns
- **Political/religious views** — Podcast subscriptions can be highly revealing
- **Professional development** — Work-related educational content

### Privacy Risks

| Risk | Severity | Mitigation |
|------|----------|------------|
| Subscription exposure | High | Mark podcast/episode titles as `sensitive: true` in preview |
| Behavioral profiling | Medium | Limit payload to 50 recent episodes, not full history |
| Author/creator tracking | Medium | Include but mark as sensitive |
| URL leakage | Low | URLs are public, but aggregation reveals identity |

### Transparency Preview Requirements

**Must display in "coming soon" card and settings:**
1. **What data is collected:**
   - Episode titles and podcast names
   - Listening timestamps
   - Playback state (played/unplayed/in-progress)
   - Feed URLs and episode URLs

2. **What is NOT collected:**
   - Audio files themselves
   - Download history beyond listening history
   - User account information
   - Playback position within episodes

3. **TCC Permission Notice:**
   - "Requires Full Disk Access to read Apple Podcasts database"
   - "You must enable this in System Settings → Privacy & Security → Full Disk Access"
   - "LocalPush cannot access this data without your explicit permission"

4. **Data retention:**
   - "Only the 50 most recent episodes are included in each webhook"
   - "Data is sent immediately and not stored by LocalPush"

### Recommended Preview Fields

```rust
fields.push(PreviewField {
    label: "Last Episode".to_string(),
    value: episode_title,
    sensitive: true, // ← Blurred by default in UI
});

fields.push(PreviewField {
    label: "Podcast".to_string(),
    value: podcast_title,
    sensitive: true, // ← Blurred by default in UI
});
```

---

## 7. File Watching Strategy

### What to Watch: Main Database File

**Watch:** `MTLibrary.sqlite` (main database file)

**Why not WAL file?**
- WAL files (`MTLibrary.sqlite-wal`) are ephemeral and frequently truncated
- File system events on WAL are noisy and don't reliably indicate new listening activity
- Main database file receives modification events when WAL checkpoints occur
- macOS FSEvents will trigger on both direct writes and WAL commits

### File Watcher Configuration

```rust
// In LocalPush's file watcher (existing code)
fn watch_path(&self) -> Option<PathBuf> {
    Some(self.db_path.clone()) // Watch MTLibrary.sqlite
}
```

**Expected behavior:**
1. User plays/pauses/completes an episode in Apple Podcasts
2. Podcasts app writes to WAL file
3. Periodically (or on app close), WAL checkpoints to main database
4. FSEvents triggers on `MTLibrary.sqlite` modification
5. LocalPush detects change and calls `parse()`
6. Webhook sent with updated listening history

### Checkpoint Frequency

- **Active use:** WAL checkpoints every ~1000 pages (~4MB)
- **App close:** Full checkpoint on graceful exit
- **Result:** Changes may have a delay of minutes to hours

**Workaround for real-time updates:** Not feasible without hooking into Podcasts app or polling the database (which would drain battery).

---

## 8. Known Limitations and Gotchas

### 1. TCC Permission Barrier

**Problem:** LocalPush must have Full Disk Access to read Group Containers.

**Impact:**
- User must manually enable in System Settings
- No programmatic way to request this permission
- If permission is denied, source will fail with `FileNotFound` or permission error

**Solution:**
- Provide clear onboarding instructions in app
- Detect permission denial and show helpful error message
- Link to System Settings deep link (if available)

**Reference:** [macOS TCC HackTricks](https://book.hacktricks.wiki/en/macos-hardening/macos-security-and-privilege-escalation/macos-security-protections/macos-tcc/index.html)

### 2. Schema Volatility

**Problem:** Apple does not publish official schema documentation. Column names are inferred from reverse engineering.

**Impact:**
- Schema may change across macOS versions
- Column names like `ZPLAYCOUNT` or `ZAUTHOR` may not exist or may be named differently
- App updates could break queries

**Solution:**
- Use `SELECT *` queries initially to discover schema
- Wrap column access in `row.get().ok()` to handle missing columns gracefully
- Add schema version detection (query `sqlite_master` for table structure)
- Log warnings when expected columns are missing

### 3. WAL Mode Checkpoint Delays

**Problem:** Changes are written to WAL file first, checkpointed to main database later.

**Impact:**
- File watcher may not trigger immediately after listening activity
- User may see stale data in preview for minutes/hours

**Solution:**
- Document this limitation in UI ("Updates may be delayed")
- Consider adding manual refresh button
- OR: Poll WAL file size and trigger parse when it grows (increases battery usage)

### 4. Core Data Timestamp Conversion

**Problem:** Apple uses a non-standard epoch (`2001-01-01` instead of `1970-01-01`).

**Impact:**
- Timestamps will be wildly incorrect if offset is forgotten
- Off-by-one errors can shift dates by decades

**Solution:**
- Use constant `APPLE_EPOCH_OFFSET = 978307200.0`
- Add unit tests to verify conversion
- Validate output timestamps are reasonable (e.g., not in the year 2038)

### 5. Database Locking (Low Risk)

**Problem:** SQLite write locks prevent concurrent writes, but reads are allowed in WAL mode.

**Impact:**
- LocalPush should never encounter a lock conflict (read-only access)
- Opening in `SQLITE_OPEN_READ_ONLY` mode prevents accidental writes

**Solution:**
- Always open database with `SQLITE_OPEN_READ_ONLY` flag
- Handle `SQLITE_BUSY` errors gracefully (retry after delay)

### 6. Play State Enumeration Unknown

**Problem:** `ZPLAYSTATE` values are not documented.

**Impact:**
- Filtering by "completed" episodes may be inaccurate
- State values may have changed across macOS versions

**Solution:**
- Inspect database manually to observe state values
- Provide raw `play_state` integer in payload for debugging
- Update logic once values are confirmed

---

## 9. Implementation Checklist

- [ ] Add `rusqlite` dependency to `Cargo.toml` with `bundled` feature
- [ ] Create `src-tauri/src/sources/apple_podcasts.rs` file
- [ ] Define `ApplePodcastsSource` struct with `new()` and `new_with_path()` methods
- [ ] Implement `open_connection()` with `SQLITE_OPEN_READ_ONLY` flag
- [ ] Define payload structs: `ApplePodcastsPayload`, `EpisodePlay`, `TodaySummary`, `PayloadMetadata`
- [ ] Implement `Source::parse()` with two SQL queries (recent episodes, today summary)
- [ ] Implement `Source::preview()` with summary stats and most recent episode
- [ ] Add Core Data timestamp conversion helper (`core_data_to_unix`)
- [ ] Add unit tests for timestamp conversion
- [ ] Test with real Apple Podcasts database (verify schema matches)
- [ ] Handle missing columns gracefully (`.ok()` for optional fields)
- [ ] Register source in `src-tauri/src/sources/mod.rs`
- [ ] Update UI to mark episode/podcast titles as `sensitive` in preview
- [ ] Add TCC permission notice to "coming soon" card
- [ ] Write user documentation for enabling Full Disk Access
- [ ] Test file watching behavior (play episode, wait for checkpoint, verify webhook sent)
- [ ] Add error handling for permission denied (show helpful message)

---

## 10. Future Enhancements

### Phase 2: Advanced Features

1. **Playback Position Tracking**
   - Look for `ZPLAYHEADPOSITION` or similar column
   - Calculate listening time per episode
   - Detect partial plays vs full completions

2. **Subscription Management**
   - Query `ZMTPODCAST` for all subscribed shows
   - Track new subscriptions vs unsubscribes
   - Send webhook on subscription changes

3. **Download History**
   - Find table tracking downloaded episodes
   - Monitor storage usage per podcast
   - Alert when downloads consume excessive space

4. **Playlist Support**
   - Detect if playlists are stored in database
   - Track listening patterns across custom playlists

5. **Real-time WAL Monitoring**
   - Poll WAL file size every 60 seconds
   - Trigger parse when WAL grows significantly
   - Trade-off: battery usage vs real-time updates

---

## 11. Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_core_data_to_unix() {
        // 2026-02-06 12:00:00 UTC in Core Data epoch
        let raw = 761_486_400.0;
        let unix = ApplePodcastsSource::core_data_to_unix(raw);
        // Should equal 1739_793_600 (2026-02-06 12:00:00 UTC)
        assert_eq!(unix as i64, 1_739_793_600);
    }

    #[test]
    fn test_today_start_raw() {
        let raw = ApplePodcastsSource::today_start_raw();
        // Should be a reasonable value (not negative, not in the future)
        assert!(raw > 700_000_000.0); // After ~2023
        assert!(raw < 900_000_000.0); // Before ~2029
    }
}
```

### Integration Tests

1. **Mock Database:**
   - Create `tests/fixtures/MTLibrary.sqlite` with sample data
   - Test queries return expected results
   - Test missing columns don't crash

2. **Real Database (Manual):**
   - Run LocalPush on developer machine with real Podcasts data
   - Verify episode titles match what's in Podcasts app
   - Verify timestamps are accurate
   - Verify today's summary counts are correct

3. **Permission Denied:**
   - Test behavior when Full Disk Access is not granted
   - Verify helpful error message is shown

---

## 12. References

### Research Sources

1. **Apple Podcasts Database Structure:**
   - [Si Jobling: Recently Played Episodes Data from Apple Podcasts](https://sijobling.com/blog/recently-played-episodes-data-from-apple-podcasts/)
   - [Organizing Creativity: Exporting macOS Podcasts](https://www.organizingcreativity.com/2022/05/exporting-macos-podcasts/)
   - [Douglas Watson: How To Export Apple Podcasts to mp3 Files](https://douglas-watson.github.io/post/2020-05_export_podcasts/)

2. **macOS TCC Permissions:**
   - [HackTricks: macOS TCC](https://book.hacktricks.wiki/en/macos-hardening/macos-security-and-privilege-escalation/macos-security-protections/macos-tcc/index.html)
   - [Rainforest QA: A deep dive into macOS TCC.db](https://www.rainforestqa.com/blog/macos-tcc-db-deep-dive)

3. **SQLite WAL Mode:**
   - [SQLite Official: Write-Ahead Logging](https://sqlite.org/wal.html)
   - [SQLite Official: WAL-mode File Format](https://sqlite.org/walformat.html)
   - [Simon Willison: Enabling WAL mode for SQLite database files](https://til.simonwillison.net/sqlite/enabling-wal-mode)

4. **Rust SQLite Integration:**
   - [Tauri SQL Plugin](https://v2.tauri.app/plugin/sql/)
   - [GitHub: RandomEngy/tauri-sqlite](https://github.com/RandomEngy/tauri-sqlite)
   - [DEV: Tauri + SQLite](https://dev.to/randomengy/tauri-sqlite-p3o)
   - [MoonGuard: How to use local SQLite database with Tauri and Rust](https://blog.moonguard.dev/how-to-use-local-sqlite-database-with-tauri)

### Tools

- **DB Browser for SQLite:** [https://sqlitebrowser.org/](https://sqlitebrowser.org/) — Inspect schema and test queries
- **rusqlite crate:** [https://docs.rs/rusqlite/](https://docs.rs/rusqlite/) — Rust SQLite bindings

---

## 13. Open Questions

1. **Does `ZPLAYCOUNT` column exist?**
   - Need to inspect real database to confirm
   - May be named differently or not exist at all

2. **What are the exact `ZPLAYSTATE` enum values?**
   - Need to observe different states in real database
   - Likely: 0=unplayed, 1=in-progress, 2=completed

3. **Is there a `ZPLAYHEADPOSITION` column?**
   - Would enable tracking partial listens
   - Need to verify in real schema

4. **How frequently do WAL checkpoints occur?**
   - Empirical testing needed
   - May vary by macOS version or Podcasts app settings

5. **Can we detect when Podcasts app is running?**
   - If yes, could trigger parse immediately after app close (guaranteed checkpoint)
   - Would reduce latency for file watcher

---

## Conclusion

This implementation plan provides a comprehensive roadmap for building an Apple Podcasts source plugin for LocalPush. The main technical challenges are TCC permissions, schema discovery, and WAL checkpoint delays. With proper error handling and user documentation, this source will provide valuable listening history data for webhook integrations.

**Next Steps:**
1. Verify schema on real database using DB Browser for SQLite
2. Implement `ApplePodcastsSource` struct and `Source` trait
3. Test with real data and iterate on column names
4. Add UI transparency features for privacy
5. Write user documentation for Full Disk Access setup
