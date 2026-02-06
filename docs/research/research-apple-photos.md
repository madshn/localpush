# Apple Photos Source Plugin for LocalPush - Implementation Plan

**Date:** 2026-02-06
**Status:** Research Complete
**Platform:** macOS (Tauri 2.0)

---

## Executive Summary

This document outlines the implementation of an Apple Photos source plugin for LocalPush that extracts **statistics and metadata only** (no image content) from the user's local Photos library. The plugin will monitor the Photos.sqlite database for changes and deliver aggregated statistics to configured webhooks.

**Key Design Principle:** Privacy-first statistics. No image content, no face identities, optional location data.

---

## 1. Database Location and Access

### Primary Database Path

```
~/Pictures/Photos Library.photoslibrary/database/Photos.sqlite
```

**Additional files in the same directory:**
- `Photos.sqlite-wal` (Write-Ahead Log)
- `Photos.sqlite-shm` (Shared Memory)
- `Photos.sqlite-lock` (Lock file when Photos.app is running)

### Access Method

**Read-Only SQLite Access:**
```rust
use rusqlite::{Connection, OpenFlags};

let db_path = dirs::home_dir()
    .unwrap()
    .join("Pictures/Photos Library.photoslibrary/database/Photos.sqlite");

let conn = Connection::open_with_flags(
    &db_path,
    OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI
)?;
```

### Locking Behavior

**Critical findings:**
- The `photolibraryd` daemon holds a lock on the database while Photos.app is running
- **Read-only access is still possible** even when Photos.app is active (SQLite allows concurrent readers)
- WAL mode enables safe concurrent reads without blocking
- Lock file presence does NOT prevent reads, only writes

**Recommendation:** Always use `SQLITE_OPEN_READ_ONLY` flag to avoid any write conflicts.

---

## 2. TCC/Permissions Requirements

### Required Permissions

