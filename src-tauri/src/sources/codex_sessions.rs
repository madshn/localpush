use super::{PreviewField, Source, SourceError, SourcePreview};
use crate::source_config::PropertyDef;
use chrono::{DateTime, Duration, Utc};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default)]
pub struct CodexTokenUsage {
    pub input: u64,
    pub cached_input: u64,
    pub output: u64,
    pub reasoning_output: u64,
    pub total: u64,
}

impl CodexTokenUsage {
    fn from_value(v: &Value) -> Option<Self> {
        Some(Self {
            input: v.get("input_tokens")?.as_u64()?,
            cached_input: v
                .get("cached_input_tokens")
                .and_then(|x| x.as_u64())
                .unwrap_or(0),
            output: v.get("output_tokens")?.as_u64()?,
            reasoning_output: v
                .get("reasoning_output_tokens")
                .and_then(|x| x.as_u64())
                .unwrap_or(0),
            total: v.get("total_tokens")?.as_u64()?,
        })
    }

    pub(crate) fn saturating_delta(&self, prev: &Self) -> Self {
        Self {
            input: self.input.saturating_sub(prev.input),
            cached_input: self.cached_input.saturating_sub(prev.cached_input),
            output: self.output.saturating_sub(prev.output),
            reasoning_output: self.reasoning_output.saturating_sub(prev.reasoning_output),
            total: self.total.saturating_sub(prev.total),
        }
    }

    pub(crate) fn add_assign(&mut self, other: &Self) {
        self.input += other.input;
        self.cached_input += other.cached_input;
        self.output += other.output;
        self.reasoning_output += other.reasoning_output;
        self.total += other.total;
    }
}

#[derive(Debug, Clone)]
pub struct CodexTokenSnapshot {
    pub timestamp: DateTime<Utc>,
    pub total_usage: CodexTokenUsage,
    pub last_usage: Option<CodexTokenUsage>,
}

#[derive(Debug, Clone)]
pub struct CodexSessionRecord {
    pub id: String,
    pub file_path: String,
    pub project_path: Option<String>,
    pub git_branch: Option<String>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub session_span_seconds: Option<i64>,
    pub agentic_seconds: Option<i64>,
    pub message_count: u32,
    pub title: Option<String>,
    pub model: Option<String>,
    pub token_totals: CodexTokenUsage,
    pub token_snapshots: Vec<CodexTokenSnapshot>,
    pub earliest_event_ts: Option<DateTime<Utc>>,
    pub latest_event_ts: Option<DateTime<Utc>>,
}

fn parse_ts(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

fn format_number(n: u64) -> String {
    n.to_string()
        .as_bytes()
        .rchunks(3)
        .rev()
        .map(std::str::from_utf8)
        .collect::<Result<Vec<&str>, _>>()
        .unwrap_or_default()
        .join(",")
}

fn walk_jsonl_files(root: &Path, out: &mut Vec<PathBuf>) {
    let read_dir = match fs::read_dir(root) {
        Ok(rd) => rd,
        Err(_) => return,
    };

    for entry in read_dir.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk_jsonl_files(&path, out);
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) == Some("jsonl") {
            out.push(path);
        }
    }
}

fn session_id_from_filename(path: &Path) -> String {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown-session");
    stem.rsplit_once('-')
        .map(|(_, tail)| tail.to_string())
        .unwrap_or_else(|| stem.to_string())
}

fn derive_title_from_value(v: &Value) -> Option<String> {
    let s = match v {
        Value::String(s) => s,
        _ => return None,
    };
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(trimmed.chars().take(120).collect())
}

