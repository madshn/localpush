use super::{recursive_path_change_hint, PreviewField, Source, SourceError, SourcePreview};
use crate::config::AppConfig;
use crate::source_config::{window_setting_for_source, PropertyDef, SourceConfigStore};
use chrono::{DateTime, Duration, NaiveDate, Utc};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::Arc;

use super::codex_sessions::{collect_codex_sessions, CodexTokenUsage};

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

#[derive(Clone)]
pub struct CodexStatsSource {
    sessions_root: PathBuf,
    config: Option<Arc<AppConfig>>,
    reference_now: Option<DateTime<Utc>>,
}

impl CodexStatsSource {
    pub fn new(config: Arc<AppConfig>) -> Result<Self, SourceError> {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .map_err(|_| SourceError::ParseError("Could not determine home directory".into()))?;
        Ok(Self {
            sessions_root: PathBuf::from(home).join(".codex").join("sessions"),
            config: Some(config),
            reference_now: None,
        })
    }

    pub fn new_with_path(path: impl Into<PathBuf>) -> Self {
        Self {
            sessions_root: path.into(),
            config: None,
            reference_now: None,
        }
    }

    #[cfg(test)]
    pub fn new_with_path_and_now(path: impl Into<PathBuf>, reference_now: DateTime<Utc>) -> Self {
        Self {
            sessions_root: path.into(),
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
            sessions_root: path.into(),
            config: Some(config),
            reference_now: Some(reference_now),
        }
    }

    fn now(&self) -> DateTime<Utc> {
        self.reference_now.unwrap_or_else(Utc::now)
    }

    fn window_days(&self) -> i64 {
        let Some(config) = &self.config else {
            return 1;
        };
        let def = window_setting_for_source(self.id())
            .expect("codex-stats should have a window setting definition");
        SourceConfigStore::new(config.clone()).get_window_days(self.id(), &def)
    }
}

impl Source for CodexStatsSource {
    fn id(&self) -> &str {
        "codex-stats"
    }

    fn name(&self) -> &str {
        "Codex Statistics"
    }

    fn watch_path(&self) -> Option<PathBuf> {
        Some(self.sessions_root.clone())
    }

    fn watch_recursive(&self) -> bool {
        true
    }

    fn delivery_change_hint(&self) -> Result<Option<String>, SourceError> {
        Ok(
            recursive_path_change_hint(&self.sessions_root, None)?.map(|hint| {
                format!(
                    "day:{}:window:{}:{}",
                    self.now().date_naive().format("%Y-%m-%d"),
                    self.window_days(),
                    hint
                )
            }),
        )
    }