| Permission | Type | Reason |
|-----------|------|--------|
| **Full Disk Access** | TCC | Required to read `~/Pictures/Photos Library.photoslibrary/` |
| **Photos** | Optional | NOT required (we're reading SQLite directly, not using PhotoKit API) |

### Implementation Notes

1. **App must request Full Disk Access** in `Info.plist`:
   ```xml
   <key>NSAppleEventsUsageDescription</key>
   <string>LocalPush needs access to your Photos library database to generate statistics.</string>
   ```

2. **User must manually grant** in System Settings → Privacy & Security → Full Disk Access

3. **Detection:** Check if database is readable before attempting to parse:
   ```rust
   fn check_permissions(&self) -> bool {
       std::fs::metadata(&self.db_path).is_ok()
   }
   ```

4. **Graceful degradation:** If permission denied, return error with instructions to enable FDA

### Privacy Implications

The TCC database (`~/Library/Application Support/com.apple.TCC/TCC.db`) tracks permission grants. Full Disk Access is broad and powerful—users should understand this plugin can read their entire Photos library metadata.

---

## 3. Database Schema - Key Tables

### 3.1 ZASSET (Primary Asset Table)

**Purpose:** Core photo/video records

**Key Columns:**
| Column | Type | Description |
|--------|------|-------------|
| `Z_PK` | INTEGER | Primary key |
| `ZUUID` | TEXT | Unique asset identifier |
| `ZDATECREATED` | REAL | Unix timestamp (seconds since 2001-01-01) |
| `ZADDEDDATE` | REAL | When asset was added to library |
| `ZKIND` | INTEGER | Asset type (0=photo, 1=video) |
| `ZKINDSUBTYPE` | INTEGER | Subtype (panorama, live photo, time-lapse, etc.) |
| `ZLATITUDE` | REAL | GPS latitude (if available) |
| `ZLONGITUDE` | REAL | GPS longitude (if available) |
| `ZFAVORITE` | INTEGER | 1 if favorited, 0 otherwise |
| `ZTRASHEDSTATE` | INTEGER | 0=active, 1=trashed |
| `ZHASADJUSTMENTS` | INTEGER | 1 if edited, 0 otherwise |
| `ZHEIGHT` | INTEGER | Image height in pixels |
| `ZWIDTH` | INTEGER | Image width in pixels |

**Date Conversion:** Apple Core Data dates are stored as seconds since 2001-01-01 00:00:00 UTC
```rust
const APPLE_EPOCH_OFFSET: i64 = 978307200; // 2001-01-01 in Unix time

fn apple_date_to_unix(apple_date: f64) -> i64 {
    APPLE_EPOCH_OFFSET + apple_date as i64
}
```

### 3.2 ZADDITIONALASSETATTRIBUTES (Extended Metadata)

**Purpose:** Supplementary asset information

**Key Columns:**
| Column | Type | Description |
|--------|------|-------------|
| `ZASSET` | INTEGER | Foreign key to ZASSET.Z_PK |
| `ZIMPORTEDBYDISPLAYNAME` | TEXT | Source app (Camera, Instagram, etc.) |
| `ZORIGINALFILENAME` | TEXT | Original filename |
| `ZTIMEZONEOFFSET` | INTEGER | Timezone offset at capture |
| `ZTIMEZONENAME` | TEXT | Timezone name |
| `ZEXIFTIMESTAMPSTRING` | TEXT | EXIF timestamp string |
| `ZSCENEANALYSISTIMESTAMP` | REAL | When ML analysis completed |
| `ZASSETDESCRIPTION` | TEXT | User-added caption |

### 3.3 ZGENERICALBUM (Albums)

**Purpose:** User-created albums and smart albums

**Key Columns:**
| Column | Type | Description |
|--------|------|-------------|
| `Z_PK` | INTEGER | Primary key |
| `Z_ENT` | INTEGER | Entity type (Album, Folder, Smart Album) |
| `ZTITLE` | TEXT | Album name |
| `ZUUID` | TEXT | Unique album identifier |
| `ZKEYASSET` | INTEGER | Cover photo reference (FK to ZASSET) |
| `ZCREATIONDATE` | REAL | Album creation date |
| `ZSTARTDATE` | REAL | Earliest asset in album |
| `ZENDDATE` | REAL | Latest asset in album |
| `ZTRASHEDSTATE` | INTEGER | 0=active, 1=trashed |

### 3.4 Z_26ASSETS (Album Membership)

**Purpose:** Many-to-many relationship between albums and assets

**Note:** The table name varies by entity type. `Z_26ASSETS` is common, but may be different (check `Z_PRIMARYKEY` table).

**Key Columns:**
| Column | Type | Description |
|--------|------|-------------|
| `Z_26ALBUMS` | INTEGER | Foreign key to ZGENERICALBUM.Z_PK |
| `Z_3ASSETS` | INTEGER | Foreign key to ZASSET.Z_PK |

### 3.5 ZDETECTEDFACE (Face Detection)

**Purpose:** Face detection results (NOT identification)

**Key Columns:**
| Column | Type | Description |
|--------|------|-------------|
| `ZASSET` | INTEGER | Foreign key to ZASSET.Z_PK |
| `ZDETECTIONTYPE` | INTEGER | Face detection type |
| `ZCENTERPOINTX` | REAL | Face center X coordinate (normalized) |
| `ZCENTERPOINTY` | REAL | Face center Y coordinate (normalized) |
| `ZSIZE` | REAL | Face region size |

**Privacy Note:** This table does NOT contain face identities. It only shows detection bounding boxes.

### 3.6 ZSCENECLASSIFICATION (ML Scene Analysis)

**Purpose:** Apple's ML-detected scene types

**Key Columns:**
| Column | Type | Description |
|--------|------|-------------|
| `ZASSET` | INTEGER | Foreign key to ZASSET.Z_PK |
| `ZSCENEIDENTIFIER` | INTEGER | Scene type ID |
| `ZCONFIDENCE` | REAL | Confidence score (0.0-1.0) |

**Scene Examples:** Beach, Mountain, Sunset, Food, Pet, Indoor, Outdoor, etc.

**Critical:** This is a LARGE table. One test showed 762 assets generated 12,067 scene classification rows.

### 3.7 ZUNMANAGEDADJUSTMENT (Edits)

**Purpose:** Track photo/video edits

**Key Columns:**
| Column | Type | Description |
|--------|------|-------------|
| `ZASSET` | INTEGER | Foreign key to ZASSET.Z_PK |
| `ZADJUSTMENTTIMESTAMP` | REAL | When edit occurred |
| `ZADJUSTMENTFORMATVERSION` | TEXT | Edit type (Filter-1.4, Adjust-1.5, Video-Trim-1.6) |
| `ZEDITORLOCALIZEDNAME` | TEXT | App used for editing |

---

## 4. SQL Queries for Aggregate Statistics

### 4.1 Library Overview Stats

```sql
-- Total photos and videos
SELECT
    COUNT(*) as total_assets,
    SUM(CASE WHEN ZKIND = 0 THEN 1 ELSE 0 END) as photo_count,
    SUM(CASE WHEN ZKIND = 1 THEN 1 ELSE 0 END) as video_count,
    SUM(CASE WHEN ZFAVORITE = 1 THEN 1 ELSE 0 END) as favorites_count,
    MIN(ZDATECREATED) as earliest_date,
    MAX(ZDATECREATED) as latest_date
FROM ZASSET
WHERE ZTRASHEDSTATE = 0;
```

### 4.2 Recent Activity (Last 24h/7d)

```sql
-- Photos added in last 7 days
-- Apple dates: seconds since 2001-01-01
-- Current time - 7 days = (current_unix - 978307200) - (7 * 86400)

SELECT COUNT(*) as recent_imports_7d
FROM ZASSET
WHERE ZTRASHEDSTATE = 0
  AND ZADDEDDATE > (strftime('%s', 'now') - 978307200 - 604800);
```

### 4.3 Album Statistics

```sql
-- Total albums and photos per album
SELECT
    COUNT(DISTINCT a.Z_PK) as total_albums,
    AVG(asset_counts.count) as avg_photos_per_album
FROM ZGENERICALBUM a
LEFT JOIN (
    SELECT Z_26ALBUMS, COUNT(*) as count
    FROM Z_26ASSETS
    GROUP BY Z_26ALBUMS
) asset_counts ON a.Z_PK = asset_counts.Z_26ALBUMS
WHERE a.ZTRASHEDSTATE = 0
  AND a.Z_ENT = 3; -- Entity type 3 = Album (check Z_PRIMARYKEY)
```

### 4.4 Top Albums by Size

```sql
SELECT
    a.ZTITLE as album_name,
    COUNT(m.Z_3ASSETS) as photo_count,
    a.ZCREATIONDATE as created_date
FROM ZGENERICALBUM a
LEFT JOIN Z_26ASSETS m ON a.Z_PK = m.Z_26ALBUMS
WHERE a.ZTRASHEDSTATE = 0
GROUP BY a.Z_PK
ORDER BY photo_count DESC
LIMIT 10;
```

### 4.5 Scene Classification Distribution (Top 10)

```sql
-- Most common scene types
SELECT
    ZSCENEIDENTIFIER as scene_id,
    COUNT(*) as occurrence_count
FROM ZSCENECLASSIFICATION
WHERE ZCONFIDENCE > 0.5
GROUP BY ZSCENEIDENTIFIER
ORDER BY occurrence_count DESC
LIMIT 10;
```

**Note:** Scene IDs are numeric. Mapping to human-readable names requires reverse-engineering or using Context7 to query Apple ML documentation.

### 4.6 Face Detection Summary

```sql
-- Photos with detected faces (count only, no identities)
SELECT
    COUNT(DISTINCT ZASSET) as photos_with_faces,
    COUNT(*) as total_faces_detected
FROM ZDETECTEDFACE;
```

### 4.7 Location Data Summary (OPT-IN ONLY)

```sql
-- Photos with GPS coordinates
SELECT COUNT(*) as photos_with_location
FROM ZASSET
WHERE ZTRASHEDSTATE = 0
  AND ZLATITUDE IS NOT NULL
  AND ZLONGITUDE IS NOT NULL
  AND ZLATITUDE != 0
  AND ZLONGITUDE != 0;
```

**Privacy Warning:** Location data reveals user movement patterns. Should be OPT-IN only.

### 4.8 Storage Statistics

```sql
-- Total pixels (rough storage estimate)
SELECT
    SUM(ZHEIGHT * ZWIDTH) as total_pixels,
    AVG(ZHEIGHT * ZWIDTH) as avg_pixels_per_photo
FROM ZASSET
WHERE ZTRASHEDSTATE = 0
  AND ZKIND = 0; -- Photos only
```

---

## 5. Rust Implementation Plan

### 5.1 Dependencies

```toml
[dependencies]
rusqlite = { version = "0.32", features = ["bundled"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
dirs = "5.0"
thiserror = "1.0"
chrono = "0.4"
```

### 5.2 Source Trait Implementation

```rust
use rusqlite::{Connection, OpenFlags, Result as SqliteResult};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

const APPLE_EPOCH_OFFSET: i64 = 978307200; // 2001-01-01 in Unix time

#[derive(Error, Debug)]
pub enum PhotosSourceError {
    #[error("Photos library not found")]
    LibraryNotFound,

    #[error("Permission denied (Full Disk Access required)")]
    PermissionDenied,

    #[error("Database error: {0}")]
    DatabaseError(#[from] rusqlite::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PhotosStats {
    pub library_stats: LibraryStats,
    pub recent_activity: RecentActivity,
    pub albums: AlbumStats,
    pub scene_classification: Option<Vec<SceneCount>>,
    pub faces: Option<FaceStats>,
    pub location: Option<LocationStats>, // OPT-IN
    pub timestamp: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LibraryStats {
    pub total_assets: u32,
    pub photo_count: u32,
    pub video_count: u32,
    pub favorites_count: u32,
    pub earliest_date: i64, // Unix timestamp
    pub latest_date: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RecentActivity {
    pub imports_last_24h: u32,
    pub imports_last_7d: u32,
    pub edits_last_7d: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AlbumStats {
    pub total_albums: u32,
    pub avg_photos_per_album: f64,
    pub top_albums: Vec<AlbumInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AlbumInfo {
    pub name: String,
    pub photo_count: u32,
    pub created_date: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SceneCount {
    pub scene_id: u32,
    pub occurrence_count: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FaceStats {
    pub photos_with_faces: u32,
    pub total_faces_detected: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LocationStats {
    pub photos_with_location: u32,
}

pub struct ApplePhotosSource {
    db_path: PathBuf,
    include_location: bool, // User opt-in
    include_scenes: bool,   // User opt-in (can be large)
    include_faces: bool,    // User opt-in
}

impl ApplePhotosSource {
    pub fn new() -> Result<Self, PhotosSourceError> {
        let home = dirs::home_dir().ok_or(PhotosSourceError::LibraryNotFound)?;
        let db_path = home.join("Pictures/Photos Library.photoslibrary/database/Photos.sqlite");

        if !db_path.exists() {
            return Err(PhotosSourceError::LibraryNotFound);
        }

        Ok(Self {
            db_path,
            include_location: false,
            include_scenes: true,
            include_faces: false,
        })
    }

    pub fn with_location(mut self, enabled: bool) -> Self {
        self.include_location = enabled;
        self
    }

    pub fn with_scenes(mut self, enabled: bool) -> Self {
        self.include_scenes = enabled;
        self
    }

    pub fn with_faces(mut self, enabled: bool) -> Self {
        self.include_faces = enabled;
        self
    }

    fn open_connection(&self) -> SqliteResult<Connection> {
        Connection::open_with_flags(
            &self.db_path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI,
        )
    }

    fn get_library_stats(&self, conn: &Connection) -> SqliteResult<LibraryStats> {
        let mut stmt = conn.prepare(
            "SELECT
                COUNT(*) as total,
                SUM(CASE WHEN ZKIND = 0 THEN 1 ELSE 0 END) as photos,
                SUM(CASE WHEN ZKIND = 1 THEN 1 ELSE 0 END) as videos,
                SUM(CASE WHEN ZFAVORITE = 1 THEN 1 ELSE 0 END) as favorites,
                MIN(ZDATECREATED) as earliest,
                MAX(ZDATECREATED) as latest
             FROM ZASSET
             WHERE ZTRASHEDSTATE = 0"
        )?;

        stmt.query_row([], |row| {
            Ok(LibraryStats {
                total_assets: row.get(0)?,
                photo_count: row.get(1)?,
                video_count: row.get(2)?,
                favorites_count: row.get(3)?,
                earliest_date: APPLE_EPOCH_OFFSET + row.get::<_, i64>(4)?,
                latest_date: APPLE_EPOCH_OFFSET + row.get::<_, i64>(5)?,
            })
        })
    }

    fn get_recent_activity(&self, conn: &Connection) -> SqliteResult<RecentActivity> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let apple_now = now - APPLE_EPOCH_OFFSET;
        let day_ago = apple_now - 86400;
        let week_ago = apple_now - (7 * 86400);

        let imports_24h: u32 = conn.query_row(
            "SELECT COUNT(*) FROM ZASSET WHERE ZTRASHEDSTATE = 0 AND ZADDEDDATE > ?1",
            [day_ago],
            |row| row.get(0),
        )?;

        let imports_7d: u32 = conn.query_row(
            "SELECT COUNT(*) FROM ZASSET WHERE ZTRASHEDSTATE = 0 AND ZADDEDDATE > ?1",
            [week_ago],
            |row| row.get(0),
        )?;

        let edits_7d: u32 = conn.query_row(
            "SELECT COUNT(DISTINCT ZASSET) FROM ZUNMANAGEDADJUSTMENT WHERE ZADJUSTMENTTIMESTAMP > ?1",
            [week_ago],
            |row| row.get(0),
        )?;

        Ok(RecentActivity {
            imports_last_24h: imports_24h,
            imports_last_7d: imports_7d,
            edits_last_7d: edits_7d,
        })
    }

    fn get_album_stats(&self, conn: &Connection) -> SqliteResult<AlbumStats> {
        // Get total albums
        let total_albums: u32 = conn.query_row(
            "SELECT COUNT(*) FROM ZGENERICALBUM WHERE ZTRASHEDSTATE = 0 AND Z_ENT = 3",
            [],
            |row| row.get(0),
        )?;

        // Get average photos per album
        let avg_photos: f64 = conn.query_row(
            "SELECT AVG(count) FROM (
                SELECT COUNT(*) as count
                FROM Z_26ASSETS
                GROUP BY Z_26ALBUMS
            )",
            [],
            |row| row.get(0),
        ).unwrap_or(0.0);

        // Get top 10 albums
        let mut stmt = conn.prepare(
            "SELECT a.ZTITLE, COUNT(m.Z_3ASSETS), a.ZCREATIONDATE
             FROM ZGENERICALBUM a
             LEFT JOIN Z_26ASSETS m ON a.Z_PK = m.Z_26ALBUMS
             WHERE a.ZTRASHEDSTATE = 0
             GROUP BY a.Z_PK
             ORDER BY COUNT(m.Z_3ASSETS) DESC
             LIMIT 10"
        )?;

        let top_albums = stmt.query_map([], |row| {
            Ok(AlbumInfo {
                name: row.get(0)?,
                photo_count: row.get(1)?,
                created_date: APPLE_EPOCH_OFFSET + row.get::<_, i64>(2)?,
            })
        })?.collect::<Result<Vec<_>, _>>()?;

        Ok(AlbumStats {
            total_albums,
            avg_photos_per_album: avg_photos,
            top_albums,
        })
    }

    fn get_scene_classification(&self, conn: &Connection) -> SqliteResult<Vec<SceneCount>> {
        if !self.include_scenes {
            return Ok(vec![]);
        }

        let mut stmt = conn.prepare(
            "SELECT ZSCENEIDENTIFIER, COUNT(*) as count
             FROM ZSCENECLASSIFICATION
             WHERE ZCONFIDENCE > 0.5
             GROUP BY ZSCENEIDENTIFIER
             ORDER BY count DESC
             LIMIT 20"
        )?;

        stmt.query_map([], |row| {
            Ok(SceneCount {
                scene_id: row.get(0)?,
                occurrence_count: row.get(1)?,
            })
        })?.collect::<Result<Vec<_>, _>>()
    }

    fn get_face_stats(&self, conn: &Connection) -> SqliteResult<Option<FaceStats>> {
        if !self.include_faces {
            return Ok(None);
        }

        let mut stmt = conn.prepare(
            "SELECT COUNT(DISTINCT ZASSET), COUNT(*)
             FROM ZDETECTEDFACE"
        )?;

        stmt.query_row([], |row| {
            Ok(Some(FaceStats {
                photos_with_faces: row.get(0)?,
                total_faces_detected: row.get(1)?,
            }))
        })
    }

    fn get_location_stats(&self, conn: &Connection) -> SqliteResult<Option<LocationStats>> {
        if !self.include_location {
            return Ok(None);
        }

        let count: u32 = conn.query_row(
            "SELECT COUNT(*) FROM ZASSET
             WHERE ZTRASHEDSTATE = 0
               AND ZLATITUDE IS NOT NULL
               AND ZLONGITUDE IS NOT NULL
               AND ZLATITUDE != 0
               AND ZLONGITUDE != 0",
            [],
            |row| row.get(0),
        )?;

        Ok(Some(LocationStats {
            photos_with_location: count,
        }))
    }

    pub fn collect_stats(&self) -> Result<PhotosStats, PhotosSourceError> {
        let conn = self.open_connection()
            .map_err(|e| {
                if e.to_string().contains("unable to open") {
                    PhotosSourceError::PermissionDenied
                } else {
                    PhotosSourceError::DatabaseError(e)
                }
            })?;

        Ok(PhotosStats {
            library_stats: self.get_library_stats(&conn)?,
            recent_activity: self.get_recent_activity(&conn)?,
            albums: self.get_album_stats(&conn)?,
            scene_classification: if self.include_scenes {
                Some(self.get_scene_classification(&conn)?)
            } else {
                None
            },
            faces: self.get_face_stats(&conn)?,
            location: self.get_location_stats(&conn)?,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
        })
    }
}

impl Source for ApplePhotosSource {
    fn id(&self) -> &str {
        "apple-photos"
    }

    fn name(&self) -> &str {
        "Apple Photos Library Statistics"
    }

    fn watch_path(&self) -> Option<PathBuf> {
        // Watch the database directory for changes
        Some(self.db_path.parent().unwrap().to_path_buf())
    }

    fn parse(&self) -> Result<serde_json::Value, SourceError> {
        let stats = self.collect_stats()
            .map_err(|e| SourceError::ParseError(e.to_string()))?;

        serde_json::to_value(stats)
            .map_err(|e| SourceError::ParseError(e.to_string()))
    }

    fn preview(&self) -> Result<SourcePreview, SourceError> {
        let stats = self.collect_stats()
            .map_err(|e| SourceError::ParseError(e.to_string()))?;

        Ok(SourcePreview {
            title: format!("Photos Library: {} assets", stats.library_stats.total_assets),
            summary: format!(
                "{} photos, {} videos, {} albums",
                stats.library_stats.photo_count,
                stats.library_stats.video_count,
                stats.albums.total_albums
            ),
            fields: vec![
                ("Recent imports (24h)".to_string(), stats.recent_activity.imports_last_24h.to_string()),
                ("Recent imports (7d)".to_string(), stats.recent_activity.imports_last_7d.to_string()),
                ("Favorites".to_string(), stats.library_stats.favorites_count.to_string()),
            ],
        })
    }
}
```

---

## 6. Sample JSON Payload

```json
{
  "library_stats": {
    "total_assets": 8542,
    "photo_count": 7893,
    "video_count": 649,
    "favorites_count": 234,
    "earliest_date": 1420070400,
    "latest_date": 1738800000
  },
  "recent_activity": {
    "imports_last_24h": 12,
    "imports_last_7d": 87,
    "edits_last_7d": 5
  },
  "albums": {
    "total_albums": 42,
    "avg_photos_per_album": 127.3,
    "top_albums": [
      {
        "name": "Vacation 2025",
        "photo_count": 456,
        "created_date": 1735689600
      },
      {
        "name": "Family",
        "photo_count": 389,
        "created_date": 1609459200
      },
      {
        "name": "Work Events",
        "photo_count": 234,
        "created_date": 1672531200
      }
    ]
  },
  "scene_classification": [
    {
      "scene_id": 2084,
      "occurrence_count": 1247
    },
    {
      "scene_id": 1056,
      "occurrence_count": 893
    },
    {
      "scene_id": 3421,
      "occurrence_count": 672
    }
  ],
  "faces": {
    "photos_with_faces": 3421,
    "total_faces_detected": 8934
  },
  "location": null,
  "timestamp": 1738838400
}
```

---

## 7. Privacy & Risk Assessment

### 7.1 Privacy Tiers

| Data Category | Privacy Risk | Mitigation |
|--------------|--------------|------------|
| **Library totals** | LOW | Aggregate counts only, no identifying info |
| **Recent activity** | LOW | Time-based counts, no content |
| **Album names** | MEDIUM | User-created names may contain personal info |
| **Scene classification** | MEDIUM | Reveals activities (beach, food, indoor, etc.) |
| **Face detection counts** | MEDIUM-HIGH | Reveals social patterns (photos with people) |
| **Location data** | HIGH | Reveals movement patterns, home/work locations |

### 7.2 Required User Consent

**Before enabling this source, users should consent to:**

1. ✓ Full Disk Access to read Photos library database
2. ✓ Aggregate statistics collection (counts, dates, album names)
3. ⚠️ **OPT-IN:** Scene classification data (reveals photo content categories)
4. ⚠️ **OPT-IN:** Face detection counts (reveals social patterns)
5. ⚠️ **OPT-IN:** Location data (HIGH RISK - reveals movement patterns)

### 7.3 Recommended Defaults

```rust
ApplePhotosSource::new()
    .with_scenes(false)      // OFF by default
    .with_faces(false)       // OFF by default
    .with_location(false)    // OFF by default (NEVER auto-enable)
```

### 7.4 Data Retention

**Webhook delivery only** - LocalPush should NOT store this data locally beyond:
- Temporary processing buffer
- Error logs (sanitized, no personal data)

### 7.5 Privacy Disclosure Example

> **Apple Photos Source**
>
> This source reads statistics from your Photos library database. It does NOT access image files or send image content.
>
> **What we collect:**
> - Total photo/video counts
> - Album names and sizes
> - Recent import activity
> - Favorite counts
>
> **Optional data (disabled by default):**
> - [ ] Scene classification (e.g., "beach", "food", "indoor")
> - [ ] Face detection counts (number of detected faces, not identities)
> - [ ] Location data (GPS coordinates presence - HIGH PRIVACY RISK)
>
> **Permissions required:** Full Disk Access

---

## 8. Known Limitations & Gotchas

### 8.1 Database Schema Variations

**Issue:** macOS Photos database schema changes between OS versions.

**Mitigation:**
- Test on macOS Sonoma (14.x), Sequoia (15.x), and future versions
- Use schema introspection to detect table/column existence before querying
- Gracefully handle missing tables/columns

```rust
fn table_exists(conn: &Connection, table_name: &str) -> bool {
    conn.prepare(&format!(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='{}'",
        table_name
    ))
    .and_then(|mut stmt| stmt.exists([]))
    .unwrap_or(false)
}
```

### 8.2 Timestamp Conversions

**Issue:** Apple Core Data uses 2001-01-01 epoch, not Unix epoch (1970-01-01).

**Solution:** Always convert:
```rust
const APPLE_EPOCH_OFFSET: i64 = 978307200;
unix_timestamp = apple_date + APPLE_EPOCH_OFFSET;
```

### 8.3 Scene ID Mapping

**Issue:** `ZSCENEIDENTIFIER` values are numeric codes with no built-in name mapping.

**Options:**
1. Return raw IDs (let webhook consumer map them)
2. Reverse-engineer mapping from Apple frameworks (requires significant research)
3. Use Context7 to query Apple ML documentation for scene type definitions

**Recommendation:** Return raw IDs initially. Document known mappings as discovered.

### 8.4 Album Relationship Table Names

**Issue:** The many-to-many table name varies (`Z_26ASSETS`, `Z_27ASSETS`, etc.) based on entity types.

**Solution:** Query `Z_PRIMARYKEY` table to find correct entity ID:
```sql
SELECT Z_ENT FROM Z_PRIMARYKEY WHERE Z_NAME = 'GenericAlbum';
-- Result: 26 (use Z_26ASSETS table)
```

### 8.5 WAL Mode File Watching

**Issue:** Photos.sqlite uses WAL (Write-Ahead Logging). Changes appear in `-wal` file first.

**Solution:** Watch both `Photos.sqlite` and `Photos.sqlite-wal` for changes:
```rust
fn watch_path(&self) -> Option<PathBuf> {
    Some(self.db_path.parent().unwrap().to_path_buf())
}
```

### 8.6 Concurrent Read Performance

**Issue:** Large libraries (10k+ photos) can make queries slow.

**Mitigation:**
- Use indexed columns (ZTRASHEDSTATE, ZKIND, ZADDEDDATE)
- Limit aggregations to necessary ranges
- Consider caching results and only querying deltas

### 8.7 iCloud Photos Behavior

**Issue:** If iCloud Photos is enabled, database may contain placeholders for non-downloaded assets.

**Impact:** Counts may include assets not physically on device.

**Detection:** Check `ZCLOUDMASTERTYPE` column in ZASSET (0=local, non-zero=cloud)

### 8.8 Face Detection Timing

**Issue:** `ZDETECTEDFACE` table is populated asynchronously by `photoanalysisd` daemon.

**Impact:** Newly imported photos won't have face data immediately.

**Workaround:** Document that face stats are "as of last analysis" and may lag.

### 8.9 Locked Database While Photos App Running

**Status:** NOT an issue for read-only access.

**Confirmed:** SQLite WAL mode allows concurrent readers even when writer (Photos.app) has lock.

---

## 9. File Watching Strategy

### Watch Target

**Primary:** Directory containing database files
```
~/Pictures/Photos Library.photoslibrary/database/
```

**Files to monitor:**
- `Photos.sqlite` - Main database
- `Photos.sqlite-wal` - Write-ahead log (changes appear here first)
- `Photos.sqlite-shm` - Shared memory

### Trigger Conditions

Re-parse and send webhook when:
1. `Photos.sqlite-wal` file size changes
2. `Photos.sqlite` modification time updates
3. Any checkpoint operation (WAL merged into main DB)

### Debouncing

**Issue:** Photos.app writes frequently during analysis/imports.

**Solution:** Debounce file change events:
```rust
let mut last_parse = Instant::now();
const DEBOUNCE_SECS: u64 = 60;

if last_parse.elapsed().as_secs() >= DEBOUNCE_SECS {
    self.parse_and_send();
    last_parse = Instant::now();
}
```

### Rate Limiting

**Recommendation:** Maximum 1 parse per minute to avoid hammering database during bulk imports.

---

## 10. Testing Strategy

### 10.1 Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apple_date_conversion() {
        let apple_date = 639187200.0; // 2021-04-01 00:00:00 UTC
        let unix_ts = apple_date_to_unix(apple_date);
        assert_eq!(unix_ts, 1617235200);
    }

    #[test]
    fn test_library_stats_parsing() {
        // Use test fixture database
        let source = ApplePhotosSource::new().unwrap();
        let stats = source.collect_stats().unwrap();
        assert!(stats.library_stats.total_assets > 0);
    }
}
```

### 10.2 Integration Tests

**Test with real Photos library:**
1. Create test library with known counts
2. Import 10 photos, 5 videos
3. Create 3 albums with specific names
4. Mark 2 favorites
5. Run source.parse()
6. Verify counts match

### 10.3 Permission Testing

**Verify graceful degradation:**
1. Run without Full Disk Access → expect `PermissionDenied` error
2. Grant FDA → expect successful parse
3. Remove Photos library → expect `LibraryNotFound` error

### 10.4 Performance Testing

**Large library stress test:**
- Test with 50k+ photo library
- Measure parse time (target: <5 seconds)
- Check memory usage (target: <50MB)

---

## 11. Future Enhancements

### 11.1 Scene ID to Name Mapping

Research Apple's scene classification ontology to provide human-readable labels:
```json
"scene_classification": [
  {
    "scene_id": 2084,
    "scene_name": "Beach/Ocean",
    "occurrence_count": 1247
  }
]
```

### 11.2 Smart Albums Support

Detect and report smart album criteria (e.g., "Recently Added", "Favorites", "Selfies").

### 11.3 Memories Support

Apple's Memories feature stores curated collections in database. Could expose:
- Memory count
- Featured memories
- Memory themes

### 11.4 Shared Albums

Track shared album membership and cloud sync status.

### 11.5 Delta Updates

Instead of full stats on every change, send only deltas:
```json
{
  "delta": {
    "new_imports": 5,
    "new_albums": ["Summer 2026"],
    "deleted_photos": 2
  }
}
```

---

## 12. References

### Research Sources

1. **Local Photo Library Photos.sqlite Query Documentation** - [The Forensic Scooter](https://theforensicscooter.com/2022/05/02/photos-sqlite-query-documentation-notable-artifacts/)
   - Comprehensive ZASSET, ZADDITIONALASSETATTRIBUTES column documentation
   - SQL query examples for forensic analysis

2. **iOS Photos.sqlite Queries** - [GitHub: AndrewRathbun](https://github.com/AndrewRathbun/iOS_Photos.sqlite_Queries)
   - Version-specific query collections (iOS 11-16, macOS compatibility)
   - Timestamp handling, date range filtering

3. **Apple Photos Forensics** - [GitHub: muxcmux](https://github.com/muxcmux/apple-photos-forensics)
   - Database structure reverse engineering
   - Table relationships, entity types

4. **rust-apple-photos Library** - [GitHub: dangreco](https://github.com/dangreco/rust-apple-photos)
   - Existing Rust implementation (archived but useful reference)
   - RKMaster, RKAlbum models

5. **macOS TCC Deep Dive** - [Rainforest QA Blog](https://www.rainforestqa.com/blog/macos-tcc-db-deep-dive)
   - TCC database structure, Full Disk Access requirements
   - Permission grant detection

6. **Apple Machine Learning Research: On-Device Scene Analysis** - [Apple ML Research](https://machinelearning.apple.com/research/on-device-scene-analysis)
   - ANSA (Apple Neural Scene Analyzer) architecture
   - Privacy-preserving ML design

7. **Photoanalysisd Daemon** - [MacKeeper Blog](https://mackeeper.com/blog/photoanalysisd-on-mac/)
   - Background analysis process behavior
   - Face detection, scene classification timing

8. **SQLite WAL Mode** - [SQLite Documentation](https://www.sqlite.org/wal.html)
   - Write-Ahead Logging mechanics
   - Concurrent read capabilities

### Additional Resources

- **Photos.sqlite ZINTERNALRESOURCE Table Reference** - [The Forensic Scooter](https://theforensicscooter.com/2022/12/03/photos-sqlite-zinternalresource-table-reference-guide/)
- **macOS TCC HackTricks** - [HackTricks Wiki](https://book.hacktricks.wiki/en/macos-hardening/macos-security-and-privilege-escalation/macos-security-protections/macos-tcc/index.html)
- **Simon Willison: Using SQL to find my best photo of a pelican** - [Blog Post](https://simonwillison.net/2020/May/21/dogsheep-photos/)

---

## 13. Implementation Checklist

- [ ] Set up Rust dependencies (rusqlite, serde, dirs)
- [ ] Implement ApplePhotosSource struct with Source trait
- [ ] Add Full Disk Access entitlement to Info.plist
- [ ] Implement library stats SQL query
- [ ] Implement recent activity SQL query
- [ ] Implement album stats SQL query
- [ ] Add opt-in scene classification query
- [ ] Add opt-in face detection query
- [ ] Add opt-in location query
- [ ] Implement Apple date to Unix timestamp conversion
- [ ] Add database existence check
- [ ] Add permission check with graceful error
- [ ] Implement file watcher for database directory
- [ ] Add debouncing for file change events
- [ ] Create unit tests for date conversion
- [ ] Create integration tests with test library
- [ ] Test on macOS Sonoma and Sequoia
- [ ] Add privacy disclosure UI
- [ ] Document opt-in settings
- [ ] Performance test with large library (50k+ photos)
- [ ] Handle schema variations between macOS versions
- [ ] Add logging for debugging
- [ ] Create sample webhook payload documentation

---

## Conclusion

This implementation plan provides a complete roadmap for building a privacy-first Apple Photos statistics source for LocalPush. The focus on **metadata and aggregates only** (no image content) keeps the privacy risk manageable while still providing valuable insights into library activity.

**Key Success Factors:**
1. Read-only SQLite access works even while Photos.app is running
2. Full Disk Access is the only required permission
3. Opt-in model for sensitive data (location, faces, scenes)
4. Graceful error handling for missing permissions
5. Performance-tested on large libraries

**Next Steps:**
1. Prototype basic library stats collection
2. Test on real Photos library with known counts
3. Iterate on SQL queries for performance
4. Add opt-in features incrementally
5. User test privacy disclosure UX
