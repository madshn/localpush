use super::{recursive_path_change_hint, PreviewField, Source, SourceError, SourcePreview};
use crate::config::AppConfig;
use crate::source_config::{window_setting_for_source, PropertyDef, SourceConfigStore};
use crate::sources::claude_sessions_collector::collect_claude_sessions;
use chrono::{DateTime, Duration, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::Arc;
use tracing::info;

/// Structured payload sent to webhooks
#[derive(Debug, Serialize, Deserialize)]
pub struct ClaudeStatsPayload {
    pub version: u32,
    pub last_computed_date: String,
    pub today: Option<DailyStats>,
    pub yesterday: Option<DailyStats>,
    /// Rolling breakdown across the configured window, zero-filled for inactive days (oldest → newest)
    pub daily_breakdown: Vec<DailyStats>,
    pub model_totals: Vec<ModelTotal>,
    pub summary: SummaryStats,
    pub metadata: PayloadMetadata,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DailyStats {
    pub date: String,
    pub messages: u64,
    pub sessions: u64,
    pub tool_calls: u64,
    pub tokens_by_model: HashMap<String, u64>,
    pub total_tokens: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelTotal {
    pub model: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_creation_tokens: u64,
    pub total_tokens: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SummaryStats {
    pub total_sessions: u64,
    pub total_messages: u64,
    pub first_session_date: Option<String>,
    pub days_active: usize,
    /// Always None — hour-of-day data is not available from JSONL.
    pub peak_hour: Option<u8>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PayloadMetadata {
    pub source: String,
    pub generated_at: DateTime<Utc>,
    pub file_path: String,
}

/// Daily accumulator used during aggregation.
#[derive(Default)]
struct DayBucket {
    messages: u64,
    tool_calls: u64,
    tokens_by_model: HashMap<String, u64>,
}

/// Claude Code statistics source.
///
/// Reads session data directly from `~/.claude/projects/*/*.jsonl` using a configurable
/// recent-day window.
/// This replaces the old `stats-cache.json` approach, which stopped auto-updating
/// in Claude Code v2.1.45.
pub struct ClaudeStatsSource {
    claude_projects_dir: PathBuf,
    config: Option<Arc<AppConfig>>,
    reference_now: Option<DateTime<Utc>>,
}

impl ClaudeStatsSource {
    pub fn new(config: Arc<AppConfig>) -> Result<Self, SourceError> {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .map_err(|_| {
                SourceError::ParseError("Could not determine home directory".to_string())
            })?;

        let claude_projects_dir = PathBuf::from(home).join(".claude").join("projects");

        Ok(Self {
            claude_projects_dir,
            config: Some(config),
            reference_now: None,
        })
    }

    /// Constructor with custom projects directory (for testing).
    pub fn new_with_path(path: impl Into<PathBuf>) -> Self {
        Self {
            claude_projects_dir: path.into(),
            config: None,
            reference_now: None,
        }
    }

    /// Constructor with custom path and pinned reference time (for deterministic tests).
    #[cfg(test)]
    pub fn new_with_path_and_now(path: impl Into<PathBuf>, reference_now: DateTime<Utc>) -> Self {
        Self {
            claude_projects_dir: path.into(),
            config: None,
            reference_now: Some(reference_now),
        }
    }

    #[cfg(test)]
    pub fn new_with_path_config_and_now(
        path: impl Into<PathBuf>,
        config: Arc<AppConfig>,
        reference_now: DateTime<Utc>,
    ) -> Self {
        Self {
            claude_projects_dir: path.into(),
            config: Some(config),
            reference_now: Some(reference_now),
        }
    }

    pub fn new_without_config() -> Result<Self, SourceError> {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .map_err(|_| {
                SourceError::ParseError("Could not determine home directory".to_string())
            })?;

        Ok(Self {
            claude_projects_dir: PathBuf::from(home).join(".claude").join("projects"),
            config: None,
            reference_now: None,
        })
    }

    fn now(&self) -> DateTime<Utc> {
        self.reference_now.unwrap_or_else(Utc::now)
    }

    fn today_date(&self) -> NaiveDate {
        self.now().date_naive()
    }

    fn window_days(&self) -> usize {
        let Some(config) = &self.config else {
            return 30;
        };
        let def = window_setting_for_source(self.id())
            .expect("claude-stats should have a window setting definition");
        SourceConfigStore::new(config.clone()).get_window_days(self.id(), &def) as usize
    }

    /// Format number with comma separators (e.g. 1_234_567 → "1,234,567").
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

    /// Calculate percentage change from yesterday to today.
    fn percentage_change(today: u64, yesterday: u64) -> Option<f64> {
        if yesterday == 0 {
            return None;
        }
        let change = ((today as f64 - yesterday as f64) / yesterday as f64) * 100.0;
        Some(change)
    }

    /// Aggregate JSONL sessions into per-day buckets and per-model totals.
    ///
    /// Returns `(day_buckets, day_session_ids, model_map, earliest_date)`.
    #[allow(clippy::type_complexity)]
    fn aggregate(
        &self,
    ) -> (
        BTreeMap<String, DayBucket>,
        BTreeMap<String, std::collections::BTreeSet<String>>,
        BTreeMap<String, ModelAcc>,
        Option<String>,
    ) {
        let cutoff = self.now() - Duration::days(self.window_days() as i64);
        let sessions = collect_claude_sessions(&self.claude_projects_dir, Some(cutoff));

        info!(
            "claude-stats: aggregating {} sessions from JSONL",
            sessions.len()
        );

        let mut day_buckets: BTreeMap<String, DayBucket> = BTreeMap::new();
        // Track unique session IDs per day (to avoid double-counting a session that spans midnight).
        let mut day_session_ids: BTreeMap<String, std::collections::BTreeSet<String>> =
            BTreeMap::new();
        let mut model_map: BTreeMap<String, ModelAcc> = BTreeMap::new();
        let mut earliest_date: Option<String> = None;

        for session in &sessions {
            // Attribute each message to the calendar day of its timestamp.
            let mut session_days: std::collections::BTreeSet<String> =
                std::collections::BTreeSet::new();

            for msg in &session.messages {
                // Skip messages outside the configured recent-day window. The file-level mtime
                // filter only checks if the *file* was touched recently; a
                // long-lived session file may contain messages far older than the
                // cutoff.
                if msg.timestamp < cutoff {
                    continue;
                }
                let day = msg.timestamp.date_naive().format("%Y-%m-%d").to_string();
                session_days.insert(day.clone());

                let bucket = day_buckets.entry(day.clone()).or_default();
                bucket.messages += 1;
                bucket.tool_calls += msg.tool_calls;

                // Update earliest seen date.
                if earliest_date
                    .as_ref()
                    .map(|e: &String| day < *e)
                    .unwrap_or(true)
                {
                    earliest_date = Some(day.clone());
                }

                // Accumulate per-model totals from assistant messages only.
                if msg.msg_type == "assistant" {
                    if let Some(ref model) = msg.model {
                        // Skip synthetic placeholder messages (error responses, no-ops)
                        if model == "<synthetic>" {
                            continue;
                        }
                        let acc = model_map.entry(model.clone()).or_default();
                        acc.input += msg.usage.input;
                        acc.output += msg.usage.output;
                        acc.cache_read += msg.usage.cache_read;
                        acc.cache_creation += msg.usage.cache_creation;
                        // tokens_by_model tracks total (input + output) per model per day
                        let total = msg.usage.input + msg.usage.output;
                        *bucket.tokens_by_model.entry(model.clone()).or_insert(0) += total;
                    }
                }
            }

            // Record session attribution per day.
            for day in session_days {
                day_session_ids
                    .entry(day)
                    .or_default()
                    .insert(session.session_id.clone());
            }
        }

        (day_buckets, day_session_ids, model_map, earliest_date)
    }

    /// Build a rolling daily breakdown with zero-filled gaps (oldest → newest).
    fn build_daily_breakdown(
        &self,
        day_buckets: &BTreeMap<String, DayBucket>,
        day_session_ids: &BTreeMap<String, std::collections::BTreeSet<String>>,
        window_days: usize,
    ) -> Vec<DailyStats> {
        let today = self.today_date();
        let mut breakdown = Vec::with_capacity(window_days);

        for i in (0..window_days).rev() {
            let date = today - Duration::days(i as i64);
            let date_str = date.format("%Y-%m-%d").to_string();

            let daily = match day_buckets.get(&date_str) {
                Some(bucket) => {
                    let sessions = day_session_ids
                        .get(&date_str)
                        .map(|s| s.len() as u64)
                        .unwrap_or(0);
                    let total_tokens: u64 = bucket.tokens_by_model.values().sum();
                    DailyStats {
                        date: date_str,
                        messages: bucket.messages,
                        sessions,
                        tool_calls: bucket.tool_calls,
                        tokens_by_model: bucket.tokens_by_model.clone(),
                        total_tokens,
                    }
                }
                None => DailyStats {
                    date: date_str,
                    messages: 0,
                    sessions: 0,
                    tool_calls: 0,
                    tokens_by_model: HashMap::new(),
                    total_tokens: 0,
                },
            };

            breakdown.push(daily);
        }

        breakdown
    }
}

/// Per-model token accumulator used during aggregation.
#[derive(Default)]
struct ModelAcc {
    input: u64,
    output: u64,
    cache_read: u64,
    cache_creation: u64,
}

impl Default for ClaudeStatsSource {
    fn default() -> Self {
        Self::new_without_config().expect("Failed to initialize ClaudeStatsSource")
    }
}

impl Source for ClaudeStatsSource {
    fn id(&self) -> &str {
        "claude-stats"
    }

    fn name(&self) -> &str {
        "Claude Code Statistics"
    }

    fn watch_path(&self) -> Option<PathBuf> {
        Some(self.claude_projects_dir.clone())
    }

    fn watch_recursive(&self) -> bool {
        true
    }

    fn delivery_change_hint(&self) -> Result<Option<String>, SourceError> {
        Ok(
            recursive_path_change_hint(&self.claude_projects_dir, None)?.map(|hint| {
                format!(
                    "day:{}:window:{}:{}",
                    self.today_date().format("%Y-%m-%d"),
                    self.window_days(),
                    hint
                )
            }),
        )
    }

    fn parse(&self) -> Result<serde_json::Value, SourceError> {
        let (day_buckets, day_session_ids, model_map, earliest_date) = self.aggregate();

        let today_str = self.today_date().format("%Y-%m-%d").to_string();
        let yesterday_str = (self.today_date() - Duration::days(1))
            .format("%Y-%m-%d")
            .to_string();

        let make_daily = |date_str: &str| -> Option<DailyStats> {
            day_buckets.get(date_str).map(|bucket| {
                let sessions = day_session_ids
                    .get(date_str)
                    .map(|s| s.len() as u64)
                    .unwrap_or(0);
                let total_tokens: u64 = bucket.tokens_by_model.values().sum();
                DailyStats {
                    date: date_str.to_string(),
                    messages: bucket.messages,
                    sessions,
                    tool_calls: bucket.tool_calls,
                    tokens_by_model: bucket.tokens_by_model.clone(),
                    total_tokens,
                }
            })
        };

        let today = make_daily(&today_str);
        let yesterday = make_daily(&yesterday_str);
        let window_days = self.window_days();
        let daily_breakdown =
            self.build_daily_breakdown(&day_buckets, &day_session_ids, window_days);

        let model_totals: Vec<ModelTotal> = model_map
            .into_iter()
            .map(|(model, acc)| ModelTotal {
                model: model.clone(),
                input_tokens: acc.input,
                output_tokens: acc.output,
                cache_read_tokens: acc.cache_read,
                cache_creation_tokens: acc.cache_creation,
                total_tokens: acc.input + acc.output,
            })
            .collect();

        let total_sessions: u64 = {
            let mut all_ids = std::collections::BTreeSet::new();
            for ids in day_session_ids.values() {
                all_ids.extend(ids.iter().cloned());
            }
            all_ids.len() as u64
        };
        let total_messages: u64 = day_buckets.values().map(|b| b.messages).sum();
        let days_active = day_buckets.len();

        let summary = SummaryStats {
            total_sessions,
            total_messages,
            first_session_date: earliest_date,
            days_active,
            peak_hour: None,
        };

        let payload = ClaudeStatsPayload {
            version: 2,
            last_computed_date: today_str.clone(),
            today,
            yesterday,
            daily_breakdown,
            model_totals,
            summary,
            metadata: PayloadMetadata {
                source: "localpush".to_string(),
                generated_at: self.now(),
                file_path: self.claude_projects_dir.display().to_string(),
            },
        };

        let mut value = serde_json::to_value(payload).map_err(SourceError::JsonError)?;
        if let Some(obj) = value.as_object_mut() {
            obj.insert("window_days".to_string(), serde_json::json!(window_days));
        }
        Ok(value)
    }

    fn preview(&self) -> Result<SourcePreview, SourceError> {
        let (day_buckets, day_session_ids, _, _) = self.aggregate();

        let today_str = self.today_date().format("%Y-%m-%d").to_string();
        let yesterday_str = (self.today_date() - Duration::days(1))
            .format("%Y-%m-%d")
            .to_string();

        let today_bucket = day_buckets.get(&today_str);
        let yesterday_bucket = day_buckets.get(&yesterday_str);

        let summary = if let Some(bucket) = today_bucket {
            let total_tokens: u64 = bucket.tokens_by_model.values().sum();
            let formatted = Self::format_number(total_tokens);

            let trend = if let Some(yb) = yesterday_bucket {
                let yesterday_tokens: u64 = yb.tokens_by_model.values().sum();
                if let Some(change) = Self::percentage_change(total_tokens, yesterday_tokens) {
                    if change > 0.0 {
                        format!(" (+{:.1}%)", change)
                    } else {
                        format!(" ({:.1}%)", change)
                    }
                } else {
                    String::new()
                }
            } else {
                String::new()
            };

            format!("{} tokens today{}", formatted, trend)
        } else {
            "No activity today".to_string()
        };

        let mut fields = Vec::new();

        if let Some(bucket) = today_bucket {
            let today_sessions = day_session_ids
                .get(&today_str)
                .map(|s| s.len() as u64)
                .unwrap_or(0);

            fields.push(PreviewField {
                label: "Messages".to_string(),
                value: Self::format_number(bucket.messages),
                sensitive: false,
            });

            fields.push(PreviewField {
                label: "Sessions".to_string(),
                value: Self::format_number(today_sessions),
                sensitive: false,
            });

            for (model, &count) in &bucket.tokens_by_model {
                let model_name = model.split('-').nth(1).unwrap_or(model).to_uppercase();
                fields.push(PreviewField {
                    label: format!("{} Tokens", model_name),
                    value: Self::format_number(count),
                    sensitive: false,
                });
            }
        }

        let all_session_ids: std::collections::BTreeSet<&String> =
            day_session_ids.values().flat_map(|s| s.iter()).collect();
        let window_days = self.window_days();
        fields.push(PreviewField {
            label: format!("Total Sessions ({}d)", window_days),
            value: Self::format_number(all_session_ids.len() as u64),
            sensitive: false,
        });

        fields.push(PreviewField {
            label: format!("Days Active ({}d)", window_days),
            value: day_buckets.len().to_string(),
            sensitive: false,
        });

        Ok(SourcePreview {
            title: self.name().to_string(),
            summary,
            fields,
            last_updated: Some(self.now()),
        })
    }

    fn available_properties(&self) -> Vec<PropertyDef> {
        vec![
            PropertyDef {
                key: "daily_breakdown".to_string(),
                label: "Daily Breakdown".to_string(),
                description:
                    "Daily stats with messages and tokens across the configured data window"
                        .to_string(),
                default_enabled: true,
                privacy_sensitive: false,
            },
            PropertyDef {
                key: "model_totals".to_string(),
                label: "Model Totals".to_string(),
                description: "Per-model token counts and usage across the configured data window"
                    .to_string(),
                default_enabled: true,
                privacy_sensitive: false,
            },
        ]
    }

    fn has_meaningful_payload(&self, payload: &serde_json::Value) -> bool {
        payload["summary"]["total_sessions"].as_u64().unwrap_or(0) > 0
            || payload["summary"]["total_messages"].as_u64().unwrap_or(0) > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use std::fs;
    use std::sync::Arc;
    use tempfile::TempDir;

    /// Write a minimal JSONL session with given timestamps and token counts.
    fn write_jsonl_session(
        projects_dir: &std::path::Path,
        project: &str,
        session_id: &str,
        events: &[(&str, &str, u64, u64)], // (timestamp, model, input, output)
    ) {
        let project_dir = projects_dir.join(project);
        fs::create_dir_all(&project_dir).unwrap();

        let lines: Vec<String> = events
            .iter()
            .map(|(ts, model, input, output)| {
                serde_json::json!({
                    "type": "assistant",
                    "timestamp": ts,
                    "message": {
                        "model": model,
                        "usage": {
                            "input_tokens": input,
                            "output_tokens": output,
                            "cache_read_input_tokens": 0,
                            "cache_creation_input_tokens": 0
                        }
                    }
                })
                .to_string()
            })
            .collect();

        fs::write(
            project_dir.join(format!("{session_id}.jsonl")),
            lines.join("\n"),
        )
        .unwrap();
    }

    fn ref_now() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-03-12T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
    }

    #[test]
    fn test_format_number() {
        assert_eq!(ClaudeStatsSource::format_number(1234), "1,234");
        assert_eq!(ClaudeStatsSource::format_number(1234567), "1,234,567");
        assert_eq!(ClaudeStatsSource::format_number(123), "123");
    }

    #[test]
    fn test_percentage_change() {
        assert_eq!(ClaudeStatsSource::percentage_change(150, 100), Some(50.0));
        assert_eq!(ClaudeStatsSource::percentage_change(75, 100), Some(-25.0));
        assert_eq!(ClaudeStatsSource::percentage_change(100, 0), None);
    }

    #[test]
    fn test_empty_dir_returns_valid_payload() {
        let tmp = TempDir::new().unwrap();
        let source = ClaudeStatsSource::new_with_path_and_now(tmp.path(), ref_now());

        let value = source.parse().unwrap();
        assert_eq!(value["version"], 2);
        assert_eq!(value["summary"]["total_sessions"], 0);
        assert_eq!(value["summary"]["total_messages"], 0);
        assert_eq!(value["summary"]["days_active"], 0);
        assert!(value["daily_breakdown"].as_array().unwrap().len() == 30);
    }

    #[test]
    fn test_daily_breakdown_30_entries_zero_filled() {
        let tmp = TempDir::new().unwrap();
        let source = ClaudeStatsSource::new_with_path_and_now(tmp.path(), ref_now());

        let value = source.parse().unwrap();
        let breakdown = value["daily_breakdown"].as_array().unwrap();
        assert_eq!(breakdown.len(), 30);

        // All zeros since directory is empty.
        for day in breakdown {
            assert_eq!(day["messages"], 0);
            assert_eq!(day["sessions"], 0);
            assert_eq!(day["tool_calls"], 0);
            assert_eq!(day["total_tokens"], 0);
        }

        // Oldest first, newest last.
        let first_date = breakdown[0]["date"].as_str().unwrap();
        let last_date = breakdown[29]["date"].as_str().unwrap();
        assert!(first_date < last_date);
        assert_eq!(last_date, "2026-03-12");
    }

    #[test]
    fn test_window_setting_expands_daily_breakdown() {
        let tmp = TempDir::new().unwrap();
        let config = Arc::new(AppConfig::open_in_memory().unwrap());
        config.set("source_window.claude-stats.days", "30").unwrap();

        let source = ClaudeStatsSource::new_with_path_config_and_now(tmp.path(), config, ref_now());
        let value = source.parse().unwrap();
        let breakdown = value["daily_breakdown"].as_array().unwrap();

        assert_eq!(value["window_days"], 30);
        assert_eq!(breakdown.len(), 30);
    }

    #[test]
    fn test_model_totals_aggregated() {
        let tmp = TempDir::new().unwrap();
        let now = ref_now();
        let today = now.date_naive().format("%Y-%m-%d").to_string();
        let ts = format!("{}T10:00:00Z", today);

        write_jsonl_session(
            tmp.path(),
            "-Users-test-proj",
            "s1",
            &[
                (ts.as_str(), "claude-opus-4-6", 100, 50),
                (ts.as_str(), "claude-opus-4-6", 200, 100),
            ],
        );

        let source = ClaudeStatsSource::new_with_path_and_now(tmp.path(), now);
        let value = source.parse().unwrap();

        let model_totals = value["model_totals"].as_array().unwrap();
        assert_eq!(model_totals.len(), 1);
        assert_eq!(model_totals[0]["model"], "claude-opus-4-6");
        assert_eq!(model_totals[0]["input_tokens"], 300);
        assert_eq!(model_totals[0]["output_tokens"], 150);
        assert_eq!(model_totals[0]["total_tokens"], 450);
    }

    #[test]
    fn test_today_slice_populated() {
        let tmp = TempDir::new().unwrap();
        let now = ref_now();
        let today = now.date_naive().format("%Y-%m-%d").to_string();
        let ts = format!("{}T10:00:00Z", today);

        write_jsonl_session(
            tmp.path(),
            "-Users-test",
            "sess-today",
            &[(ts.as_str(), "claude-sonnet-4-6", 500, 200)],
        );

        let source = ClaudeStatsSource::new_with_path_and_now(tmp.path(), now);
        let value = source.parse().unwrap();

        let today_val = &value["today"];
        assert!(!today_val.is_null(), "today should be Some");
        assert_eq!(today_val["date"], today);
        assert_eq!(today_val["messages"], 1);
        assert_eq!(today_val["tool_calls"], 0);
    }

    #[test]
    fn test_tool_calls_counted_from_assistant_tool_use_events() {
        let tmp = TempDir::new().unwrap();
        let now = ref_now();
        let today = now.date_naive().format("%Y-%m-%d").to_string();
        let ts = format!("{today}T10:00:00Z");
        let project_dir = tmp.path().join("-Users-test");
        fs::create_dir_all(&project_dir).unwrap();

        let payload = [
            serde_json::json!({
                "type": "assistant",
                "timestamp": ts,
                "message": {
                    "model": "claude-sonnet-4-6",
                    "content": [
                        {"type": "tool_use", "name": "Bash", "input": {"command": "pwd"}},
                        {"type": "tool_use", "name": "Read", "input": {"path": "README.md"}}
                    ],
                    "usage": {
                        "input_tokens": 100,
                        "output_tokens": 50,
                        "cache_read_input_tokens": 0,
                        "cache_creation_input_tokens": 0
                    }
                }
            })
            .to_string(),
            serde_json::json!({
                "type": "assistant",
                "timestamp": ts,
                "message": {
                    "model": "claude-sonnet-4-6",
                    "content": [
                        {"type": "text", "text": "No tools here"}
                    ],
                    "usage": {
                        "input_tokens": 50,
                        "output_tokens": 25,
                        "cache_read_input_tokens": 0,
                        "cache_creation_input_tokens": 0
                    }
                }
            })
            .to_string(),
        ]
        .join("\n");

        fs::write(project_dir.join("session-1.jsonl"), payload).unwrap();

        let source = ClaudeStatsSource::new_with_path_and_now(tmp.path(), now);
        let value = source.parse().unwrap();

        assert_eq!(value["today"]["tool_calls"], 2);
        assert_eq!(value["daily_breakdown"][29]["tool_calls"], 2);
    }

    #[test]
    fn test_summary_counts_sessions_across_days() {
        let tmp = TempDir::new().unwrap();
        let now = ref_now();
        let today = now.date_naive().format("%Y-%m-%d").to_string();
        let yesterday = (now.date_naive() - Duration::days(1))
            .format("%Y-%m-%d")
            .to_string();

        let ts_today = format!("{}T10:00:00Z", today);
        let ts_yest = format!("{}T10:00:00Z", yesterday);

        write_jsonl_session(
            tmp.path(),
            "-Users-test",
            "s1",
            &[(ts_today.as_str(), "claude-opus-4-6", 100, 50)],
        );
        write_jsonl_session(
            tmp.path(),
            "-Users-test",
            "s2",
            &[(ts_yest.as_str(), "claude-opus-4-6", 100, 50)],
        );

        let source = ClaudeStatsSource::new_with_path_and_now(tmp.path(), now);
        let value = source.parse().unwrap();

        assert_eq!(value["summary"]["total_sessions"], 2);
        assert_eq!(value["summary"]["total_messages"], 2);
        assert_eq!(value["summary"]["days_active"], 2);
        assert!(value["summary"]["peak_hour"].is_null());
    }

    #[test]
    fn test_old_messages_in_recent_file_excluded() {
        // A long-lived session file touched today may contain messages from
        // months ago. Only messages within the 30-day window should be counted.
        let tmp = TempDir::new().unwrap();
        let now = ref_now(); // 2026-03-12T12:00:00Z
        let today = now.date_naive().format("%Y-%m-%d").to_string();
        let ts_recent = format!("{}T10:00:00Z", today);
        // 60 days ago — well outside the 30-day window
        let old_date = (now - Duration::days(60))
            .date_naive()
            .format("%Y-%m-%d")
            .to_string();
        let ts_old = format!("{}T10:00:00Z", old_date);

        write_jsonl_session(
            tmp.path(),
            "-Users-test",
            "long-lived-session",
            &[
                (ts_old.as_str(), "claude-opus-4-6", 1000, 500),
                (ts_recent.as_str(), "claude-opus-4-6", 200, 100),
            ],
        );

        let source = ClaudeStatsSource::new_with_path_and_now(tmp.path(), now);
        let value = source.parse().unwrap();

        // Only the recent message should be counted.
        assert_eq!(value["summary"]["total_messages"], 1);
        // Model totals should reflect only the recent message's tokens.
        let model_totals = value["model_totals"].as_array().unwrap();
        assert_eq!(model_totals.len(), 1);
        assert_eq!(model_totals[0]["input_tokens"], 200);
        assert_eq!(model_totals[0]["output_tokens"], 100);
    }

    #[test]
    fn test_source_trait_basics() {
        let tmp = TempDir::new().unwrap();
        let source = ClaudeStatsSource::new_with_path(tmp.path());

        assert_eq!(source.id(), "claude-stats");
        assert_eq!(source.name(), "Claude Code Statistics");
        assert!(source.watch_path().is_some());
        assert!(source.watch_recursive());
    }

    #[test]
    fn test_available_properties_no_cost_or_hour_breakdown() {
        let tmp = TempDir::new().unwrap();
        let source = ClaudeStatsSource::new_with_path(tmp.path());
        let props = source.available_properties();
        let keys: Vec<&str> = props.iter().map(|p| p.key.as_str()).collect();

        assert!(keys.contains(&"daily_breakdown"));
        assert!(keys.contains(&"model_totals"));
        // These were removed (not available from JSONL).
        assert!(!keys.contains(&"cost_breakdown"));
        assert!(!keys.contains(&"hour_breakdown"));
        assert!(!keys.contains(&"longest_session"));
    }

    #[test]
    fn test_preview_no_activity() {
        let tmp = TempDir::new().unwrap();
        let source = ClaudeStatsSource::new_with_path_and_now(tmp.path(), ref_now());
        let preview = source.preview().unwrap();
        assert_eq!(preview.summary, "No activity today");
        assert_eq!(preview.title, "Claude Code Statistics");
    }

    #[test]
    fn test_preview_with_today_activity() {
        let tmp = TempDir::new().unwrap();
        let now = ref_now();
        let today = now.date_naive().format("%Y-%m-%d").to_string();
        let ts = format!("{}T10:00:00Z", today);

        write_jsonl_session(
            tmp.path(),
            "-Users-test",
            "s1",
            &[(ts.as_str(), "claude-opus-4-6", 1000, 500)],
        );

        let source = ClaudeStatsSource::new_with_path_and_now(tmp.path(), now);
        let preview = source.preview().unwrap();

        assert!(
            preview.summary.contains("tokens today"),
            "Summary was: {}",
            preview.summary
        );
    }
}