    fn parse(&self) -> Result<Value, SourceError> {
        let sessions = collect_codex_sessions(&self.sessions_root, None);
        let window_days = self.window_days();
        let today = self.now().date_naive();
        let window_start = today - Duration::days(window_days);
        let window_end = today;

        let mut day_totals: BTreeMap<String, CodexTokenUsage> = BTreeMap::new();
        let mut day_session_counts: BTreeMap<String, u64> = BTreeMap::new();
        let mut models_observed_for_day: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        let mut sessions_in_window: BTreeSet<String> = BTreeSet::new();

        for session in &sessions {
            let mut prev_total = CodexTokenUsage::default();
            let mut seen_days_for_session: BTreeSet<String> = BTreeSet::new();
            for snap in &session.token_snapshots {
                let delta = snap.total_usage.saturating_delta(&prev_total);
                prev_total = snap.total_usage.clone();

                let day = snap.timestamp.date_naive().format("%Y-%m-%d").to_string();
                day_totals
                    .entry(day.clone())
                    .or_default()
                    .add_assign(&delta);
                seen_days_for_session.insert(day.clone());
                let snap_day = snap.timestamp.date_naive();
                if snap_day >= window_start && snap_day < window_end {
                    sessions_in_window.insert(session.id.clone());
                }

                if let Some(model) = &session.model {
                    models_observed_for_day
                        .entry(day)
                        .or_default()
                        .insert(model.clone());
                }
            }
            for day in seen_days_for_session {
                *day_session_counts.entry(day).or_insert(0) += 1;
            }
        }

        let latest_day: NaiveDate = today - Duration::days(1);
        let latest_day_key = latest_day.format("%Y-%m-%d").to_string();
        let latest_day_sessions = day_session_counts
            .get(&latest_day_key)
            .copied()
            .unwrap_or(0);

        // We only emit leaf metrics. Per-model versioned leaves are gated by provable attribution.
        // Current Codex token_count snapshots are cumulative and not reliably tagged by model per token delta,
        // so emit the safe unversioned Codex family leaf.
        let metric_key = "token.openai.codex";
        let mut metrics = Vec::with_capacity(window_days as usize);
        let mut all_models_observed: BTreeSet<String> = BTreeSet::new();

        for offset in 0..window_days {
            let day: NaiveDate = window_start + Duration::days(offset);
            let day_key = day.format("%Y-%m-%d").to_string();
            let totals = day_totals.get(&day_key).cloned().unwrap_or_default();

            if let Some(models) = models_observed_for_day.get(&day_key) {
                all_models_observed.extend(models.iter().cloned());
            }

            metrics.push(serde_json::json!({
                "metric_key": metric_key,
                "period_from": format!("{day_key}T00:00:00Z"),
                "period_to": format!(
                    "{}T00:00:00Z",
                    (day + Duration::days(1)).format("%Y-%m-%d")
                ),
                "value": totals.total,
                "source": "localpush",
                "cost_model": "subscription",
                "tags": {
                    "input": totals.input,
                    "cached_input": totals.cached_input,
                    "output": totals.output,
                    "reasoning_output": totals.reasoning_output
                }
            }));
        }

        Ok(serde_json::json!({
            "metrics": metrics,
            "meta": {
                "source_family": "codex",
                "source_type": "stats",
                "schema_version": 2,
                "day_boundary": "utc",
                "selected_window": {
                    "mode": "recent_days",
                    "days": window_days,
                    "start_date": window_start.format("%Y-%m-%d").to_string(),
                    "end_date_exclusive": window_end.format("%Y-%m-%d").to_string()
                },
                "latest_date": latest_day_key,
                "sessions_in_window": sessions_in_window.len(),
                "latest_day_sessions": latest_day_sessions,
                "attribution_mode": "safe_unversioned_family_only",
                "models_observed": all_models_observed.into_iter().collect::<Vec<_>>(),
                "per_model_versioned_metrics_emitted": false,
                "notes": [
                    "Watch session JSONL files (or a derived local cache); period windows are derived from event timestamps, not filesystem paths",
                    "Leaf metrics only; aggregate metrics are computed downstream",
                    "Versioned model leaves are withheld until per-model token attribution is provably correct",
                    "Metrics are emitted for every day in the configured window, including zero-activity days"
                ]
            }
        }))
    }

    fn preview(&self) -> Result<SourcePreview, SourceError> {
        let payload = self.parse()?;
        let metrics = payload["metrics"].as_array().cloned().unwrap_or_default();
        let total: u64 = metrics
            .iter()
            .map(|metric| metric["value"].as_u64().unwrap_or(0))
            .sum();
        let window_days = self.window_days();
        let latest_date = payload["meta"]["latest_date"].as_str().unwrap_or("");

        Ok(SourcePreview {
            title: self.name().to_string(),
            summary: if window_days == 1 {
                format!("{} tokens for {}", format_number(total), latest_date)
            } else {
                format!(
                    "{} tokens across last {} days",
                    format_number(total),
                    window_days
                )
            },
            fields: vec![
                PreviewField {
                    label: "Metric Key".into(),
                    value: "token.openai.codex".to_string(),
                    sensitive: false,
                },
                PreviewField {
                    label: "Tokens".into(),
                    value: format_number(total),
                    sensitive: false,
                },
                PreviewField {
                    label: "Window".into(),
                    value: format!("{} days", window_days),
                    sensitive: false,
                },
            ],
            last_updated: Some(Utc::now()),
        })
    }

    fn available_properties(&self) -> Vec<PropertyDef> {
        vec![PropertyDef {
            key: "metrics".into(),
            label: "Metrics".into(),
            description: "Leaf KPI metrics for each day in the configured UTC data window".into(),
            default_enabled: true,
            privacy_sensitive: false,
        }]
    }

    fn has_meaningful_payload(&self, payload: &serde_json::Value) -> bool {
        payload["meta"]["sessions_in_window"].as_u64().unwrap_or(0) > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use crate::sources::codex_sessions::{normalize_model_key, CodexSessionsSource};
    use std::fs;
    use std::path::PathBuf;
    use std::sync::Arc;

    fn fixture_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/codex/2026-02-23/raw/sessions")
    }

