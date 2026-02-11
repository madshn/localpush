use super::{PreviewField, Source, SourceError, SourcePreview};
use crate::source_config::PropertyDef;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;
use tracing::{debug, info, warn};

/// Wrapper for the sessions-index.json file format (legacy)
#[derive(Debug, Deserialize)]
struct SessionIndexFile {
    entries: Vec<SessionIndexEntry>,
}

/// Entry in a project's sessions-index.json (legacy)
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

/// Unified session metadata extracted from either JSONL files or sessions-index.json
#[derive(Debug)]
struct SessionInfo {
    session_id: String,
    first_prompt: Option<String>,
    summary: Option<String>,
    message_count: u32,
    created: Option<String>,
    modified: Option<String>,
    git_branch: Option<String>,
    project_path: Option<String>,
    #[allow(dead_code)]
    jsonl_path: Option<String>,
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
/// from sessions modified within the last 7 days.
///
/// Primary discovery: scan JSONL files directly in project directories.
/// Fallback: parse sessions-index.json (older Claude Code versions).
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

    /// Scan project directories for JSONL session files.
    ///
    /// Each project directory (e.g. `-Users-name-dev-project/`) contains
    /// `{session-uuid}.jsonl` files. We use file system mtime as the
    /// "modified" timestamp and parse the JSONL content for metadata.
    fn scan_jsonl_sessions(&self, cutoff: DateTime<Utc>) -> Vec<(SessionInfo, TokenSummary)> {
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

        for project_entry in read_dir.flatten() {
            let project_path = project_entry.path();
            if !project_path.is_dir() {
                continue;
            }

            let project_name = project_entry.file_name().to_string_lossy().to_string();

            let project_dir = match fs::read_dir(&project_path) {
                Ok(rd) => rd,
                Err(_) => continue,
            };

            for file_entry in project_dir.flatten() {
                let path = file_entry.path();
                let name = file_entry.file_name().to_string_lossy().to_string();

                // Only process UUID.jsonl files (skip subagent dirs, index files)
                if !name.ends_with(".jsonl") {
                    continue;
                }

                // Check file mtime against cutoff (fast — no file content reading)
                let metadata = match fs::metadata(&path) {
                    Ok(m) => m,
                    Err(_) => continue,
                };
                let modified_time = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);
                let modified_dt: DateTime<Utc> = modified_time.into();

                if modified_dt < cutoff {
                    continue;
                }

                // Extract session ID from filename
                let session_id = name.trim_end_matches(".jsonl").to_string();

                // Parse JSONL content for metadata and tokens
                let jsonl_path_str = path.to_string_lossy().to_string();
                let (info, tokens) =
                    Self::parse_jsonl_session(&session_id, &jsonl_path_str, &project_name, modified_dt);

                results.push((info, tokens));
            }
        }

