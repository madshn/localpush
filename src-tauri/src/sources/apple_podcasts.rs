use super::{PreviewField, Source, SourceError, SourcePreview};
use chrono::{DateTime, Utc};
use rusqlite::{Connection, OpenFlags};
use serde::Serialize;
use std::path::PathBuf;
use tracing::{debug, info, warn};

/// Seconds between Unix epoch (1970-01-01) and Core Data epoch (2001-01-01).
const CORE_DATA_EPOCH_OFFSET: f64 = 978_307_200.0;

/// Seven days in seconds, used for the "recent episodes" query window.
const SEVEN_DAYS_SECS: f64 = 86_400.0 * 7.0;

/// Maximum number of recent episodes to return.
const RECENT_EPISODE_LIMIT: u32 = 50;

/// A single played episode with metadata from its parent podcast.
#[derive(Debug, Serialize)]
struct EpisodeInfo {
    episode_title: String,
    podcast_name: String,
    duration_seconds: Option<f64>,
    play_count: i64,
    last_played: Option<String>,
}

/// Apple Podcasts listening history source.
///
/// Reads from the Core Data SQLite database that Apple Podcasts uses for local
/// storage. The database lives in a group container and requires Full Disk
/// Access (TCC permission) for external processes to read it.
pub struct ApplePodcastsSource {
    db_path: PathBuf,
}

impl ApplePodcastsSource {
    pub fn new() -> Result<Self, SourceError> {
        let home = std::env::var("HOME")
            .map_err(|_| SourceError::ParseError("HOME not set".to_string()))?;

        let db_path = PathBuf::from(home).join(
            "Library/Group Containers/243LU875E5.groups.com.apple.podcasts/Documents/MTLibrary.sqlite",
        );

        Ok(Self { db_path })
    }

    /// Constructor with an explicit path (useful for testing).
    pub fn new_with_path(path: impl Into<PathBuf>) -> Self {
        Self {
            db_path: path.into(),
        }
    }

    /// Convert a Core Data timestamp (seconds since 2001-01-01) to an ISO 8601
    /// string. Returns an empty string if the timestamp cannot be converted.
    fn core_data_to_iso(timestamp: f64) -> String {
        let unix_ts = timestamp + CORE_DATA_EPOCH_OFFSET;
        DateTime::from_timestamp(unix_ts as i64, 0)
            .map(|dt| dt.to_rfc3339())
            .unwrap_or_default()
    }

    /// Open the SQLite database in read-only mode.
    fn open_db(&self) -> Result<Connection, SourceError> {
        if !self.db_path.exists() {
            warn!("Podcasts database not found at: {}", self.db_path.display());
            return Err(SourceError::FileNotFound(self.db_path.clone()));
        }

        debug!("Opening Apple Podcasts DB: {}", self.db_path.display());

        Connection::open_with_flags(
            &self.db_path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(|e| SourceError::ParseError(format!("SQLite open: {}", e)))
    }

    /// Query episodes played within the last 7 days, ordered most-recent first.
    fn query_recent_episodes(&self) -> Result<Vec<EpisodeInfo>, SourceError> {
        let conn = self.open_db()?;

        let cutoff = (Utc::now().timestamp() as f64) - CORE_DATA_EPOCH_OFFSET - SEVEN_DAYS_SECS;

        let mut stmt = conn
            .prepare(
                "SELECT e.ZTITLE, p.ZTITLE, e.ZDURATION, e.ZPLAYCOUNT, e.ZLASTDATEPLAYED
                 FROM ZMTEPISODE e
                 LEFT JOIN ZMTPODCAST p ON e.ZPODCAST = p.Z_PK
                 WHERE e.ZLASTDATEPLAYED > ?1
                 ORDER BY e.ZLASTDATEPLAYED DESC
                 LIMIT ?2",
            )
            .map_err(|e| SourceError::ParseError(format!("SQL prepare: {}", e)))?;

        let rows = stmt
            .query_map(rusqlite::params![cutoff, RECENT_EPISODE_LIMIT], |row| {
                Ok(EpisodeInfo {
                    episode_title: row.get::<_, String>(0).unwrap_or_default(),
                    podcast_name: row.get::<_, String>(1).unwrap_or_default(),
                    duration_seconds: row.get::<_, Option<f64>>(2).ok().flatten(),
                    play_count: row.get::<_, i64>(3).unwrap_or(0),
                    last_played: row
                        .get::<_, Option<f64>>(4)
                        .ok()
                        .flatten()
                        .map(Self::core_data_to_iso),
                })
            })
            .map_err(|e| SourceError::ParseError(format!("SQL query: {}", e)))?;

        let episodes: Vec<EpisodeInfo> = rows.filter_map(|r| r.ok()).collect();

        info!("Loaded {} recent Apple Podcasts episodes", episodes.len());

        Ok(episodes)
    }

    /// Return aggregate counts: (total_episodes, total_podcasts).
    fn query_stats(&self) -> Result<(u64, u64), SourceError> {
        let conn = self.open_db()?;

        let total_episodes: u64 = conn
            .query_row("SELECT COUNT(*) FROM ZMTEPISODE", [], |row| row.get(0))
            .unwrap_or(0);

        let total_podcasts: u64 = conn
            .query_row("SELECT COUNT(*) FROM ZMTPODCAST", [], |row| row.get(0))
            .unwrap_or(0);

        Ok((total_episodes, total_podcasts))
    }

    /// Format a number with comma-separated thousands (e.g. 1234 -> "1,234").
    fn format_number(n: u64) -> String {
        n.to_string()
            .as_bytes()
            .rchunks(3)
            .rev()
            .map(std::str::from_utf8)
            .collect::<Result<Vec<&str>, _>>()
            .unwrap()
            .join(",")
    }
}

impl Source for ApplePodcastsSource {
    fn id(&self) -> &str {
        "apple-podcasts"
    }

