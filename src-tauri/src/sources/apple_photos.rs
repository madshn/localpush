use super::{PreviewField, Source, SourceError, SourcePreview};
use chrono::{DateTime, Utc};
use rusqlite::{Connection, OpenFlags};
use std::path::PathBuf;
use tracing::{debug, info, warn};

/// Seconds between Unix epoch (1970-01-01) and Core Data epoch (2001-01-01).
const CORE_DATA_EPOCH_OFFSET: f64 = 978_307_200.0;

/// Aggregated library statistics from the Photos database.
#[derive(Debug, serde::Serialize)]
struct LibraryStats {
    total_photos: i64,
    total_videos: i64,
    favorites: i64,
    recent_imports: i64,
    albums: i64,
}

/// Apple Photos library source.
///
/// Reads aggregate statistics (counts only) from the Photos SQLite database.
/// Requires Full Disk Access (TCC permission) on macOS.
/// Never exposes individual photo details, filenames, or locations.
pub struct ApplePhotosSource {
    db_path: PathBuf,
}

impl ApplePhotosSource {
    pub fn new() -> Result<Self, SourceError> {
        let home = std::env::var("HOME")
            .map_err(|_| SourceError::ParseError("HOME not set".to_string()))?;

        let db_path = PathBuf::from(home)
            .join("Pictures/Photos Library.photoslibrary/database/Photos.sqlite");

        Ok(Self { db_path })
    }

    /// Constructor with custom path (for testing)
    pub fn new_with_path(path: impl Into<PathBuf>) -> Self {
        Self {
            db_path: path.into(),
        }
    }

    /// Open the Photos database in read-only mode.
    fn open_db(&self) -> Result<Connection, SourceError> {
        if !self.db_path.exists() {
            warn!("Photos database not found at: {}", self.db_path.display());
            return Err(SourceError::FileNotFound(self.db_path.clone()));
        }

        debug!("Opening Photos database: {}", self.db_path.display());

        Connection::open_with_flags(
            &self.db_path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(|e| SourceError::ParseError(format!("SQLite: {}", e)))
    }

    /// Convert a Core Data timestamp to an ISO 8601 string.
    ///
    /// Core Data stores timestamps as seconds since 2001-01-01.
    /// Adding `CORE_DATA_EPOCH_OFFSET` yields a Unix timestamp.
    #[allow(dead_code)]
    fn core_data_to_iso(timestamp: f64) -> String {
        let unix_ts = timestamp + CORE_DATA_EPOCH_OFFSET;
        DateTime::from_timestamp(unix_ts as i64, 0)
            .map(|dt| dt.to_rfc3339())
            .unwrap_or_default()
    }

    /// Query aggregate statistics from the Photos database.
    fn query_library_stats(&self) -> Result<LibraryStats, SourceError> {
        let conn = self.open_db()?;

        let total_photos: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM ZASSET WHERE ZKIND = 0 AND ZTRASHEDSTATE = 0",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let total_videos: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM ZASSET WHERE ZKIND = 1 AND ZTRASHEDSTATE = 0",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let favorites: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM ZASSET WHERE ZFAVORITE = 1 AND ZTRASHEDSTATE = 0",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        // Recent imports (last 7 days) using Core Data epoch
        let cutoff = (Utc::now().timestamp() as f64) - CORE_DATA_EPOCH_OFFSET - 86400.0 * 7.0;
        let recent_imports: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM ZASSET WHERE ZADDEDDATE > ?1 AND ZTRASHEDSTATE = 0",
                [cutoff],
                |row| row.get(0),
            )
            .unwrap_or(0);

        // User albums only (ZKIND = 2 is a regular album)
        let albums: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM ZGENERICALBUM WHERE ZKIND = 2",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        info!(
            "Photos library: {} photos, {} videos, {} favorites",
            total_photos, total_videos, favorites
        );

        Ok(LibraryStats {
            total_photos,
            total_videos,
            favorites,
            recent_imports,
            albums,
        })
    }

    /// Format a number with comma separators (e.g. 12345 -> "12,345")
    fn format_number(n: i64) -> String {
        let abs = n.unsigned_abs().to_string();
        let formatted = abs
            .as_bytes()
            .rchunks(3)
            .rev()
            .map(std::str::from_utf8)
            .collect::<Result<Vec<&str>, _>>()
            .unwrap()
            .join(",");

        if n < 0 {
            format!("-{}", formatted)
        } else {
            formatted
        }
    }
}

