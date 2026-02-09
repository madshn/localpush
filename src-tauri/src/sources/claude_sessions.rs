use super::{PreviewField, Source, SourceError, SourcePreview};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;
use tracing::{debug, info, warn};

/// Wrapper for the sessions-index.json file format
#[derive(Debug, Deserialize)]
struct SessionIndexFile {
    entries: Vec<SessionIndexEntry>,
}

/// Entry in a project's sessions-index.json
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SessionIndexEntry {
    session_id: String,
    full_path: Option<String>,
    first_prompt: Option<String>,
    summary: Option<String>,
    message_count: Option<u32>,
    created: Option<String>,
    modified: Option<String>,
    git_branch: Option<String>,
    project_path: Option<String>,
}

/// Aggregated token counts from a session's JSONL file
#[derive(Debug, Default, serde::Serialize)]
struct TokenSummary {
    input: u64,
    output: u64,
    cache_read: u64,
    cache_creation: u64,
    model: Option<String>,
}

/// Claude Code session activity source.
///
/// Watches `~/.claude/projects/` and aggregates session metadata + token usage
/// from sessions modified within the last 24 hours.
pub struct ClaudeSessionsSource {
    claude_projects_dir: PathBuf,
}

impl ClaudeSessionsSource {
    pub fn new() -> Result<Self, SourceError> {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .map_err(|_| {
                SourceError::ParseError("Could not determine home directory".to_string())
            })?;

        let claude_projects_dir = PathBuf::from(home).join(".claude").join("projects");

        Ok(Self { claude_projects_dir })
    }

    /// Constructor with custom path (for testing)
    pub fn new_with_path(path: impl Into<PathBuf>) -> Self {
        Self {
            claude_projects_dir: path.into(),
        }
    }

    /// Scan all project directories for sessions-index.json files
    fn scan_session_indices(&self) -> Vec<(String, Vec<SessionIndexEntry>)> {
        let read_dir = match fs::read_dir(&self.claude_projects_dir) {
            Ok(rd) => rd,
            Err(e) => {
                debug!(
                    "Cannot read projects dir {}: {}",
                    self.claude_projects_dir.display(),
                    e
                );
                return Vec::new();
            }
        };

        let mut results = Vec::new();

        for entry in read_dir.flatten() {
            let index_path = entry.path().join("sessions-index.json");
            if !index_path.exists() {
                continue;
            }

            let content = match fs::read_to_string(&index_path) {
                Ok(c) => c,
                Err(e) => {
                    warn!("Failed to read {}: {}", index_path.display(), e);
                    continue;
                }
            };

            // Try wrapped format first: { "version": N, "entries": [...] }
            // Fall back to bare array: [...]
            let entries = serde_json::from_str::<SessionIndexFile>(&content)
                .map(|f| f.entries)
                .or_else(|_| serde_json::from_str::<Vec<SessionIndexEntry>>(&content));

            match entries {
                Ok(entries) => {
                    let dir_name = entry.file_name().to_string_lossy().to_string();
                    debug!("Found {} sessions in {}", entries.len(), dir_name);
                    results.push((dir_name, entries));
                }
                Err(e) => {
                    warn!("Failed to parse {}: {}", index_path.display(), e);
                }
            }
        }

        results
    }

    /// Extract token usage from a session's JSONL file
    fn extract_tokens(jsonl_path: &str) -> TokenSummary {
        let mut summary = TokenSummary::default();

        let content = match fs::read_to_string(jsonl_path) {
            Ok(c) => c,
            Err(_) => return summary,
        };

        for line in content.lines() {
            let obj = match serde_json::from_str::<serde_json::Value>(line) {
                Ok(v) => v,
                Err(_) => continue,
            };

            if obj.get("type").and_then(|t| t.as_str()) != Some("assistant") {
                continue;
            }

            if let Some(usage) = obj.pointer("/message/usage") {
                summary.input += usage
                    .get("input_tokens")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                summary.output += usage
                    .get("output_tokens")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                summary.cache_read += usage
                    .get("cache_read_input_tokens")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                summary.cache_creation += usage
                    .get("cache_creation_input_tokens")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
            }

            if summary.model.is_none() {
                summary.model = obj
                    .pointer("/message/model")
                    .and_then(|m| m.as_str())
                    .map(|s| s.to_string());
            }
        }

        summary
    }

    /// Collect sessions modified within the last 7 days, sorted newest first.
    /// sessions-index.json is not updated in real-time by Claude Code,
    /// so a 24h window often misses active sessions.
    fn recent_sessions(&self) -> Vec<(SessionIndexEntry, TokenSummary)> {
        let cutoff = Utc::now() - chrono::Duration::days(7);
        let mut results = Vec::new();

        for (_dir, entries) in self.scan_session_indices() {
            for entry in entries {
                let modified_dt = entry
                    .modified
                    .as_ref()
                    .and_then(|m| DateTime::parse_from_rfc3339(m).ok())
                    .map(|dt| dt.with_timezone(&Utc));

                let is_recent = modified_dt.is_some_and(|dt| dt >= cutoff);
                if !is_recent {
                    continue;
                }

                let tokens = entry
                    .full_path
                    .as_deref()
                    .map(Self::extract_tokens)
                    .unwrap_or_default();

                results.push((entry, tokens));
            }
        }

        // Most recently modified first
        results.sort_by(|a, b| b.0.modified.cmp(&a.0.modified));

        info!("Found {} recent sessions (last 7d)", results.len());
        results
    }

