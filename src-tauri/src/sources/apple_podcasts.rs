use super::{PreviewField, Source, SourceError, SourcePreview};
use crate::source_config::PropertyDef;
use chrono::{DateTime, Utc};
use regex::Regex;
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

/// An extracted link from an episode description.
#[derive(Debug, Serialize, Clone)]
struct ExtractedLink {
    url: String,
    source: String, // "anchor" or "bare"
}

/// A transcript snippet entry.
#[derive(Debug, Serialize, Clone)]
struct TranscriptSnippet {
    speaker_id: Option<String>,
    content: String,
}

/// A single played episode with metadata from its parent podcast.
#[derive(Debug, Serialize)]
struct EpisodeInfo {
    episode_title: String,
    podcast_name: String,
    duration_seconds: Option<f64>,
    play_count: i64,
    last_played: Option<String>,
    episode_url: Option<String>,
    links: Vec<ExtractedLink>,
    has_transcript: bool,
    transcript_snippet: Option<Vec<TranscriptSnippet>>,
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

    /// Extract URLs from HTML description text.
    fn extract_urls(html_description: &str) -> Vec<ExtractedLink> {
        let mut links = Vec::new();

        // Tier 1: Extract href="..." from anchor tags
        if let Ok(href_re) = Regex::new(r#"href="([^"]+)""#) {
            for cap in href_re.captures_iter(html_description) {
                if let Some(url_match) = cap.get(1) {
                    links.push(ExtractedLink {
                        url: url_match.as_str().to_string(),
                        source: "anchor".to_string(),
                    });
                }
            }
        }

        // Tier 2: Extract bare URLs not already captured
        if let Ok(bare_re) = Regex::new(r#"https?://[^\s<>"']+"#) {
            for mat in bare_re.find_iter(html_description) {
                let url = mat.as_str().to_string();
                if !links.iter().any(|l| l.url == url) {
                    links.push(ExtractedLink {
                        url,
                        source: "bare".to_string(),
                    });
                }
            }
        }

        // Filter boilerplate
        links.retain(|l| !l.url.contains("acast.com/privacy"));

        links
    }

    /// Parse transcript snippet JSON if available.
    fn parse_transcript_snippet(json_str: &str) -> Option<Vec<TranscriptSnippet>> {
        if json_str.is_empty() {
            return None;
        }

        // Try to parse as JSON array
        if let Ok(parsed) = serde_json::from_str::<Vec<serde_json::Value>>(json_str) {
            let snippets: Vec<TranscriptSnippet> = parsed
                .iter()
                .filter_map(|item| {
                    Some(TranscriptSnippet {
                        speaker_id: item.get("speaker_id")?.as_str().map(String::from),
                        content: item.get("content")?.as_str()?.to_string(),
                    })
                })
                .collect();

            if snippets.is_empty() {
                None
            } else {
                Some(snippets)
            }
        } else {
            None
        }
    }