impl Source for ApplePhotosSource {
    fn id(&self) -> &str {
        "apple-photos"
    }

    fn name(&self) -> &str {
        "Apple Photos"
    }

    fn watch_path(&self) -> Option<PathBuf> {
        self.db_path.parent().map(|p| p.to_path_buf())
    }

    fn parse(&self) -> Result<serde_json::Value, SourceError> {
        let stats = self.query_library_stats()?;

        Ok(serde_json::json!({
            "source": "apple_photos",
            "timestamp": Utc::now().to_rfc3339(),
            "library": {
                "total_photos": stats.total_photos,
                "total_videos": stats.total_videos,
                "total_assets": stats.total_photos + stats.total_videos,
                "favorites": stats.favorites,
                "recent_imports_7d": stats.recent_imports,
                "albums": stats.albums,
            }
        }))
    }

    fn preview(&self) -> Result<SourcePreview, SourceError> {
        let stats = self.query_library_stats()?;
        let total = stats.total_photos + stats.total_videos;

        let summary = format!(
            "{} photos, {} videos",
            Self::format_number(stats.total_photos),
            Self::format_number(stats.total_videos),
        );

        let fields = vec![
            PreviewField {
                label: "Photos".to_string(),
                value: Self::format_number(stats.total_photos),
                sensitive: false,
            },
            PreviewField {
                label: "Videos".to_string(),
                value: Self::format_number(stats.total_videos),
                sensitive: false,
            },
            PreviewField {
                label: "Total Assets".to_string(),
                value: Self::format_number(total),
                sensitive: false,
            },
            PreviewField {
                label: "Favorites".to_string(),
                value: Self::format_number(stats.favorites),
                sensitive: false,
            },
            PreviewField {
                label: "Recent Imports (7d)".to_string(),
                value: Self::format_number(stats.recent_imports),
                sensitive: false,
            },
            PreviewField {
                label: "Albums".to_string(),
                value: Self::format_number(stats.albums),
                sensitive: false,
            },
        ];

        let last_updated = std::fs::metadata(&self.db_path)
            .ok()
            .and_then(|m| m.modified().ok())
            .map(DateTime::<Utc>::from);

        Ok(SourcePreview {
            title: self.name().to_string(),
            summary,
            fields,
            last_updated,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_core_data_timestamp() {
        // 2026-01-01 00:00:00 UTC in Core Data epoch = 788918400.0
        let iso = ApplePhotosSource::core_data_to_iso(788_918_400.0);
        assert!(iso.starts_with("2026-"));
    }

    #[test]
    fn test_core_data_timestamp_zero() {
        // Core Data epoch zero = 2001-01-01 00:00:00 UTC
        let iso = ApplePhotosSource::core_data_to_iso(0.0);
        assert!(iso.starts_with("2001-01-01"));
    }

    #[test]
    fn test_source_trait_impl() {
        let source = ApplePhotosSource::new_with_path("/tmp/fake-photos.sqlite");
        assert_eq!(source.id(), "apple-photos");
        assert_eq!(source.name(), "Apple Photos");
    }

    #[test]
    fn test_watch_path_is_parent_dir() {
        let source = ApplePhotosSource::new_with_path("/tmp/database/Photos.sqlite");
        assert_eq!(source.watch_path(), Some(PathBuf::from("/tmp/database")));
    }

    #[test]
    fn test_missing_db_returns_file_not_found() {
        let source = ApplePhotosSource::new_with_path("/tmp/nonexistent-photos.sqlite");
        let err = source.parse().unwrap_err();
        assert!(matches!(err, SourceError::FileNotFound(_)));
    }

    #[test]
    fn test_format_number() {
        assert_eq!(ApplePhotosSource::format_number(0), "0");
        assert_eq!(ApplePhotosSource::format_number(999), "999");
        assert_eq!(ApplePhotosSource::format_number(1_234), "1,234");
        assert_eq!(ApplePhotosSource::format_number(12_345), "12,345");
        assert_eq!(ApplePhotosSource::format_number(1_234_567), "1,234,567");
    }
}