    fn name(&self) -> &str {
        "Apple Podcasts"
    }

    fn watch_path(&self) -> Option<PathBuf> {
        Some(self.db_path.clone())
    }

    fn parse(&self) -> Result<serde_json::Value, SourceError> {
        let episodes = self.query_recent_episodes()?;
        let (total_episodes, total_podcasts) = self.query_stats()?;
        let recent_count = episodes.len();

        Ok(serde_json::json!({
            "source": "apple_podcasts",
            "timestamp": Utc::now().to_rfc3339(),
            "recent_episodes": episodes,
            "stats": {
                "total_episodes": total_episodes,
                "total_podcasts": total_podcasts,
                "recent_count": recent_count,
            }
        }))
    }

    fn preview(&self) -> Result<SourcePreview, SourceError> {
        let episodes = self.query_recent_episodes()?;
        let (total_episodes, total_podcasts) = self.query_stats()?;

        let summary = format!(
            "{} episodes from {} podcasts",
            Self::format_number(total_episodes),
            total_podcasts
        );

        let mut fields = vec![
            PreviewField {
                label: "Total Episodes".to_string(),
                value: Self::format_number(total_episodes),
                sensitive: false,
            },
            PreviewField {
                label: "Subscribed Podcasts".to_string(),
                value: total_podcasts.to_string(),
                sensitive: false,
            },
            PreviewField {
                label: "Recent (7d)".to_string(),
                value: episodes.len().to_string(),
                sensitive: false,
            },
        ];

        if let Some(ep) = episodes.first() {
            fields.push(PreviewField {
                label: "Latest".to_string(),
                value: format!("{} — {}", ep.podcast_name, ep.episode_title),
                sensitive: true,
            });
        }

        let last_updated = std::fs::metadata(&self.db_path)
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| DateTime::<Utc>::from(t).into());

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
    fn test_core_data_timestamp_conversion() {
        // 2026-01-15 00:00:00 UTC = Unix 1768435200
        // Core Data = 1768435200 - 978307200 = 790128000
        let iso = ApplePodcastsSource::core_data_to_iso(790_128_000.0);
        assert!(iso.starts_with("2026-01-15"), "got: {}", iso);
    }

    #[test]
    fn test_core_data_epoch_start() {
        // Core Data timestamp 0 = 2001-01-01T00:00:00+00:00
        let iso = ApplePodcastsSource::core_data_to_iso(0.0);
        assert!(iso.starts_with("2001-01-01"), "got: {}", iso);
    }

    #[test]
    fn test_source_trait_metadata() {
        let source = ApplePodcastsSource::new_with_path("/tmp/fake.sqlite");
        assert_eq!(source.id(), "apple-podcasts");
        assert_eq!(source.name(), "Apple Podcasts");
        assert!(source.watch_path().is_some());
    }

    #[test]
    fn test_watch_path_matches_db_path() {
        let path = PathBuf::from("/tmp/test.sqlite");
        let source = ApplePodcastsSource::new_with_path(path.clone());
        assert_eq!(source.watch_path().unwrap(), path);
    }

    #[test]
    fn test_missing_db_returns_file_not_found() {
        let source = ApplePodcastsSource::new_with_path("/tmp/nonexistent-db.sqlite");
        let err = source.parse().unwrap_err();
        assert!(
            matches!(err, SourceError::FileNotFound(_)),
            "expected FileNotFound, got: {:?}",
            err
        );
    }

    #[test]
    fn test_format_number() {
        assert_eq!(ApplePodcastsSource::format_number(0), "0");
        assert_eq!(ApplePodcastsSource::format_number(42), "42");
        assert_eq!(ApplePodcastsSource::format_number(1_234), "1,234");
        assert_eq!(ApplePodcastsSource::format_number(1_234_567), "1,234,567");
    }
}
