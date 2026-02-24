use super::{PreviewField, Source, SourceError, SourcePreview};
use crate::source_config::PropertyDef;
use chrono::{DateTime, Duration, NaiveDate, Utc};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

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
    reference_now: Option<DateTime<Utc>>,
}

impl CodexStatsSource {
    pub fn new() -> Result<Self, SourceError> {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .map_err(|_| SourceError::ParseError("Could not determine home directory".into()))?;
        Ok(Self {
            sessions_root: PathBuf::from(home).join(".codex").join("sessions"),
            reference_now: None,
        })
    }

    pub fn new_with_path(path: impl Into<PathBuf>) -> Self {
        Self {
            sessions_root: path.into(),
            reference_now: None,
        }
    }

    #[cfg(test)]
    pub fn new_with_path_and_now(path: impl Into<PathBuf>, reference_now: DateTime<Utc>) -> Self {
        Self {
            sessions_root: path.into(),
            reference_now: Some(reference_now),
        }
    }

    fn now(&self) -> DateTime<Utc> {
        self.reference_now.unwrap_or_else(Utc::now)
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

    fn parse(&self) -> Result<Value, SourceError> {
        let sessions = collect_codex_sessions(&self.sessions_root, None);

        let mut day_totals: BTreeMap<String, CodexTokenUsage> = BTreeMap::new();
        let mut day_session_counts: BTreeMap<String, u64> = BTreeMap::new();
        let mut models_observed_for_day: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

        for session in &sessions {
            let mut prev_total = CodexTokenUsage::default();
            let mut seen_days_for_session: BTreeSet<String> = BTreeSet::new();
            for snap in &session.token_snapshots {
                let delta = snap.total_usage.saturating_delta(&prev_total);
                prev_total = snap.total_usage.clone();

                let day = snap.timestamp.date_naive().format("%Y-%m-%d").to_string();
                day_totals.entry(day.clone()).or_default().add_assign(&delta);
                seen_days_for_session.insert(day.clone());

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

        let target_day: NaiveDate = self.now().date_naive() - Duration::days(1);
        let target_day_key = target_day.format("%Y-%m-%d").to_string();
        let totals = day_totals.get(&target_day_key).cloned().unwrap_or_default();
        let sessions_count = day_session_counts.get(&target_day_key).copied().unwrap_or(0);
        let models_observed: Vec<String> = models_observed_for_day
            .get(&target_day_key)
            .map(|s| s.iter().cloned().collect())
            .unwrap_or_default();

        let period_from = format!("{target_day_key}T00:00:00Z");
        let period_to = format!(
            "{}T00:00:00Z",
            (target_day + Duration::days(1)).format("%Y-%m-%d")
        );

        // We only emit leaf metrics. Per-model versioned leaves are gated by provable attribution.
        // Current Codex token_count snapshots are cumulative and not reliably tagged by model per token delta,
        // so emit the safe unversioned Codex family leaf.
        let metric_key = "token.openai.codex";

        let metrics = vec![serde_json::json!({
            "metric_key": metric_key,
            "period_from": period_from,
            "period_to": period_to,
            "value": totals.total,
            "source": "localpush",
            "cost_model": "subscription",
            "tags": {
                "input": totals.input,
                "cached_input": totals.cached_input,
                "output": totals.output,
                "reasoning_output": totals.reasoning_output
            }
        })];

        Ok(serde_json::json!({
            "metrics": metrics,
            "meta": {
                "source_family": "codex",
                "source_type": "stats",
                "schema_version": 2,
                "day_boundary": "utc",
                "selected_window": "yesterday",
                "target_date": target_day_key,
                "sessions_in_window": sessions_count,
                "attribution_mode": "safe_unversioned_family_only",
                "models_observed": models_observed,
                "per_model_versioned_metrics_emitted": false,
                "notes": [
                    "Watch session JSONL files (or a derived local cache); period windows are derived from event timestamps, not filesystem paths",
                    "Leaf metrics only; aggregate metrics are computed downstream",
                    "Versioned model leaves are withheld until per-model token attribution is provably correct"
                ]
            }
        }))
    }

    fn preview(&self) -> Result<SourcePreview, SourceError> {
        let payload = self.parse()?;
        let metric = &payload["metrics"][0];
        let total = metric["value"].as_u64().unwrap_or(0);
        let date = metric["period_from"]
            .as_str()
            .unwrap_or("")
            .split('T')
            .next()
            .unwrap_or("");

        Ok(SourcePreview {
            title: self.name().to_string(),
            summary: format!("{} tokens for {}", format_number(total), date),
            fields: vec![
                PreviewField {
                    label: "Metric Key".into(),
                    value: metric["metric_key"].as_str().unwrap_or("").to_string(),
                    sensitive: false,
                },
                PreviewField {
                    label: "Tokens".into(),
                    value: format_number(total),
                    sensitive: false,
                },
            ],
            last_updated: Some(Utc::now()),
        })
    }

    fn available_properties(&self) -> Vec<PropertyDef> {
        vec![
            PropertyDef {
                key: "metrics".into(),
                label: "Metrics".into(),
                description: "Leaf KPI metrics for yesterday UTC window".into(),
                default_enabled: true,
                privacy_sensitive: false,
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sources::codex_sessions::{CodexSessionsSource, normalize_model_key};
    use std::fs;
    use std::path::PathBuf;

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
        assert_eq!(payload["meta"]["target_date"], "2026-02-23");
    }

    #[test]
    fn test_codex_stats_fixture_matches_expected_golden() {
        let actual = normalize(fixture_source().parse().unwrap());
        let expected_path = fixture_base().join("expected/codex-stats.json");
        let expected: Value = serde_json::from_str(&fs::read_to_string(expected_path).unwrap()).unwrap();
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
        let sessions = normalize(CodexSessionsSource::new_with_path(fixture_dir()).parse().unwrap());
        let stats = normalize(fixture_source().parse().unwrap());

        assert_eq!(
            manifest["verification"]["sessions_in_scope"].as_u64().unwrap(),
            sessions["summary"]["sessions_count"].as_u64().unwrap()
        );

        let metric = &stats["metrics"][0];
        let tags = &metric["tags"];

        assert_eq!(manifest["verification"]["token_totals"]["input"], tags["input"]);
        assert_eq!(manifest["verification"]["token_totals"]["output"], tags["output"]);
        assert_eq!(manifest["verification"]["token_totals"]["total"], metric["value"]);
        assert_eq!(
            manifest["verification"]["token_totals"]["cache_read"],
            tags["cached_input"]
        );
    }

    #[test]
    #[ignore]
    fn regenerate_codex_fixture_goldens_2026_02_23() {
        let base = fixture_base();
        let sessions = normalize(CodexSessionsSource::new_with_path(fixture_dir()).parse().unwrap());
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