    /// Calculate duration in seconds between created and modified timestamps
    fn session_duration(entry: &SessionIndexEntry) -> Option<i64> {
        let start = entry
            .created
            .as_ref()
            .and_then(|c| DateTime::parse_from_rfc3339(c).ok());
        let end = entry
            .modified
            .as_ref()
            .and_then(|m| DateTime::parse_from_rfc3339(m).ok());

        match (start, end) {
            (Some(s), Some(e)) => Some((e - s).num_seconds()),
            _ => None,
        }
    }

    /// Format a number with comma separators (e.g. 1234567 -> "1,234,567")
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

    /// Get the display title for a session, falling back through summary -> first_prompt -> "Untitled"
    fn session_title(entry: &SessionIndexEntry) -> &str {
        entry
            .summary
            .as_deref()
            .or(entry.first_prompt.as_deref())
            .unwrap_or("Untitled session")
    }
}

impl Source for ClaudeSessionsSource {
    fn id(&self) -> &str {
        "claude-sessions"
    }

    fn name(&self) -> &str {
        "Claude Code Sessions"
    }

    fn watch_path(&self) -> Option<PathBuf> {
        Some(self.claude_projects_dir.clone())
    }

    fn parse(&self) -> Result<serde_json::Value, SourceError> {
        let recent = self.recent_sessions();

        let sessions: Vec<serde_json::Value> = recent
            .iter()
            .map(|(entry, tokens)| {
                serde_json::json!({
                    "id": entry.session_id,
                    "project_path": entry.project_path,
                    "git_branch": entry.git_branch,
                    "title": Self::session_title(entry),
                    "start_time": entry.created,
                    "end_time": entry.modified,
                    "duration_seconds": Self::session_duration(entry),
                    "message_count": entry.message_count,
                    "tokens": {
                        "input": tokens.input,
                        "output": tokens.output,
                        "cache_read": tokens.cache_read,
                        "cache_creation": tokens.cache_creation,
                    },
                    "model": tokens.model,
                })
            })
            .collect();

        let total_tokens: u64 = recent.iter().map(|(_, t)| t.input + t.output).sum();
        let total_duration: i64 = recent
            .iter()
            .filter_map(|(e, _)| Self::session_duration(e))
            .sum();

        Ok(serde_json::json!({
            "source": "claude_code_sessions",
            "timestamp": Utc::now().to_rfc3339(),
            "sessions": sessions,
            "summary": {
                "sessions_7d": recent.len(),
                "total_tokens_7d": total_tokens,
                "total_duration_7d_seconds": total_duration,
            }
        }))
    }

