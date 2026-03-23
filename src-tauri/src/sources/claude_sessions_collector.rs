/// Shared JSONL scanning/parsing logic for Claude Code project sessions.
///
/// Both `claude_stats` and `claude_sessions` consume this collector to avoid
/// duplicating the JSONL traversal logic.
use chrono::{DateTime, Utc};
use std::fs;
use std::path::Path;
use std::time::SystemTime;
use tracing::debug;

/// Token usage from a single assistant message (or accumulated across a session).
#[derive(Debug, Default, Clone)]
pub struct TokenUsage {
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    pub cache_creation: u64,
}

/// A parsed assistant or user message within a session.
#[derive(Debug, Clone)]
pub struct ClaudeMessage {
    pub timestamp: DateTime<Utc>,
    pub msg_type: String, // "user" or "assistant"
    pub model: Option<String>,
    pub usage: TokenUsage,
    pub tool_calls: u64,
}

/// Aggregated session data parsed from a single `.jsonl` file.
#[derive(Debug, Clone)]
pub struct ClaudeSession {
    pub session_id: String,
    /// Ordered list of timestamps for all events (user + assistant).
    pub timestamps: Vec<DateTime<Utc>>,
    pub messages: Vec<ClaudeMessage>,
    pub project_path: Option<String>,
    pub git_branch: Option<String>,
    pub first_prompt: Option<String>,
    pub summary: Option<String>,
}

impl ClaudeSession {
    /// Returns the timestamp of the first event, if any.
    pub fn first_timestamp(&self) -> Option<DateTime<Utc>> {
        self.timestamps.first().copied()
    }

    /// Returns the timestamp of the last event, if any.
    pub fn last_timestamp(&self) -> Option<DateTime<Utc>> {
        self.timestamps.last().copied()
    }

    /// Total user + assistant message count.
    pub fn message_count(&self) -> u32 {
        self.messages.len() as u32
    }

    /// Best-effort primary model (first model seen in assistant messages).
    pub fn primary_model(&self) -> Option<&str> {
        self.messages.iter().find_map(|m| m.model.as_deref())
    }

    /// Summed token usage across all assistant messages.
    pub fn total_tokens(&self) -> TokenUsage {
        self.messages
            .iter()
            .fold(TokenUsage::default(), |mut acc, m| {
                acc.input += m.usage.input;
                acc.output += m.usage.output;
                acc.cache_read += m.usage.cache_read;
                acc.cache_creation += m.usage.cache_creation;
                acc
            })
    }
}

/// Scan `projects_dir` for Claude Code JSONL session files.
///
/// Each sub-directory of `projects_dir` (e.g. `-Users-name-dev-myproject/`)
/// may contain `{session-uuid}.jsonl` files.
///
/// `mtime_cutoff` — when provided, only files whose filesystem mtime is at or
/// after the cutoff are parsed (fast pre-filter, avoids reading old files).
/// Pass `None` to include all files regardless of age.
pub fn collect_claude_sessions(
    projects_dir: &Path,
    mtime_cutoff: Option<DateTime<Utc>>,
) -> Vec<ClaudeSession> {
    let read_dir = match fs::read_dir(projects_dir) {
        Ok(rd) => rd,
        Err(e) => {
            debug!("Cannot read projects dir {}: {}", projects_dir.display(), e);
            return Vec::new();
        }
    };

    let mut results = Vec::new();

    for project_entry in read_dir.flatten() {
        let project_path = project_entry.path();
        if !project_path.is_dir() {
            continue;
        }

        let project_dir_name = project_entry.file_name().to_string_lossy().to_string();

        let project_dir = match fs::read_dir(&project_path) {
            Ok(rd) => rd,
            Err(_) => continue,
        };

        for file_entry in project_dir.flatten() {
            let path = file_entry.path();
            let name = file_entry.file_name().to_string_lossy().to_string();

            if !name.ends_with(".jsonl") {
                continue;
            }

            // Fast mtime pre-filter (avoids reading old files).
            if let Some(cutoff) = mtime_cutoff {
                let metadata = match fs::metadata(&path) {
                    Ok(m) => m,
                    Err(_) => continue,
                };
                let modified_time = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);
                let modified_dt: DateTime<Utc> = modified_time.into();
                if modified_dt < cutoff {
                    continue;
                }
            }

            let session_id = name.trim_end_matches(".jsonl").to_string();
            let session = parse_jsonl_session(&session_id, &path, &project_dir_name);
            results.push(session);
        }
    }

    debug!(
        "collect_claude_sessions: found {} sessions under {}",
        results.len(),
        projects_dir.display()
    );

    results
}

