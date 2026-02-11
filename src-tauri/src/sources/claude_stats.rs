use super::{PreviewField, Source, SourceError, SourcePreview};
use crate::source_config::PropertyDef;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tracing::{debug, info, warn};

/// Raw structure of Claude Code stats-cache.json
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
#[serde(rename_all = "camelCase")]
struct ClaudeStatsRaw {
    version: u32,
    last_computed_date: String,
    daily_activity: Vec<DailyActivity>,
    daily_model_tokens: Vec<DailyModelTokens>,
    model_usage: HashMap<String, ModelUsage>,
    total_sessions: u64,
    total_messages: u64,
    longest_session: Option<LongestSession>,
    first_session_date: Option<String>,
    hour_counts: HashMap<String, u64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DailyActivity {
    date: String,
    message_count: u64,
    session_count: u64,
    tool_call_count: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DailyModelTokens {
    date: String,
    tokens_by_model: HashMap<String, u64>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
#[serde(rename_all = "camelCase")]
struct ModelUsage {
    input_tokens: u64,
    output_tokens: u64,
    cache_read_input_tokens: u64,
    cache_creation_input_tokens: u64,
    #[serde(default)]
    web_search_requests: u64,
    #[serde(alias = "costUSD", default)]
    cost_usd: f64,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
#[serde(rename_all = "camelCase")]
struct LongestSession {
    session_id: String,
    duration: u64,
    message_count: u64,
    timestamp: String,
}

/// Structured payload sent to webhooks
#[derive(Debug, Serialize, Deserialize)]
pub struct ClaudeStatsPayload {
    pub version: u32,
    pub last_computed_date: String,
    pub today: Option<DailyStats>,
    pub yesterday: Option<DailyStats>,
    /// 14-day rolling breakdown with zero-filled gaps (oldest → newest)
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
    pub peak_hour: Option<u8>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PayloadMetadata {
    pub source: String,
    pub generated_at: DateTime<Utc>,
    pub file_path: String,
}

/// Claude Code statistics source
pub struct ClaudeStatsSource {
    stats_path: PathBuf,
}

impl ClaudeStatsSource {
    pub fn new() -> Result<Self, SourceError> {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .map_err(|_| {
                SourceError::ParseError("Could not determine home directory".to_string())
            })?;

        let stats_path = PathBuf::from(home).join(".claude").join("stats-cache.json");

        Ok(Self { stats_path })
    }

    /// Constructor with custom path (for testing)
    pub fn new_with_path(path: impl Into<PathBuf>) -> Self {
        Self {
            stats_path: path.into(),
        }
    }

    /// Helper to parse the raw stats file
    fn load_stats(&self) -> Result<ClaudeStatsRaw, SourceError> {
        debug!("Loading Claude stats from: {}", self.stats_path.display());

        if !self.stats_path.exists() {
            warn!("Stats file not found at: {}", self.stats_path.display());
            return Err(SourceError::FileNotFound(self.stats_path.clone()));
        }

        let content = fs::read_to_string(&self.stats_path)?;
        let stats: ClaudeStatsRaw = serde_json::from_str(&content)?;

        info!(
            "Loaded Claude stats: {} sessions, {} messages",
            stats.total_sessions, stats.total_messages
        );

        Ok(stats)
    }

    /// Get today's date string
    fn today() -> String {
        chrono::Local::now().format("%Y-%m-%d").to_string()
    }

    /// Get yesterday's date string
    fn yesterday() -> String {
        let yesterday = chrono::Local::now() - chrono::Duration::days(1);
        yesterday.format("%Y-%m-%d").to_string()
    }

    /// Find daily activity for a specific date
    fn find_daily_activity(
        stats: &ClaudeStatsRaw,
        date: &str,
    ) -> Option<(DailyActivity, HashMap<String, u64>)> {
        let activity = stats
            .daily_activity
            .iter()
            .find(|a| a.date == date)
            .cloned();

        let tokens = stats
            .daily_model_tokens
            .iter()
            .find(|t| t.date == date)
            .map(|t| t.tokens_by_model.clone())
            .unwrap_or_default();

        activity.map(|a| (a, tokens))
    }

    /// Calculate total tokens for a day
    fn total_tokens(tokens_by_model: &HashMap<String, u64>) -> u64 {
        tokens_by_model.values().sum()
    }

    /// Find peak hour from hour_counts
    fn find_peak_hour(hour_counts: &HashMap<String, u64>) -> Option<u8> {
        hour_counts
            .iter()
            .max_by_key(|(_, &count)| count)
            .and_then(|(hour, _)| hour.parse().ok())
    }

    /// Format number with commas
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

    /// Calculate percentage change
    fn percentage_change(today: u64, yesterday: u64) -> Option<f64> {
        if yesterday == 0 {
            return None;
        }
        let change = ((today as f64 - yesterday as f64) / yesterday as f64) * 100.0;
        Some(change)
    }

    /// Build a rolling daily breakdown with zero-filled gaps.
    /// Returns `window_days` entries ordered oldest → newest.
    fn build_daily_breakdown(stats: &ClaudeStatsRaw, window_days: usize) -> Vec<DailyStats> {
        let today = chrono::Local::now().date_naive();
        let mut breakdown = Vec::with_capacity(window_days);

        for i in (0..window_days).rev() {
            let date = today - chrono::Duration::days(i as i64);
            let date_str = date.format("%Y-%m-%d").to_string();

            let daily = match Self::find_daily_activity(stats, &date_str) {
                Some((activity, tokens)) => DailyStats {
                    date: date_str,
                    messages: activity.message_count,
                    sessions: activity.session_count,
                    tool_calls: activity.tool_call_count,
                    total_tokens: Self::total_tokens(&tokens),
                    tokens_by_model: tokens,
                },
                None => DailyStats {
                    date: date_str,
                    messages: 0,
                    sessions: 0,
                    tool_calls: 0,
                    total_tokens: 0,
                    tokens_by_model: HashMap::new(),
                },
            };

            breakdown.push(daily);
        }

        breakdown
    }
}

impl Default for ClaudeStatsSource {
    fn default() -> Self {
        Self::new().expect("Failed to initialize ClaudeStatsSource")
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
        Some(self.stats_path.clone())
    }

    fn parse(&self) -> Result<serde_json::Value, SourceError> {
        let stats = self.load_stats()?;

        let today_date = Self::today();
        let yesterday_date = Self::yesterday();

        // Build today's stats
        let today = Self::find_daily_activity(&stats, &today_date).map(|(activity, tokens)| {
            DailyStats {
                date: activity.date,
                messages: activity.message_count,
                sessions: activity.session_count,
                tool_calls: activity.tool_call_count,
                total_tokens: Self::total_tokens(&tokens),
                tokens_by_model: tokens,
            }
        });

        // Build yesterday's stats
        let yesterday =
            Self::find_daily_activity(&stats, &yesterday_date).map(|(activity, tokens)| {
                DailyStats {
                    date: activity.date,
                    messages: activity.message_count,
                    sessions: activity.session_count,
                    tool_calls: activity.tool_call_count,
                    total_tokens: Self::total_tokens(&tokens),
                    tokens_by_model: tokens,
                }
            });

        // Build 14-day rolling breakdown with zero-filled gaps (before model_usage is consumed)
        let daily_breakdown = Self::build_daily_breakdown(&stats, 14);

        // Build model totals
        let model_totals: Vec<ModelTotal> = stats
            .model_usage
            .into_iter()
            .map(|(model, usage)| ModelTotal {
                model: model.clone(),
                input_tokens: usage.input_tokens,
                output_tokens: usage.output_tokens,
                cache_read_tokens: usage.cache_read_input_tokens,
                cache_creation_tokens: usage.cache_creation_input_tokens,
                total_tokens: usage.input_tokens + usage.output_tokens,
            })
            .collect();

        // Build summary
        let summary = SummaryStats {
            total_sessions: stats.total_sessions,
            total_messages: stats.total_messages,
            first_session_date: stats.first_session_date,
            days_active: stats.daily_activity.len(),
            peak_hour: Self::find_peak_hour(&stats.hour_counts),
        };

        // Build payload
        let payload = ClaudeStatsPayload {
            version: stats.version,
            last_computed_date: stats.last_computed_date,
            today,
            yesterday,
            daily_breakdown,
            model_totals,
            summary,
            metadata: PayloadMetadata {
                source: "localpush".to_string(),
                generated_at: Utc::now(),
                file_path: self.stats_path.display().to_string(),
            },
        };

        serde_json::to_value(payload).map_err(SourceError::JsonError)
    }

    fn preview(&self) -> Result<SourcePreview, SourceError> {
        let stats = self.load_stats()?;

        let today_date = Self::today();
        let yesterday_date = Self::yesterday();

        let today_data = Self::find_daily_activity(&stats, &today_date);
        let yesterday_data = Self::find_daily_activity(&stats, &yesterday_date);

        // Build summary line
        let summary = if let Some((_today_activity, today_tokens)) = &today_data {
            let total_tokens = Self::total_tokens(today_tokens);
            let formatted_tokens = Self::format_number(total_tokens);

            // Calculate trend if we have yesterday's data
            let trend = if let Some((_yesterday_activity, yesterday_tokens)) = &yesterday_data {
                let yesterday_total = Self::total_tokens(yesterday_tokens);
                if let Some(change) = Self::percentage_change(total_tokens, yesterday_total) {
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

            format!("{} tokens today{}", formatted_tokens, trend)
        } else {
            "No activity today".to_string()
        };

        // Build preview fields
        let mut fields = Vec::new();

        if let Some((activity, tokens)) = today_data {
            fields.push(PreviewField {
                label: "Messages".to_string(),
                value: Self::format_number(activity.message_count),
                sensitive: false,
            });

            fields.push(PreviewField {
                label: "Sessions".to_string(),
                value: Self::format_number(activity.session_count),
                sensitive: false,
            });

            fields.push(PreviewField {
                label: "Tool Calls".to_string(),
                value: Self::format_number(activity.tool_call_count),
                sensitive: false,
            });

            // Show tokens by model
            for (model, count) in tokens {
                let model_name = model
                    .split('-')
                    .nth(1)
                    .unwrap_or(&model)
                    .to_uppercase();
                fields.push(PreviewField {
                    label: format!("{} Tokens", model_name),
                    value: Self::format_number(count),
                    sensitive: false,
                });
            }
        }

        // Add total statistics
        fields.push(PreviewField {
            label: "Total Sessions".to_string(),
            value: Self::format_number(stats.total_sessions),
            sensitive: false,
        });

        fields.push(PreviewField {
            label: "Days Active".to_string(),
            value: stats.daily_activity.len().to_string(),
            sensitive: false,
        });

        // Last update time (from file modification)
        let last_updated = fs::metadata(&self.stats_path)
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
                key: "daily_breakdown".to_string(),
                label: "Daily Breakdown".to_string(),
                description: "14-day rolling stats with messages and tokens per day".to_string(),
                default_enabled: true,
                privacy_sensitive: false,
            },
            PropertyDef {
                key: "model_totals".to_string(),
                label: "Model Totals".to_string(),
                description: "Per-model token counts and usage".to_string(),
                default_enabled: true,
                privacy_sensitive: false,
            },
            PropertyDef {
                key: "cost_breakdown".to_string(),
                label: "Cost Breakdown".to_string(),
                description: "Estimated costs per model (approximate)".to_string(),
                default_enabled: false,
                privacy_sensitive: false,
            },
            PropertyDef {
                key: "hour_breakdown".to_string(),
                label: "Hour Breakdown".to_string(),
                description: "Hourly activity distribution across the day".to_string(),
                default_enabled: false,
                privacy_sensitive: false,
            },
            PropertyDef {
                key: "longest_session".to_string(),
                label: "Longest Session".to_string(),
                description: "Peak session details and stats".to_string(),
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
    fn test_format_number() {
        assert_eq!(ClaudeStatsSource::format_number(1234), "1,234");
        assert_eq!(ClaudeStatsSource::format_number(1234567), "1,234,567");
        assert_eq!(ClaudeStatsSource::format_number(123), "123");
    }

    #[test]
    fn test_percentage_change() {
        assert_eq!(
            ClaudeStatsSource::percentage_change(150, 100),
            Some(50.0)
        );
        assert_eq!(
            ClaudeStatsSource::percentage_change(75, 100),
            Some(-25.0)
        );
        assert_eq!(ClaudeStatsSource::percentage_change(100, 0), None);
    }

    #[test]
    fn test_total_tokens() {
        let mut tokens = HashMap::new();
        tokens.insert("opus".to_string(), 1000);
        tokens.insert("sonnet".to_string(), 500);
        assert_eq!(ClaudeStatsSource::total_tokens(&tokens), 1500);
    }
}
