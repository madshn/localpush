use super::{PreviewField, Source, SourceError, SourcePreview};
use crate::source_config::PropertyDef;
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

/// Metadata for a single photo from the Photos library.
#[derive(Debug, serde::Serialize)]
struct PhotoMetadata {
    uuid: String,
    filename: Option<String>,
    date_created: Option<String>,
    date_added: String,
    photo_type: String,
    latitude: Option<f64>,
    longitude: Option<f64>,
    faces: Vec<String>,
    labels: Vec<String>,
}

/// A face detected in a photo.
#[derive(Debug)]
struct DetectedFace {
    asset_id: i64,
    person_name: String,
}

#[derive(Debug)]
struct FaceQueryColumns {
    asset_col: String,
    person_fk_col: String,
    person_name_col: String,
}

/// An ML-generated label for a photo.
#[derive(Debug)]
struct PhotoLabel {
    content: String,
}

/// Apple Photos library source.
///
/// Reads library statistics and recent photo metadata from the Photos SQLite database.
/// Requires Full Disk Access (TCC permission) on macOS.
/// Privacy-sensitive properties (locations, faces) are user-configurable.
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
        .map_err(|e| {
            let err_msg = e.to_string();
            // Detect permission denied errors
            if err_msg.contains("unable to open database")
                || err_msg.contains("disk I/O error")
                || err_msg.contains("attempt to write a readonly database")
            {
                warn!("Permission denied accessing Photos database");
                SourceError::PermissionDenied(
                    "Cannot access Apple Photos library".to_string()
                )
            } else {
                SourceError::ParseError(format!("SQLite: {}", e))
            }
        })
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

    /// Check if a uniform type identifier indicates a screenshot.
    fn is_screenshot(uti: &str) -> bool {
        uti.contains("screenshot")
            || uti.contains("public.png") && uti.contains("screen")
    }

    /// Map photo kind and subtype to a human-readable string.
    fn photo_subtype(kind: i32, subtype: i32) -> &'static str {
        match (kind, subtype) {
            (0, 0) => "normal",
            (0, 1) => "panorama",
            (1, 2) => "slo-mo",
            (1, 3) => "timelapse",
            (0, 16) => "burst",
            _ => "other",
        }
    }

    /// Detect the filename column in ZASSET (varies across macOS versions).
    fn detect_filename_column(conn: &Connection) -> Option<String> {
        let mut stmt = conn.prepare("PRAGMA table_info(ZASSET)").ok()?;
        let columns: Vec<String> = stmt
            .query_map([], |row| row.get::<_, String>(1))
            .ok()?
            .filter_map(|r| r.ok())
            .collect();

        // Prefer ZORIGINALFILENAME (older macOS), fall back to ZFILENAME (newer)
        for candidate in &["ZORIGINALFILENAME", "ZFILENAME"] {
            if columns.iter().any(|c| c == candidate) {
                return Some((*candidate).to_string());
            }
        }
        None
    }

    /// Detect face/person column names (varies across macOS versions).
    fn detect_face_query_columns(conn: &Connection) -> Option<FaceQueryColumns> {
        let mut face_stmt = conn.prepare("PRAGMA table_info(ZDETECTEDFACE)").ok()?;
        let face_columns: Vec<String> = face_stmt
            .query_map([], |row| row.get::<_, String>(1))
            .ok()?
            .filter_map(|r| r.ok())
            .collect();

        let mut person_stmt = conn.prepare("PRAGMA table_info(ZPERSON)").ok()?;
        let person_columns: Vec<String> = person_stmt
            .query_map([], |row| row.get::<_, String>(1))
            .ok()?
            .filter_map(|r| r.ok())
            .collect();

        let asset_col = ["ZASSET", "ZASSETFORFACE"]
            .into_iter()
            .find(|candidate| face_columns.iter().any(|c| c == candidate))?
            .to_string();

        let person_fk_col = ["ZPERSON", "ZPERSONFORFACE"]
            .into_iter()
            .find(|candidate| face_columns.iter().any(|c| c == candidate))?
            .to_string();

        let person_name_col = ["ZFULLNAME", "ZDISPLAYNAME", "ZNAME"]
            .into_iter()
            .find(|candidate| person_columns.iter().any(|c| c == candidate))?
            .to_string();

        Some(FaceQueryColumns {
            asset_col,
            person_fk_col,
            person_name_col,
        })
    }

    /// Query recent photos (added in the last 7 days) with their metadata.
    fn query_recent_photos(&self) -> Result<Vec<PhotoMetadata>, SourceError> {
        let conn = self.open_db()?;

        let cutoff = (Utc::now().timestamp() as f64) - CORE_DATA_EPOCH_OFFSET - 86400.0 * 7.0;

        // Detect available filename column (schema varies across macOS versions)
        let filename_col = Self::detect_filename_column(&conn);
        let filename_expr = filename_col.as_deref().unwrap_or("NULL");

        let query = format!(
            "SELECT Z_PK, ZUUID, {}, ZDATECREATED, ZADDEDDATE,
                    ZKIND, ZKINDSUBTYPE, ZUNIFORMTYPEIDENTIFIER,
                    ZLATITUDE, ZLONGITUDE
             FROM ZASSET
             WHERE ZADDEDDATE > ?1
               AND ZTRASHEDSTATE = 0
             ORDER BY ZADDEDDATE DESC
             LIMIT 50",
            filename_expr
        );

        let mut stmt = conn
            .prepare(&query)
            .map_err(|e| SourceError::ParseError(format!("Photo query prepare: {}", e)))?;

        let mut photos = Vec::new();
        let rows = stmt
            .query_map([cutoff], |row| {
                let pk: i64 = row.get(0)?;
                let uuid: String = row.get::<_, String>(1).unwrap_or_default();
                let filename: Option<String> = row.get(2).ok();
                let date_created: Option<f64> = row.get(3).ok();
                let date_added: f64 = row.get(4)?;
                let kind: i32 = row.get(5).unwrap_or(0);
                let subtype: i32 = row.get(6).unwrap_or(0);
                let uti: String = row.get::<_, String>(7).unwrap_or_default();
                let latitude: Option<f64> = row.get(8).ok();
                let longitude: Option<f64> = row.get(9).ok();

                let photo_type = if Self::is_screenshot(&uti) {
                    "screenshot".to_string()
                } else {
                    Self::photo_subtype(kind, subtype).to_string()
                };

                Ok((
                    pk,
                    PhotoMetadata {
                        uuid,
                        filename,
                        date_created: date_created.map(Self::core_data_to_iso),
                        date_added: Self::core_data_to_iso(date_added),
                        photo_type,
                        latitude,
                        longitude,
                        faces: Vec::new(), // Populated later
                        labels: Vec::new(), // Populated later
                    },
                ))
            })
            .map_err(|e| SourceError::ParseError(format!("Photo query: {}", e)))?;

        let mut asset_ids = Vec::new();
        for (pk, photo) in rows.flatten() {
            asset_ids.push(pk);
            photos.push(photo);
        }

        if !asset_ids.is_empty() {
            // Query faces for these photos
            match self.query_faces(&conn, &asset_ids) {
                Ok(faces) => {
                    for face in faces {
                        // Find the index of the matching asset
                        if let Some(idx) = asset_ids
                            .iter()
                            .position(|&id| id == face.asset_id)
                        {
                            if let Some(photo) = photos.get_mut(idx) {
                                photo.faces.push(face.person_name);
                            }
                        }
                    }
                }
                Err(err) => {
                    warn!("Failed to load face metadata, continuing without faces: {}", err);
                }
            }

            // Query ML labels
            let labels = self.query_labels(&asset_ids)?;
            for (idx, photo) in photos.iter_mut().enumerate() {
                if let Some(asset_id) = asset_ids.get(idx) {
                    photo.labels = labels
                        .iter()
                        .filter(|(id, _)| id == asset_id)
                        .map(|(_, label)| label.content.clone())
                        .collect();
                }
            }
        }

        info!("Loaded {} recent photos with metadata", photos.len());

        Ok(photos)
    }

    /// Query detected faces for the given asset IDs.
    fn query_faces(
        &self,
        conn: &Connection,
        asset_ids: &[i64],
    ) -> Result<Vec<DetectedFace>, SourceError> {
        if asset_ids.is_empty() {
            return Ok(Vec::new());
        }

        let placeholders = asset_ids
            .iter()
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(",");

        let Some(cols) = Self::detect_face_query_columns(conn) else {
            debug!("Face/person schema columns not found, skipping detected faces");
            return Ok(Vec::new());
        };

        let query = format!(
            "SELECT df.{asset_col}, p.{person_name_col}
             FROM ZDETECTEDFACE df
             LEFT JOIN ZPERSON p ON p.Z_PK = df.{person_fk_col}
             WHERE df.{asset_col} IN ({placeholders})
               AND p.{person_name_col} IS NOT NULL",
            asset_col = cols.asset_col,
            person_fk_col = cols.person_fk_col,
            person_name_col = cols.person_name_col,
            placeholders = placeholders
        );

        let mut stmt = conn
            .prepare(&query)
            .map_err(|e| SourceError::ParseError(format!("Face query prepare: {}", e)))?;

        let params: Vec<&dyn rusqlite::ToSql> =
            asset_ids.iter().map(|id| id as &dyn rusqlite::ToSql).collect();

        let rows = stmt
            .query_map(&params[..], |row| {
                Ok(DetectedFace {
                    asset_id: row.get(0)?,
                    person_name: row.get(1)?,
                })
            })
            .map_err(|e| SourceError::ParseError(format!("Face query: {}", e)))?;

        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    /// Query ML labels from psi.sqlite for the given asset IDs.
    fn query_labels(&self, asset_ids: &[i64]) -> Result<Vec<(i64, PhotoLabel)>, SourceError> {
        if asset_ids.is_empty() {
            return Ok(Vec::new());
        }

        // psi.sqlite is in the search subdirectory
        let psi_path = self
            .db_path
            .parent()
            .ok_or_else(|| {
                SourceError::ParseError("Cannot determine Photos database parent".to_string())
            })?
            .join("search/psi.sqlite");

        if !psi_path.exists() {
            debug!("psi.sqlite not found, skipping ML labels");
            return Ok(Vec::new());
        }

        let _psi_conn = Connection::open_with_flags(
            &psi_path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(|e| SourceError::ParseError(format!("psi.sqlite open: {}", e)))?;

        // Note: psi.sqlite uses split UUIDs. For simplicity, we'll skip the UUID
        // mapping for now and return empty labels. A full implementation would
        // require converting ZASSET.ZUUID to psi's uuid_0/uuid_1 format.

        // For now, return empty to avoid complexity without proper UUID mapping
        debug!("ML label extraction requires UUID mapping - not yet implemented");
        Ok(Vec::new())
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
        Some(self.db_path.clone())
    }

    fn parse(&self) -> Result<serde_json::Value, SourceError> {
        let stats = self.query_library_stats()?;
        let recent_photos = self.query_recent_photos()?;

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
            },
            "recent_photos": recent_photos,
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

    fn available_properties(&self) -> Vec<PropertyDef> {
        vec![
            PropertyDef {
                key: "library_stats".to_string(),
                label: "Library Statistics".to_string(),
                description: "Aggregate counts of photos, videos, albums".to_string(),
                default_enabled: true,
                privacy_sensitive: false,
            },
            PropertyDef {
                key: "recent_photos".to_string(),
                label: "Recent Photos".to_string(),
                description: "New photos with metadata (filenames, dates) from the last 7 days".to_string(),
                default_enabled: false,
                privacy_sensitive: true,
            },
            PropertyDef {
                key: "photo_location".to_string(),
                label: "Photo Locations".to_string(),
                description: "GPS coordinates where photos were taken".to_string(),
                default_enabled: false,
                privacy_sensitive: true,
            },
            PropertyDef {
                key: "photo_faces".to_string(),
                label: "Detected Faces".to_string(),
                description: "Names of people detected in photos".to_string(),
                default_enabled: false,
                privacy_sensitive: true,
            },
            PropertyDef {
                key: "photo_labels".to_string(),
                label: "ML Content Labels".to_string(),
                description: "Machine learning tags for photo content".to_string(),
                default_enabled: false,
                privacy_sensitive: true,
            },
        ]
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
    fn test_watch_path_is_database_file() {
        let source = ApplePhotosSource::new_with_path("/tmp/database/Photos.sqlite");
        assert_eq!(
            source.watch_path(),
            Some(PathBuf::from("/tmp/database/Photos.sqlite"))
        );
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

    #[test]
    fn test_is_screenshot() {
        assert!(ApplePhotosSource::is_screenshot("public.png.screenshot"));
        assert!(ApplePhotosSource::is_screenshot("com.apple.screenshot"));
        assert!(!ApplePhotosSource::is_screenshot("public.jpeg"));
        assert!(!ApplePhotosSource::is_screenshot("public.heic"));
    }

    #[test]
    fn test_photo_subtype() {
        assert_eq!(ApplePhotosSource::photo_subtype(0, 0), "normal");
        assert_eq!(ApplePhotosSource::photo_subtype(0, 1), "panorama");
        assert_eq!(ApplePhotosSource::photo_subtype(1, 2), "slo-mo");
        assert_eq!(ApplePhotosSource::photo_subtype(1, 3), "timelapse");
        assert_eq!(ApplePhotosSource::photo_subtype(0, 16), "burst");
        assert_eq!(ApplePhotosSource::photo_subtype(99, 99), "other");
    }

    #[test]
    fn test_available_properties_includes_privacy_flags() {
        let source = ApplePhotosSource::new_with_path("/tmp/test.sqlite");
        let props = source.available_properties();

        // Should have at least library stats, recent photos, and privacy-sensitive properties
        assert!(props.len() >= 3, "Expected at least 3 properties");

        // Location should be marked privacy-sensitive and default disabled
        let location_prop = props.iter().find(|p| p.key == "photo_location");
        assert!(location_prop.is_some(), "photo_location property should exist");
        if let Some(prop) = location_prop {
            assert!(prop.privacy_sensitive, "Location should be privacy sensitive");
            assert!(!prop.default_enabled, "Location should be disabled by default");
        }
    }

    #[test]
    fn test_detect_face_query_columns_legacy_schema() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE ZDETECTEDFACE (ZASSET INTEGER, ZPERSON INTEGER);
             CREATE TABLE ZPERSON (Z_PK INTEGER PRIMARY KEY, ZFULLNAME TEXT);",
        )
        .unwrap();

        let cols = ApplePhotosSource::detect_face_query_columns(&conn).unwrap();
        assert_eq!(cols.asset_col, "ZASSET");
        assert_eq!(cols.person_fk_col, "ZPERSON");
        assert_eq!(cols.person_name_col, "ZFULLNAME");
    }

    #[test]
    fn test_detect_face_query_columns_variant_schema() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE ZDETECTEDFACE (ZASSETFORFACE INTEGER, ZPERSONFORFACE INTEGER);
             CREATE TABLE ZPERSON (Z_PK INTEGER PRIMARY KEY, ZDISPLAYNAME TEXT);",
        )
        .unwrap();

        let cols = ApplePhotosSource::detect_face_query_columns(&conn).unwrap();
        assert_eq!(cols.asset_col, "ZASSETFORFACE");
        assert_eq!(cols.person_fk_col, "ZPERSONFORFACE");
        assert_eq!(cols.person_name_col, "ZDISPLAYNAME");
    }
}