fn normalize_model_key_parts(model_id: &str) -> (String, String, String) {
    let model = model_id.trim().to_lowercase();
    let sanitize_version = |s: &str| s.replace(['.', '-'], "_");

    if let Some(stripped) = model.strip_prefix("claude-") {
        let without_date = if stripped.len() > 9
            && stripped
                .rsplit_once('-')
                .is_some_and(|(_, last)| last.len() == 8 && last.chars().all(|c| c.is_ascii_digit()))
        {
            stripped
                .rsplit_once('-')
                .map(|(head, _)| head)
                .unwrap_or(stripped)
        } else {
            stripped
        };
        let mut parts = without_date.split('-');
        let family = parts.next().unwrap_or("claude").to_string();
        let version = sanitize_version(&parts.collect::<Vec<_>>().join("-"));
        return ("anthropic".into(), family, if version.is_empty() { "unknown".into() } else { version });
    }

    if model.starts_with("gpt-") && model.ends_with("-codex") {
        let version = model
            .trim_start_matches("gpt-")
            .trim_end_matches("-codex");
        return ("openai".into(), "codex".into(), sanitize_version(version));
    }

    if let Some(version) = model.strip_prefix("gpt-") {
        return ("openai".into(), "gpt".into(), sanitize_version(version));
    }

    if model.starts_with('o') && model.chars().nth(1).is_some_and(|c| c.is_ascii_digit()) {
        let version = model.trim_start_matches('o');
        return ("openai".into(), "o".into(), sanitize_version(version));
    }

    if let Some(version) = model.strip_prefix("gemini-") {
        return ("google".into(), "gemini".into(), sanitize_version(version));
    }

    let fallback: String = model
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect();
    ("unknown".into(), fallback, "unknown".into())
}

pub(crate) fn normalize_model_key(model_id: &str) -> String {
    let (vendor, family, version) = normalize_model_key_parts(model_id);
    format!("{vendor}.{family}.{version}")
}

fn estimate_agentic_seconds(
    start_dt: Option<DateTime<Utc>>,
    end_dt: Option<DateTime<Utc>>,
    snapshots: &[CodexTokenSnapshot],
) -> Option<i64> {
    // Estimate active machine time by summing event gaps < 5 minutes.
    // Uses token_count snapshots as the best available proxy for AI processing activity.
    const THRESHOLD_SECS: i64 = 300;

    if snapshots.is_empty() {
        return None;
    }

    let mut seconds = 0_i64;
    let mut prev = start_dt.unwrap_or(snapshots[0].timestamp);
    for snap in snapshots {
        let gap = (snap.timestamp - prev).num_seconds().max(0);
        if gap < THRESHOLD_SECS {
            seconds += gap;
        }
        prev = snap.timestamp;
    }
    if let Some(end) = end_dt {
        let gap = (end - prev).num_seconds().max(0);
        if gap < THRESHOLD_SECS {
            seconds += gap;
        }
    }
    Some(seconds)
}