    /// Query episodes played within the last 7 days, ordered most-recent first.
    fn query_recent_episodes(&self) -> Result<Vec<EpisodeInfo>, SourceError> {
        let conn = self.open_db()?;

        let cutoff = (Utc::now().timestamp() as f64) - CORE_DATA_EPOCH_OFFSET - SEVEN_DAYS_SECS;

        let mut stmt = conn
            .prepare(
                "SELECT e.ZTITLE, p.ZTITLE, e.ZDURATION, e.ZPLAYCOUNT, e.ZLASTDATEPLAYED,
                        e.ZWEBPAGEURL, e.ZITEMDESCRIPTION, e.ZTRANSCRIPTIDENTIFIER,
                        e.ZENTITLEDTRANSCRIPTSNIPPET
                 FROM ZMTEPISODE e
                 LEFT JOIN ZMTPODCAST p ON e.ZPODCAST = p.Z_PK
                 WHERE e.ZLASTDATEPLAYED > ?1
                 ORDER BY e.ZLASTDATEPLAYED DESC
                 LIMIT ?2",
            )
            .map_err(|e| SourceError::ParseError(format!("SQL prepare: {}", e)))?;

        let rows = stmt
            .query_map(rusqlite::params![cutoff, RECENT_EPISODE_LIMIT], |row| {
                let episode_title: String = row.get::<_, String>(0).unwrap_or_default();
                let podcast_name: String = row.get::<_, String>(1).unwrap_or_default();
                let duration_seconds: Option<f64> = row.get::<_, Option<f64>>(2).ok().flatten();
                let play_count: i64 = row.get::<_, i64>(3).unwrap_or(0);
                let last_played: Option<String> = row
                    .get::<_, Option<f64>>(4)
                    .ok()
                    .flatten()
                    .map(Self::core_data_to_iso);

                let episode_url: Option<String> = row.get::<_, Option<String>>(5).ok().flatten();
                let description: String = row.get::<_, String>(6).unwrap_or_default();
                let transcript_id: Option<String> = row.get::<_, Option<String>>(7).ok().flatten();
                let transcript_json: String = row.get::<_, String>(8).unwrap_or_default();

                let links = Self::extract_urls(&description);
                let has_transcript = transcript_id.is_some();
                let transcript_snippet = Self::parse_transcript_snippet(&transcript_json);

                Ok(EpisodeInfo {
                    episode_title,
                    podcast_name,
                    duration_seconds,
                    play_count,
                    last_played,
                    episode_url,
                    links,
                    has_transcript,
                    transcript_snippet,
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

    fn available_properties(&self) -> Vec<PropertyDef> {
        vec![
            PropertyDef {
                key: "recent_episodes".to_string(),
                label: "Recent Episodes".to_string(),
                description: "Episode list with play data from the last 7 days".to_string(),
                default_enabled: true,
                privacy_sensitive: false,
            },
            PropertyDef {
                key: "episode_links".to_string(),
                label: "Episode Links".to_string(),
                description: "URLs extracted from episode descriptions".to_string(),
                default_enabled: true,
                privacy_sensitive: false,
            },
            PropertyDef {
                key: "transcript_snippets".to_string(),
                label: "Transcript Snippets".to_string(),
                description: "Preview text from episode transcripts".to_string(),
                default_enabled: false,
                privacy_sensitive: false,
            },
            PropertyDef {
                key: "podcast_metadata".to_string(),
                label: "Podcast Metadata".to_string(),
                description: "Show-level information and statistics".to_string(),
                default_enabled: true,
                privacy_sensitive: false,
            },
        ]
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

    #[test]
    fn test_extract_urls_from_anchor_tags() {
        let html = r#"<a href="https://example.com/show-notes">Show notes</a>"#;
        let links = ApplePodcastsSource::extract_urls(html);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].url, "https://example.com/show-notes");
        assert_eq!(links[0].source, "anchor");
    }

    #[test]
    fn test_extract_bare_urls() {
        let text = "Check out https://example.com/page for more info";
        let links = ApplePodcastsSource::extract_urls(text);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].url, "https://example.com/page");
        assert_eq!(links[0].source, "bare");
    }

    #[test]
    fn test_extract_urls_mixed() {
        let html = r#"Visit <a href="https://site1.com">here</a> or https://site2.com"#;
        let links = ApplePodcastsSource::extract_urls(html);
        assert_eq!(links.len(), 2);
        assert_eq!(links[0].source, "anchor");
        assert_eq!(links[1].source, "bare");
    }

    #[test]
    fn test_extract_urls_filters_boilerplate() {
        let html = r#"<a href="https://acast.com/privacy">Privacy</a>"#;
        let links = ApplePodcastsSource::extract_urls(html);
        assert_eq!(links.len(), 0);
    }

    #[test]
    fn test_extract_urls_no_duplicates() {
        let html = r#"<a href="https://example.com">Link</a> and https://example.com again"#;
        let links = ApplePodcastsSource::extract_urls(html);
        assert_eq!(links.len(), 1); // Only the anchor version
        assert_eq!(links[0].source, "anchor");
    }

    #[test]
    fn test_parse_transcript_snippet_valid() {
        let json = r#"[{"speaker_id":"1","content":"Hello"},{"speaker_id":"2","content":"World"}]"#;
        let snippet = ApplePodcastsSource::parse_transcript_snippet(json);
        assert!(snippet.is_some());
        let parsed = snippet.unwrap();
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].content, "Hello");
        assert_eq!(parsed[1].content, "World");
    }

    #[test]
    fn test_parse_transcript_snippet_empty() {
        let snippet = ApplePodcastsSource::parse_transcript_snippet("");
        assert!(snippet.is_none());
    }

    #[test]
    fn test_parse_transcript_snippet_invalid_json() {
        let json = "not valid json";
        let snippet = ApplePodcastsSource::parse_transcript_snippet(json);
        assert!(snippet.is_none());
    }

    #[test]
    fn test_extract_urls_empty_string() {
        let links = ApplePodcastsSource::extract_urls("");
        assert_eq!(links.len(), 0);
    }
}