    fn preview(&self) -> Result<SourcePreview, SourceError> {
        let recent = self.recent_sessions();
        let total_tokens: u64 = recent.iter().map(|(_, t)| t.input + t.output).sum();

        let summary = if recent.is_empty() {
            "No sessions in last 7 days".to_string()
        } else {
            format!(
                "{} sessions, {} tokens",
                recent.len(),
                Self::format_number(total_tokens)
            )
        };

        let mut fields = vec![
            PreviewField {
                label: "Sessions (7d)".to_string(),
                value: recent.len().to_string(),
                sensitive: false,
            },
            PreviewField {
                label: "Total Tokens".to_string(),
                value: Self::format_number(total_tokens),
                sensitive: false,
            },
        ];

        // Show most recent session details
        if let Some((entry, _)) = recent.first() {
            fields.push(PreviewField {
                label: "Latest Session".to_string(),
                value: Self::session_title(entry).to_string(),
                sensitive: true,
            });

            if let Some(ref project) = entry.project_path {
                fields.push(PreviewField {
                    label: "Project".to_string(),
                    value: project.clone(),
                    sensitive: true,
                });
            }
        }

        Ok(SourcePreview {
            title: self.name().to_string(),
            summary,
            fields,
            last_updated: Some(Utc::now()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_test_dir() -> TempDir {
        let dir = TempDir::new().unwrap();
        let project_dir = dir.path().join("-Users-test-project");
        fs::create_dir_all(&project_dir).unwrap();

        let now = Utc::now();
        // Use the real wrapped format: { "version": 1, "entries": [...] }
        let index = serde_json::json!({
            "version": 1,
            "entries": [{
                "sessionId": "test-session-1",
                "fullPath": project_dir.join("test-session-1.jsonl").to_str().unwrap(),
                "firstPrompt": "test prompt",
                "summary": "Test session",
                "messageCount": 10,
                "created": (now - chrono::Duration::hours(2)).to_rfc3339(),
                "modified": now.to_rfc3339(),
                "gitBranch": "main",
                "projectPath": "/Users/test/project"
            }]
        });
        fs::write(
            project_dir.join("sessions-index.json"),
            serde_json::to_string(&index).unwrap(),
        )
        .unwrap();

        let jsonl = concat!(
            r#"{"type":"user","message":{"role":"user","content":"hello"}}"#,
            "\n",
            r#"{"type":"assistant","message":{"model":"claude-opus-4-6","usage":{"input_tokens":100,"output_tokens":50,"cache_read_input_tokens":500,"cache_creation_input_tokens":200}}}"#,
        );
        fs::write(project_dir.join("test-session-1.jsonl"), jsonl).unwrap();

        dir
    }

    #[test]
    fn test_parse_sessions() {
        let dir = setup_test_dir();
        let source = ClaudeSessionsSource::new_with_path(dir.path());

        let result = source.parse().unwrap();
        let sessions = result["sessions"].as_array().unwrap();

        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0]["tokens"]["input"], 100);
        assert_eq!(sessions[0]["tokens"]["output"], 50);
        assert_eq!(sessions[0]["tokens"]["cache_read"], 500);
        assert_eq!(sessions[0]["tokens"]["cache_creation"], 200);
        assert_eq!(sessions[0]["model"], "claude-opus-4-6");
    }

    #[test]
    fn test_preview() {
        let dir = setup_test_dir();
        let source = ClaudeSessionsSource::new_with_path(dir.path());

        let preview = source.preview().unwrap();

        assert_eq!(preview.title, "Claude Code Sessions");
        assert!(!preview.fields.is_empty());
        assert!(preview.summary.contains("1 sessions"));
    }

    #[test]
    fn test_empty_dir() {
        let dir = TempDir::new().unwrap();
        let source = ClaudeSessionsSource::new_with_path(dir.path());

        let result = source.parse().unwrap();
        let sessions = result["sessions"].as_array().unwrap();
        assert!(sessions.is_empty());

        let preview = source.preview().unwrap();
        assert_eq!(preview.summary, "No sessions in last 7 days");
    }

    #[test]
    fn test_extract_tokens_multiple_messages() {
        let dir = TempDir::new().unwrap();
        let jsonl = concat!(
            r#"{"type":"assistant","message":{"model":"claude-opus-4-6","usage":{"input_tokens":100,"output_tokens":50,"cache_read_input_tokens":500,"cache_creation_input_tokens":200}}}"#,
            "\n",
            r#"{"type":"assistant","message":{"model":"claude-opus-4-6","usage":{"input_tokens":200,"output_tokens":100,"cache_read_input_tokens":300,"cache_creation_input_tokens":100}}}"#,
        );
        let path = dir.path().join("test.jsonl");
        fs::write(&path, jsonl).unwrap();

        let tokens = ClaudeSessionsSource::extract_tokens(path.to_str().unwrap());

        assert_eq!(tokens.input, 300);
        assert_eq!(tokens.output, 150);
        assert_eq!(tokens.cache_read, 800);
        assert_eq!(tokens.cache_creation, 300);
        assert_eq!(tokens.model.as_deref(), Some("claude-opus-4-6"));
    }

    #[test]
    fn test_source_trait_impl() {
        let dir = TempDir::new().unwrap();
        let source = ClaudeSessionsSource::new_with_path(dir.path());

        assert_eq!(source.id(), "claude-sessions");
        assert_eq!(source.name(), "Claude Code Sessions");
        assert!(source.watch_path().is_some());
    }

    #[test]
    fn test_format_number() {
        assert_eq!(ClaudeSessionsSource::format_number(0), "0");
        assert_eq!(ClaudeSessionsSource::format_number(123), "123");
        assert_eq!(ClaudeSessionsSource::format_number(1234), "1,234");
        assert_eq!(ClaudeSessionsSource::format_number(1234567), "1,234,567");
    }

    #[test]
    fn test_old_session_excluded() {
        let dir = TempDir::new().unwrap();
        let project_dir = dir.path().join("-Users-test-old");
        fs::create_dir_all(&project_dir).unwrap();

        let old_time = Utc::now() - chrono::Duration::days(14);
        let index = serde_json::json!({
            "version": 1,
            "entries": [{
                "sessionId": "old-session",
                "firstPrompt": "old prompt",
                "messageCount": 5,
                "created": (old_time - chrono::Duration::hours(1)).to_rfc3339(),
                "modified": old_time.to_rfc3339(),
            }]
        });
        fs::write(
            project_dir.join("sessions-index.json"),
            serde_json::to_string(&index).unwrap(),
        )
        .unwrap();

        let source = ClaudeSessionsSource::new_with_path(dir.path());
        let result = source.parse().unwrap();
        let sessions = result["sessions"].as_array().unwrap();

        assert!(sessions.is_empty());
    }
}