        debug!("JSONL scan found {} recent sessions", results.len());
        results
    }

    /// Parse a JSONL session file to extract metadata and token usage.
    fn parse_jsonl_session(
        session_id: &str,
        jsonl_path: &str,
        project_dir_name: &str,
        file_modified: DateTime<Utc>,
    ) -> (SessionInfo, TokenSummary) {
        let mut tokens = TokenSummary::default();
        let mut first_prompt: Option<String> = None;
        let mut first_timestamp: Option<String> = None;
        let mut last_timestamp: Option<String> = None;
        let mut git_branch: Option<String> = None;
        let mut cwd: Option<String> = None;
        let mut message_count: u32 = 0;
        let mut summary: Option<String> = None;

        let content = match fs::read_to_string(jsonl_path) {
            Ok(c) => c,
            Err(_) => {
                return (
                    SessionInfo {
                        session_id: session_id.to_string(),
                        first_prompt: None,
                        summary: None,
                        message_count: 0,
                        created: None,
                        modified: Some(file_modified.to_rfc3339()),
                        git_branch: None,
                        project_path: None,
                        jsonl_path: Some(jsonl_path.to_string()),
                    },
                    tokens,
                );
            }
        };

        for line in content.lines() {
            let obj = match serde_json::from_str::<serde_json::Value>(line) {
                Ok(v) => v,
                Err(_) => continue,
            };

            let msg_type = obj.get("type").and_then(|t| t.as_str());
            let ts = obj.get("timestamp").and_then(|t| t.as_str()).map(|s| s.to_string());

            match msg_type {
                Some("user") => {
                    message_count += 1;
                    if first_timestamp.is_none() {
                        first_timestamp.clone_from(&ts);
                        git_branch = obj
                            .get("gitBranch")
                            .and_then(|b| b.as_str())
                            .map(|s| s.to_string());
                        cwd = obj
                            .get("cwd")
                            .and_then(|c| c.as_str())
                            .map(|s| s.to_string());

                        // Extract first prompt text
                        let msg_content = obj.pointer("/message/content");
                        first_prompt = match msg_content {
                            Some(serde_json::Value::String(s)) => {
                                Some(s.chars().take(120).collect())
                            }
                            Some(serde_json::Value::Array(arr)) => arr.iter().find_map(|item| {
                                if item.get("type").and_then(|t| t.as_str()) == Some("text") {
                                    item.get("text")
                                        .and_then(|t| t.as_str())
                                        .map(|s| s.chars().take(120).collect())
                                } else {
                                    None
                                }
                            }),
                            _ => None,
                        };
                    }
                    if ts.is_some() {
                        last_timestamp.clone_from(&ts);
                    }
                }
                Some("assistant") => {
                    if let Some(usage) = obj.pointer("/message/usage") {
                        tokens.input += usage
                            .get("input_tokens")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0);
                        tokens.output += usage
                            .get("output_tokens")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0);
                        tokens.cache_read += usage
                            .get("cache_read_input_tokens")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0);
                        tokens.cache_creation += usage
                            .get("cache_creation_input_tokens")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0);
                    }

                    if tokens.model.is_none() {
                        tokens.model = obj
                            .pointer("/message/model")
                            .and_then(|m| m.as_str())
                            .map(|s| s.to_string());
                    }

                    if ts.is_some() {
                        last_timestamp.clone_from(&ts);
                    }
                }
                Some("summary") => {
                    // Some sessions have a summary message type
                    summary = obj
                        .get("summary")
                        .and_then(|s| s.as_str())
                        .map(|s| s.to_string());
                }
                _ => {}
            }
        }

        let info = SessionInfo {
            session_id: session_id.to_string(),
            first_prompt,
            summary,
            message_count,
            created: first_timestamp,
            modified: last_timestamp.or_else(|| Some(file_modified.to_rfc3339())),
            git_branch,
            project_path: cwd.or_else(|| {
                // Derive project path from directory name convention:
                // "-Users-name-dev-project" → "/Users/name/dev/project"
                if project_dir_name.starts_with('-') {
                    Some(project_dir_name.replace('-', "/"))
                } else {
                    None
                }
            }),
            jsonl_path: Some(jsonl_path.to_string()),
        };

        (info, tokens)
    }

    /// Scan sessions-index.json files (legacy fallback for older Claude Code versions)
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

            let entries = serde_json::from_str::<SessionIndexFile>(&content)
                .map(|f| f.entries)
                .or_else(|_| serde_json::from_str::<Vec<SessionIndexEntry>>(&content));

            match entries {
                Ok(entries) => {
                    let dir_name = entry.file_name().to_string_lossy().to_string();
                    debug!("Found {} sessions in index for {}", entries.len(), dir_name);
                    results.push((dir_name, entries));
                }
                Err(e) => {
                    warn!("Failed to parse {}: {}", index_path.display(), e);
                }
            }
        }

        results
    }

    /// Extract token usage from a JSONL file path (legacy helper for index-based sessions)
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
    ///
    /// Uses two discovery strategies:
    /// 1. Primary: scan JSONL files directly (works with current Claude Code)
    /// 2. Fallback: parse sessions-index.json (older Claude Code versions)
    ///
    /// Results are deduplicated by session ID, preferring JSONL-discovered sessions.
    fn recent_sessions(&self) -> Vec<(SessionInfo, TokenSummary)> {
        let cutoff = Utc::now() - chrono::Duration::days(7);

        // Primary: scan JSONL files directly
        let mut results = self.scan_jsonl_sessions(cutoff);
        let mut seen_ids: std::collections::HashSet<String> =
            results.iter().map(|(info, _)| info.session_id.clone()).collect();

        // Fallback: sessions-index.json (may find sessions with JSONL in different locations)
        for (_dir, entries) in self.scan_session_indices() {
            for entry in entries {
                if seen_ids.contains(&entry.session_id) {
                    continue;
                }

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

                let info = SessionInfo {
                    session_id: entry.session_id.clone(),
                    first_prompt: entry.first_prompt,
                    summary: entry.summary,
                    message_count: entry.message_count.unwrap_or(0),
                    created: entry.created,
                    modified: entry.modified,
                    git_branch: entry.git_branch,
                    project_path: entry.project_path,
                    jsonl_path: entry.full_path,
                };

                seen_ids.insert(entry.session_id);
                results.push((info, tokens));
            }
        }

        // Most recently modified first
        results.sort_by(|a, b| b.0.modified.cmp(&a.0.modified));

        info!("Found {} recent sessions (last 7d)", results.len());
        results
    }

    /// Calculate duration in seconds between created and modified timestamps
    fn session_duration(info: &SessionInfo) -> Option<i64> {
        let start = info
            .created
            .as_ref()
            .and_then(|c| DateTime::parse_from_rfc3339(c).ok());
        let end = info
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

    /// Get the display title for a session
    fn session_title(info: &SessionInfo) -> &str {
        info.summary
            .as_deref()
            .or(info.first_prompt.as_deref())
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
            .map(|(info, tokens)| {
                serde_json::json!({
                    "id": info.session_id,
                    "project_path": info.project_path,
                    "git_branch": info.git_branch,
                    "title": Self::session_title(info),
                    "start_time": info.created,
                    "end_time": info.modified,
                    "duration_seconds": Self::session_duration(info),
                    "message_count": info.message_count,
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
            .filter_map(|(info, _)| Self::session_duration(info))
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
        if let Some((info, _)) = recent.first() {
            fields.push(PreviewField {
                label: "Latest Session".to_string(),
                value: Self::session_title(info).to_string(),
                sensitive: true,
            });

            if let Some(ref project) = info.project_path {
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

    fn available_properties(&self) -> Vec<PropertyDef> {
        vec![
            PropertyDef {
                key: "sessions".to_string(),
                label: "Sessions".to_string(),
                description: "Session list with metadata from the last 7 days".to_string(),
                default_enabled: true,
                privacy_sensitive: false,
            },
            PropertyDef {
                key: "cache_efficiency".to_string(),
                label: "Cache Efficiency".to_string(),
                description: "Cache hit rate and prompt caching metrics".to_string(),
                default_enabled: true,
                privacy_sensitive: false,
            },
            PropertyDef {
                key: "model_distribution".to_string(),
                label: "Model Distribution".to_string(),
                description: "Which Claude models were used across sessions".to_string(),
                default_enabled: true,
                privacy_sensitive: false,
            },
            PropertyDef {
                key: "git_branches".to_string(),
                label: "Git Branches".to_string(),
                description: "Active git branches from session contexts".to_string(),
                default_enabled: true,
                privacy_sensitive: false,
            },
            PropertyDef {
                key: "first_prompt_preview".to_string(),
                label: "First Prompt Preview".to_string(),
                description: "Opening text from each session (may contain project details)"
                    .to_string(),
                default_enabled: false,
                privacy_sensitive: true,
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// Create a test directory with a JSONL session file (new format)
    fn setup_jsonl_test_dir() -> TempDir {
        let dir = TempDir::new().unwrap();
        let project_dir = dir.path().join("-Users-test-project");
        fs::create_dir_all(&project_dir).unwrap();

        let now = Utc::now();
        let created = (now - chrono::Duration::hours(2)).to_rfc3339();
        let modified = now.to_rfc3339();

        let jsonl = format!(
            concat!(
                r#"{{"type":"user","sessionId":"test-session-1","timestamp":"{created}","cwd":"/Users/test/project","gitBranch":"main","message":{{"role":"user","content":"test prompt"}}}}"#,
                "\n",
                r#"{{"type":"assistant","timestamp":"{modified}","message":{{"model":"claude-opus-4-6","usage":{{"input_tokens":100,"output_tokens":50,"cache_read_input_tokens":500,"cache_creation_input_tokens":200}}}}}}"#,
            ),
            created = created,
            modified = modified,
        );
        fs::write(project_dir.join("test-session-1.jsonl"), jsonl).unwrap();

        dir
    }

    /// Create a test directory with sessions-index.json (legacy format)
    fn setup_legacy_test_dir() -> TempDir {
        let dir = TempDir::new().unwrap();
        let project_dir = dir.path().join("-Users-test-project");
        fs::create_dir_all(&project_dir).unwrap();

        let now = Utc::now();
        let index = serde_json::json!({
            "version": 1,
            "entries": [{
                "sessionId": "legacy-session-1",
                "fullPath": project_dir.join("legacy-session-1.jsonl").to_str().unwrap(),
                "firstPrompt": "legacy prompt",
                "summary": "Legacy session",
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
        fs::write(project_dir.join("legacy-session-1.jsonl"), jsonl).unwrap();

        dir
    }

    #[test]
    fn test_parse_jsonl_sessions() {
        let dir = setup_jsonl_test_dir();
        let source = ClaudeSessionsSource::new_with_path(dir.path());

        let result = source.parse().unwrap();
        let sessions = result["sessions"].as_array().unwrap();

        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0]["tokens"]["input"], 100);
        assert_eq!(sessions[0]["tokens"]["output"], 50);
        assert_eq!(sessions[0]["tokens"]["cache_read"], 500);
        assert_eq!(sessions[0]["tokens"]["cache_creation"], 200);
        assert_eq!(sessions[0]["model"], "claude-opus-4-6");
        assert_eq!(sessions[0]["title"], "test prompt");
        assert_eq!(sessions[0]["git_branch"], "main");
    }

    #[test]
    fn test_parse_legacy_sessions() {
        let dir = setup_legacy_test_dir();
        let source = ClaudeSessionsSource::new_with_path(dir.path());

        let result = source.parse().unwrap();
        let sessions = result["sessions"].as_array().unwrap();

        // Should find sessions from both JSONL scan and legacy index
        assert!(!sessions.is_empty());
    }

    #[test]
    fn test_preview_jsonl() {
        let dir = setup_jsonl_test_dir();
        let source = ClaudeSessionsSource::new_with_path(dir.path());

        let preview = source.preview().unwrap();

        assert_eq!(preview.title, "Claude Code Sessions");
        assert!(!preview.fields.is_empty());
        assert!(
            preview.summary.contains("1 sessions"),
            "Summary was: {}",
            preview.summary
        );
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
    fn test_old_session_excluded_via_index() {
        // Legacy index sessions with old timestamps should be excluded
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