/// Parse a single `.jsonl` file into a `ClaudeSession`.
fn parse_jsonl_session(session_id: &str, path: &Path, project_dir_name: &str) -> ClaudeSession {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => {
            return ClaudeSession {
                session_id: session_id.to_string(),
                timestamps: Vec::new(),
                messages: Vec::new(),
                project_path: None,
                git_branch: None,
                first_prompt: None,
                summary: None,
            };
        }
    };

    let mut messages: Vec<ClaudeMessage> = Vec::new();
    let mut timestamps: Vec<DateTime<Utc>> = Vec::new();
    let mut project_path: Option<String> = None;
    let mut git_branch: Option<String> = None;
    let mut first_prompt: Option<String> = None;
    let mut summary: Option<String> = None;
    let mut first_user_seen = false;

    for line in content.lines() {
        let obj = match serde_json::from_str::<serde_json::Value>(line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let msg_type = match obj.get("type").and_then(|t| t.as_str()) {
            Some(t) => t,
            None => continue,
        };

        // Parse timestamp — skip events with no parseable timestamp.
        let ts: Option<DateTime<Utc>> = obj
            .get("timestamp")
            .and_then(|t| t.as_str())
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc));

        match msg_type {
            "user" => {
                if !first_user_seen {
                    first_user_seen = true;
                    git_branch = obj
                        .get("gitBranch")
                        .and_then(|b| b.as_str())
                        .map(|s| s.to_string());

                    project_path = obj
                        .get("cwd")
                        .and_then(|c| c.as_str())
                        .map(|s| s.to_string())
                        .or_else(|| derive_path_from_dir_name(project_dir_name));

                    // Extract first prompt text.
                    let msg_content = obj.pointer("/message/content");
                    first_prompt = extract_text_content(msg_content);
                }

                if let Some(t) = ts {
                    timestamps.push(t);
                    messages.push(ClaudeMessage {
                        timestamp: t,
                        msg_type: "user".to_string(),
                        model: None,
                        usage: TokenUsage::default(),
                        tool_calls: 0,
                    });
                }
            }
            "assistant" => {
                let model = obj
                    .pointer("/message/model")
                    .and_then(|m| m.as_str())
                    .map(|s| s.to_string());

                let usage = if let Some(u) = obj.pointer("/message/usage") {
                    TokenUsage {
                        input: u.get("input_tokens").and_then(|v| v.as_u64()).unwrap_or(0),
                        output: u.get("output_tokens").and_then(|v| v.as_u64()).unwrap_or(0),
                        cache_read: u
                            .get("cache_read_input_tokens")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0),
                        cache_creation: u
                            .get("cache_creation_input_tokens")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0),
                    }
                } else {
                    TokenUsage::default()
                };

                let tool_calls = obj
                    .pointer("/message/content")
                    .and_then(|content| content.as_array())
                    .map(|items| {
                        items
                            .iter()
                            .filter(|item| {
                                item.get("type").and_then(|t| t.as_str()) == Some("tool_use")
                            })
                            .count() as u64
                    })
                    .unwrap_or(0);

                if let Some(t) = ts {
                    timestamps.push(t);
                    messages.push(ClaudeMessage {
                        timestamp: t,
                        msg_type: "assistant".to_string(),
                        model,
                        usage,
                        tool_calls,
                    });
                }
            }
            "summary" => {
                summary = obj
                    .get("summary")
                    .and_then(|s| s.as_str())
                    .map(|s| s.to_string());
            }
            _ => {}
        }
    }

    ClaudeSession {
        session_id: session_id.to_string(),
        timestamps,
        messages,
        project_path,
        git_branch,
        first_prompt,
        summary,
    }
}

/// Derive a Unix-style path from the Claude project directory naming convention.
/// Directories are named by replacing `/` with `-` in the path, with a leading `-`.
/// Example: `-Users-name-dev-project` → `/Users/name/dev/project`
fn derive_path_from_dir_name(dir_name: &str) -> Option<String> {
    if dir_name.starts_with('-') {
        Some(dir_name.replacen('-', "/", 1).replace('-', "/"))
    } else {
        None
    }
}