    fn fixture_base() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/codex/2026-02-23")
    }

    fn fixture_source() -> CodexStatsSource {
        let now = DateTime::parse_from_rfc3339("2026-02-24T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        CodexStatsSource::new_with_path_and_now(fixture_dir(), now)
    }

    fn normalize(mut v: Value) -> Value {
        if let Some(obj) = v.as_object_mut() {
            obj.remove("timestamp");
            if let Some(meta) = obj.get_mut("meta").and_then(|m| m.as_object_mut()) {
                // No runtime timestamp today, but future-proof normalize hook.
                meta.remove("generated_at");
            }
        }
        v
    }

    #[test]
    fn test_codex_stats_fixture_parses() {
        let payload = fixture_source().parse().unwrap();
        assert!(payload["metrics"].is_array());
        assert_eq!(payload["metrics"].as_array().unwrap().len(), 1);
        assert_eq!(payload["metrics"][0]["metric_key"], "token.openai.codex");
        assert_eq!(payload["metrics"][0]["source"], "localpush");
        assert_eq!(payload["metrics"][0]["cost_model"], "subscription");
        assert_eq!(payload["meta"]["latest_date"], "2026-02-23");
    }

    #[test]
    fn test_codex_stats_fixture_matches_expected_golden() {
        let actual = normalize(fixture_source().parse().unwrap());
        let expected_path = fixture_base().join("expected/codex-stats.json");
        let expected: Value =
            serde_json::from_str(&fs::read_to_string(expected_path).unwrap()).unwrap();
        if expected.get("_status").and_then(|v| v.as_str()) == Some("pending") {
            return;
        }
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_codex_fixture_manifest_verification_matches_outputs() {
        let base = fixture_base();
        let manifest: Value =
            serde_json::from_str(&fs::read_to_string(base.join("manifest.json")).unwrap()).unwrap();
        let sessions = normalize(
            CodexSessionsSource::new_with_path(fixture_dir())
                .parse()
                .unwrap(),
        );
        let stats = normalize(fixture_source().parse().unwrap());

        assert_eq!(
            manifest["verification"]["sessions_in_scope"]
                .as_u64()
                .unwrap(),
            sessions["summary"]["sessions_count"].as_u64().unwrap()
        );

        let metric = &stats["metrics"][0];
        let tags = &metric["tags"];

        assert_eq!(
            manifest["verification"]["token_totals"]["input"],
            tags["input"]
        );
        assert_eq!(
            manifest["verification"]["token_totals"]["output"],
            tags["output"]
        );
        assert_eq!(
            manifest["verification"]["token_totals"]["total"],
            metric["value"]
        );
        assert_eq!(
            manifest["verification"]["token_totals"]["cache_read"],
            tags["cached_input"]
        );
    }

    #[test]
    fn test_window_setting_emits_full_daily_series() {
        let now = DateTime::parse_from_rfc3339("2026-02-24T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let config = Arc::new(AppConfig::open_in_memory().unwrap());
        config.set("source_window.codex-stats.days", "7").unwrap();
        let source = CodexStatsSource::new_with_path_config_and_now(fixture_dir(), config, now);

        let payload = source.parse().unwrap();
        let metrics = payload["metrics"].as_array().unwrap();

        assert_eq!(metrics.len(), 7);
        assert_eq!(payload["meta"]["selected_window"]["days"], 7);
        assert_eq!(
            metrics.first().unwrap()["period_from"],
            "2026-02-17T00:00:00Z"
        );
        assert_eq!(
            metrics.last().unwrap()["period_from"],
            "2026-02-23T00:00:00Z"
        );
    }

    #[test]
    #[ignore]
    fn regenerate_codex_fixture_goldens_2026_02_23() {
        let base = fixture_base();
        let sessions = normalize(
            CodexSessionsSource::new_with_path(fixture_dir())
                .parse()
                .unwrap(),
        );
        let stats = normalize(fixture_source().parse().unwrap());

        fs::write(
            base.join("expected/codex-sessions.json"),
            serde_json::to_string_pretty(&sessions).unwrap() + "\n",
        )
        .unwrap();
        fs::write(
            base.join("expected/codex-stats.json"),
            serde_json::to_string_pretty(&stats).unwrap() + "\n",
        )
        .unwrap();
    }

    #[test]
    fn test_model_normalization_algorithm_cases() {
        let cases = [
            ("gpt-5.3-codex", "openai.codex.5_3"),
            ("gpt-4o", "openai.gpt.4o"),
            ("gpt-4o-mini", "openai.gpt.4o_mini"),
            ("gpt-4.1", "openai.gpt.4_1"),
            ("gpt-4.1-mini", "openai.gpt.4_1_mini"),
            ("o1", "openai.o.1"),
            ("o3-mini", "openai.o.3_mini"),
            ("o1-pro", "openai.o.1_pro"),
            ("claude-opus-4-6", "anthropic.opus.4_6"),
            ("claude-sonnet-4-5-20250929", "anthropic.sonnet.4_5"),
            ("gemini-2.0-flash", "google.gemini.2_0_flash"),
        ];
        for (input, expected) in cases {
            assert_eq!(normalize_model_key(input), expected, "{input}");
        }
    }
}