pub(crate) fn parse_codex_session_file(path: &Path) -> Result<CodexSessionRecord, SourceError> {
    let content = fs::read_to_string(path)?;
    let mut session_id = session_id_from_filename(path);
    let mut project_path: Option<String> = None;
    let mut git_branch: Option<String> = None;
    let mut start_ts_meta: Option<String> = None;
    let mut earliest_event_ts: Option<DateTime<Utc>> = None;
    let mut latest_event_ts: Option<DateTime<Utc>> = None;
    let mut message_count: u32 = 0;
    let mut title: Option<String> = None;
    let mut model_last: Option<String> = None;
    let mut model_counts: HashMap<String, u32> = HashMap::new();
    let mut max_total = CodexTokenUsage::default();
    let mut token_snapshots: Vec<CodexTokenSnapshot> = Vec::new();

    for line in content.lines() {
        let obj: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let ts = obj
            .get("timestamp")
            .and_then(|v| v.as_str())
            .and_then(parse_ts);
        if let Some(ts) = ts {
            if earliest_event_ts.is_none_or(|e| ts < e) {
                earliest_event_ts = Some(ts);
            }
            if latest_event_ts.is_none_or(|e| ts > e) {
                latest_event_ts = Some(ts);
            }
        }

        let top_type = obj.get("type").and_then(|v| v.as_str());
        let payload = obj.get("payload").and_then(|v| v.as_object());

        match top_type {
            Some("session_meta") => {
                if let Some(p) = payload {
                    if let Some(id) = p.get("id").and_then(|v| v.as_str()) {
                        session_id = id.to_string();
                    }
                    project_path = p
                        .get("cwd")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                        .or(project_path);
                    start_ts_meta = p
                        .get("timestamp")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                        .or(start_ts_meta);
                    git_branch = p
                        .get("git")
                        .and_then(|g| g.get("branch"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                        .or(git_branch);
                }
            }
            Some("turn_context") => {
                if let Some(p) = payload {
                    if let Some(model) = p.get("model").and_then(|v| v.as_str()) {
                        model_last = Some(model.to_string());
                        *model_counts.entry(model.to_string()).or_insert(0) += 1;
                    }
                }
            }
            Some("event_msg") => {
                if let Some(p) = payload {
                    match p.get("type").and_then(|v| v.as_str()) {
                        Some("user_message") => {
                            message_count += 1;
                            if title.is_none() {
                                title = p.get("message").and_then(derive_title_from_value);
                            }
                        }
                        Some("token_count") => {
                            let Some(info) = p.get("info") else { continue };
                            let Some(ts) = obj
                                .get("timestamp")
                                .and_then(|v| v.as_str())
                                .and_then(parse_ts)
                            else {
                                continue;
                            };
                            let Some(total_usage) = info
                                .get("total_token_usage")
                                .and_then(CodexTokenUsage::from_value)
                            else {
                                continue;
                            };
                            let last_usage =
                                info.get("last_token_usage").and_then(CodexTokenUsage::from_value);
                            if total_usage.total >= max_total.total {
                                max_total = total_usage.clone();
                            }
                            token_snapshots.push(CodexTokenSnapshot {
                                timestamp: ts,
                                total_usage,
                                last_usage,
                            });
                        }
                        Some("agent_message") => {}
                        _ => {}
                    }
                }
            }
            Some("response_item") => {
                if let Some(p) = payload {
                    if p.get("type").and_then(|v| v.as_str()) == Some("message")
                        && p.get("role").and_then(|v| v.as_str()) == Some("user")
                        && title.is_none()
                    {
                        if let Some(content) = p.get("content") {
                            match content {
                                Value::String(_) => {
                                    title = derive_title_from_value(content);
                                }
                                Value::Array(arr) => {
                                    for item in arr {
                                        if item.get("type").and_then(|v| v.as_str())
                                            == Some("input_text")
                                        {
                                            title =
                                                item.get("text").and_then(derive_title_from_value);
                                            if title.is_some() {
                                                break;
                                            }
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    token_snapshots.sort_by_key(|s| s.timestamp);

    let model = if model_counts.is_empty() {
        model_last
    } else {
        model_counts
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(m, _)| m)
            .or(model_last)
    };

    let start_dt = start_ts_meta
        .as_deref()
        .and_then(parse_ts)
        .or(earliest_event_ts);
    let end_dt = latest_event_ts;
    let session_span_seconds = match (start_dt, end_dt) {
        (Some(s), Some(e)) => Some((e - s).num_seconds().max(0)),
        _ => None,
    };
    let agentic_seconds = estimate_agentic_seconds(start_dt, end_dt, &token_snapshots);

    Ok(CodexSessionRecord {
        id: session_id,
        file_path: path.display().to_string(),
        project_path,
        git_branch,
        start_time: start_dt.map(|d| d.to_rfc3339()),
        end_time: end_dt.map(|d| d.to_rfc3339()),
        session_span_seconds,
        agentic_seconds,
        message_count,
        title,
        model,
        token_totals: max_total,
        token_snapshots,
        earliest_event_ts,
        latest_event_ts,
    })
}

pub(crate) fn collect_codex_sessions(
    root: &Path,
    recent_within_days: Option<i64>,
) -> Vec<CodexSessionRecord> {
    let mut files = Vec::new();
    walk_jsonl_files(root, &mut files);
    files.sort();

    let cutoff = recent_within_days.map(|days| Utc::now() - Duration::days(days));
    let mut sessions = Vec::new();

    for path in files {
        match parse_codex_session_file(&path) {
            Ok(session) => {
                if let Some(cutoff) = cutoff {
                    let modified = session
                        .end_time
                        .as_deref()
                        .and_then(parse_ts)
                        .or(session.latest_event_ts);
                    if modified.is_some_and(|ts| ts < cutoff) {
                        continue;
                    }
                }
                sessions.push(session);
            }
            Err(_) => continue,
        }
    }

    sessions.sort_by(|a, b| b.end_time.cmp(&a.end_time));
    sessions
}

pub struct CodexSessionsSource {
    sessions_root: PathBuf,
    recent_within_days: Option<i64>,
}

impl CodexSessionsSource {
    pub fn new() -> Result<Self, SourceError> {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .map_err(|_| SourceError::ParseError("Could not determine home directory".into()))?;
        Ok(Self {
            sessions_root: PathBuf::from(home).join(".codex").join("sessions"),
            recent_within_days: Some(7),
        })
    }

    pub fn new_with_path(path: impl Into<PathBuf>) -> Self {
        Self {
            sessions_root: path.into(),
            recent_within_days: None,
        }
    }

    fn sessions(&self) -> Vec<CodexSessionRecord> {
        collect_codex_sessions(&self.sessions_root, self.recent_within_days)
    }
}

impl Source for CodexSessionsSource {
    fn id(&self) -> &str {
        "codex-sessions"
    }

    fn name(&self) -> &str {
        "Codex Sessions"
    }

    fn watch_path(&self) -> Option<PathBuf> {
        Some(self.sessions_root.clone())
    }

    fn watch_recursive(&self) -> bool {
        true
    }

    fn parse(&self) -> Result<Value, SourceError> {
        let sessions = self.sessions();

        let session_values: Vec<Value> = sessions
            .iter()
            .map(|s| {
                serde_json::json!({
                    "id": s.id,
                    "project_path": s.project_path,
                    "git_branch": s.git_branch,
                    "title": s.title.clone().unwrap_or_else(|| "Codex session".to_string()),
                    "start_time": s.start_time,
                    "end_time": s.end_time,
                    "session_span_seconds": s.session_span_seconds,
                    "agentic_seconds": s.agentic_seconds,
                    "message_count": s.message_count,
                    "tokens": {
                        "input": s.token_totals.input,
                        "output": s.token_totals.output,
                        "cache_read": s.token_totals.cached_input,
                        "cache_creation": 0_u64,
                        "reasoning_output": s.token_totals.reasoning_output,
                    },
                    "model": s.model.as_deref().map(normalize_model_key),
                })
            })
            .collect();

        let mut sum = CodexTokenUsage::default();
        let total_duration_seconds: i64 = sessions
            .iter()
            .filter_map(|s| s.session_span_seconds)
            .sum();
        let total_agentic_seconds: i64 = sessions.iter().filter_map(|s| s.agentic_seconds).sum();
        for s in &sessions {
            sum.add_assign(&s.token_totals);
        }

        Ok(serde_json::json!({
            "source": "codex_sessions",
            "timestamp": Utc::now().to_rfc3339(),
            "schema_version": 1,
            "source_family": "codex",
            "source_type": "sessions",
            "semantics": {
                "token_count_basis": "session_max_of_event_msg.token_count.info.total_token_usage",
                "message_count_basis": "count(event_msg.user_message)",
                "duration_basis": "session_meta.timestamp_to_last_event_timestamp",
                "dedupe_basis": "one_record_per_jsonl_session_file",
                "window": {
                    "mode": if self.recent_within_days.is_some() { "recent_days" } else { "all_in_path" },
                    "days": self.recent_within_days,
                },
                "unsupported_metrics": ["cache_creation_tokens"],
                "notes": [
                    "cache_read maps to Codex cached_input_tokens",
                    "reasoning_output is included inside tokens for schema parity",
                    "agentic_seconds is an estimate using token_count event gaps < 5 minutes"
                ]
            },
            "sessions": session_values,
            "summary": {
                "sessions_count": sessions.len(),
                "total_tokens": sum.total,
                "total_duration_seconds": total_duration_seconds,
                "total_agentic_seconds": total_agentic_seconds,
                "total_input_tokens": sum.input,
                "total_output_tokens": sum.output,
                "total_cached_input_tokens": sum.cached_input,
                "total_reasoning_output_tokens": sum.reasoning_output,
            }
        }))
    }

    fn preview(&self) -> Result<SourcePreview, SourceError> {
        let sessions = self.sessions();
        let total_tokens: u64 = sessions.iter().map(|s| s.token_totals.total).sum();
        let summary = if sessions.is_empty() {
            "No Codex sessions found".to_string()
        } else {
            format!("{} sessions, {} tokens", sessions.len(), format_number(total_tokens))
        };

        let mut fields = vec![
            PreviewField {
                label: "Sessions".into(),
                value: sessions.len().to_string(),
                sensitive: false,
            },
            PreviewField {
                label: "Total Tokens".into(),
                value: format_number(total_tokens),
                sensitive: false,
            },
        ];
        if let Some(latest) = sessions.first() {
            fields.push(PreviewField {
                label: "Latest Session".into(),
                value: latest.title.clone().unwrap_or_else(|| "Codex session".into()),
                sensitive: false,
            });
            if let Some(project) = &latest.project_path {
                fields.push(PreviewField {
                    label: "Project".into(),
                    value: project.clone(),
                    sensitive: false,
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
                key: "sessions".into(),
                label: "Sessions".into(),
                description: "Session list with token totals and context".into(),
                default_enabled: true,
                privacy_sensitive: true,
            },
            PropertyDef {
                key: "summary".into(),
                label: "Summary".into(),
                description: "Aggregated token and session totals".into(),
                default_enabled: true,
                privacy_sensitive: false,
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn fixture_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/codex/2026-02-23/raw/sessions")
    }

    fn fixture_base() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/codex/2026-02-23")
    }

    fn normalize(mut v: Value) -> Value {
        if let Some(obj) = v.as_object_mut() {
            obj.remove("timestamp");
        }
        v
    }

    #[test]
    fn test_parse_codex_fixture_sessions_basic() {
        let source = CodexSessionsSource::new_with_path(fixture_dir());
        let payload = source.parse().unwrap();
        let sessions = payload["sessions"].as_array().unwrap();
        assert_eq!(sessions.len(), 6);
        assert_eq!(payload["source"], "codex_sessions");
        assert_eq!(payload["source_family"], "codex");
        assert_eq!(payload["source_type"], "sessions");
        assert!(payload["summary"]["total_tokens"].as_u64().unwrap() > 0);
    }

    #[test]
    fn test_codex_sessions_fixture_matches_expected_golden() {
        let source = CodexSessionsSource::new_with_path(fixture_dir());
        let actual = normalize(source.parse().unwrap());
        let expected_path = fixture_base().join("expected/codex-sessions.json");
        let expected: Value = serde_json::from_str(&fs::read_to_string(expected_path).unwrap()).unwrap();
        if expected.get("_status").and_then(|v| v.as_str()) == Some("pending") {
            // Placeholder is allowed before goldens are generated.
            return;
        }
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_parse_single_codex_file_has_token_snapshots() {
        let mut files = Vec::new();
        walk_jsonl_files(&fixture_dir(), &mut files);
        files.sort();
        let record = parse_codex_session_file(&files[0]).unwrap();
        assert!(!record.token_snapshots.is_empty());
        assert!(record.token_totals.total > 0);
        assert!(record.message_count > 0);
    }
}