/// Extract a plain-text string from a JSONL message content field.
/// Handles both `String` and `Array<{type:"text",text:"..."}>`  formats.
fn extract_text_content(content: Option<&serde_json::Value>) -> Option<String> {
    match content {
        Some(serde_json::Value::String(s)) => Some(s.chars().take(120).collect()),
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write_session(dir: &Path, session_id: &str, lines: &str) {
        let project_dir = dir.join("-Users-test-proj");
        fs::create_dir_all(&project_dir).unwrap();
        fs::write(project_dir.join(format!("{session_id}.jsonl")), lines).unwrap();
    }

    #[test]
    fn test_collect_parses_tokens() {
        let tmp = TempDir::new().unwrap();
        let now = Utc::now();
        let ts = now.to_rfc3339();

        write_session(
            tmp.path(),
            "sess-1",
            &format!(
                concat!(
                    r#"{{"type":"user","timestamp":"{ts}","cwd":"/Users/test/proj","gitBranch":"main","message":{{"role":"user","content":"hello"}}}}"#,
                    "\n",
                    r#"{{"type":"assistant","timestamp":"{ts}","message":{{"model":"claude-opus-4-6","usage":{{"input_tokens":100,"output_tokens":50,"cache_read_input_tokens":200,"cache_creation_input_tokens":300}}}}}}"#,
                ),
                ts = ts
            ),
        );

        let sessions = collect_claude_sessions(tmp.path(), None);
        assert_eq!(sessions.len(), 1);

        let s = &sessions[0];
        let totals = s.total_tokens();
        assert_eq!(totals.input, 100);
        assert_eq!(totals.output, 50);
        assert_eq!(totals.cache_read, 200);
        assert_eq!(totals.cache_creation, 300);
        assert_eq!(s.primary_model(), Some("claude-opus-4-6"));
        assert_eq!(s.git_branch.as_deref(), Some("main"));
        assert_eq!(s.project_path.as_deref(), Some("/Users/test/proj"));
        assert_eq!(s.first_prompt.as_deref(), Some("hello"));
    }

    #[test]
    fn test_mtime_cutoff_filters_old_files() {
        let tmp = TempDir::new().unwrap();
        write_session(
            tmp.path(),
            "old-sess",
            r#"{"type":"user","timestamp":"2020-01-01T00:00:00Z","message":{"role":"user","content":"hi"}}"#,
        );

        // Cutoff of "now" should exclude a file with old mtime (files just written have current mtime).
        // Use a future cutoff to force exclusion.
        let future = Utc::now() + chrono::Duration::days(1);
        let sessions = collect_claude_sessions(tmp.path(), Some(future));
        assert!(
            sessions.is_empty(),
            "future cutoff should exclude all files"
        );
    }

    #[test]
    fn test_empty_dir_returns_empty() {
        let tmp = TempDir::new().unwrap();
        let sessions = collect_claude_sessions(tmp.path(), None);
        assert!(sessions.is_empty());
    }

    #[test]
    fn test_nonexistent_dir_returns_empty() {
        let sessions =
            collect_claude_sessions(Path::new("/tmp/__localpush_nonexistent_test_dir__"), None);
        assert!(sessions.is_empty());
    }

    #[test]
    fn test_multiple_messages_accumulate() {
        let tmp = TempDir::new().unwrap();
        let now = Utc::now();
        let ts = now.to_rfc3339();

        write_session(
            tmp.path(),
            "multi",
            &format!(
                concat!(
                    r#"{{"type":"assistant","timestamp":"{ts}","message":{{"model":"claude-opus-4-6","usage":{{"input_tokens":100,"output_tokens":50,"cache_read_input_tokens":0,"cache_creation_input_tokens":0}}}}}}"#,
                    "\n",
                    r#"{{"type":"assistant","timestamp":"{ts}","message":{{"model":"claude-opus-4-6","usage":{{"input_tokens":200,"output_tokens":100,"cache_read_input_tokens":0,"cache_creation_input_tokens":0}}}}}}"#,
                ),
                ts = ts
            ),
        );

        let sessions = collect_claude_sessions(tmp.path(), None);
        assert_eq!(sessions.len(), 1);
        let totals = sessions[0].total_tokens();
        assert_eq!(totals.input, 300);
        assert_eq!(totals.output, 150);
    }

    #[test]
    fn test_derive_path_from_dir_name() {
        assert_eq!(
            derive_path_from_dir_name("-Users-name-dev-project"),
            Some("/Users/name/dev/project".to_string())
        );
        assert_eq!(derive_path_from_dir_name("notdash"), None);
    }
}
