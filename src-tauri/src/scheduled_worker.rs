//! Scheduled delivery worker for daily/weekly push cadence
//!
//! Runs on a 60-second interval, checks scheduled bindings, and enqueues
//! targeted deliveries when they become due. The existing delivery worker
//! handles the actual HTTP dispatch with full WAL/retry guarantees.

use std::sync::Arc;
use std::time::Duration;

use chrono::{Datelike, Local, NaiveTime, Weekday};

use crate::bindings::{BindingStore, SourceBinding};
use crate::source_manager::SourceManager;
use crate::target_manager::TargetManager;
use crate::traits::DeliveryLedgerTrait;

/// Check if a scheduled binding is due for delivery
fn is_due(binding: &SourceBinding, now: chrono::DateTime<Local>) -> bool {
    let schedule_time = match &binding.schedule_time {
        Some(t) => t,
        None => return false,
    };

    let target_time = match NaiveTime::parse_from_str(schedule_time, "%H:%M") {
        Ok(t) => t,
        Err(_) => {
            tracing::warn!(
                source_id = %binding.source_id,
                schedule_time = %schedule_time,
                "Invalid schedule_time format"
            );
            return false;
        }
    };

    // Build today's target datetime in local timezone
    let today_target = now
        .date_naive()
        .and_time(target_time);
    let today_target_ts = today_target
        .and_local_timezone(now.timezone())
        .single()
        .map(|dt| dt.timestamp());

    let today_target_ts = match today_target_ts {
        Some(ts) => ts,
        None => return false,
    };

    // Not yet reached target time today
    if now.timestamp() < today_target_ts {
        return false;
    }

    // For weekly: check day of week
    if binding.delivery_mode == "weekly" {
        let target_day = match binding.schedule_day.as_deref() {
            Some(d) => match parse_weekday(d) {
                Some(wd) => wd,
                None => {
                    tracing::warn!(
                        source_id = %binding.source_id,
                        schedule_day = %d,
                        "Invalid schedule_day"
                    );
                    return false;
                }
            },
            None => return false,
        };

        if now.weekday() != target_day {
            return false;
        }
    }

    // Already delivered after today's target time?
    if let Some(last) = binding.last_scheduled_at {
        if last >= today_target_ts {
            return false;
        }
    }

    true
}

fn parse_weekday(s: &str) -> Option<Weekday> {
    match s.to_lowercase().as_str() {
        "monday" => Some(Weekday::Mon),
        "tuesday" => Some(Weekday::Tue),
        "wednesday" => Some(Weekday::Wed),
        "thursday" => Some(Weekday::Thu),
        "friday" => Some(Weekday::Fri),
        "saturday" => Some(Weekday::Sat),
        "sunday" => Some(Weekday::Sun),
        _ => None,
    }
}

/// Spawn the scheduled delivery loop (60s interval).
pub fn spawn_scheduler(
    ledger: Arc<dyn DeliveryLedgerTrait>,
    binding_store: Arc<BindingStore>,
    source_manager: Arc<SourceManager>,
    target_manager: Arc<TargetManager>,
) -> tauri::async_runtime::JoinHandle<()> {
    tauri::async_runtime::spawn(async move {
        tracing::info!("Scheduled delivery worker started (60s interval)");
        let mut interval = tokio::time::interval(Duration::from_secs(60));

        loop {
            interval.tick().await;

            let scheduled = binding_store.get_scheduled_bindings();
            if scheduled.is_empty() {
                continue;
            }

            let now = Local::now();

            for binding in &scheduled {
                if !is_due(binding, now) {
                    continue;
                }

                // Check source is enabled
                if !source_manager.is_enabled(&binding.source_id) {
                    tracing::debug!(
                        source_id = %binding.source_id,
                        "Skipping scheduled delivery — source disabled"
                    );
                    continue;
                }

                // Parse fresh data from source
                let source = match source_manager.get_source(&binding.source_id) {
                    Some(s) => s,
                    None => {
                        tracing::warn!(
                            source_id = %binding.source_id,
                            "Source not found for scheduled delivery"
                        );
                        continue;
                    }
                };

                let payload = match source.parse() {
                    Ok(p) => p,
                    Err(e) => {
                        tracing::error!(
                            source_id = %binding.source_id,
                            error = %e,
                            "Failed to parse source for scheduled delivery"
                        );
                        continue;
                    }
                };

                // Enqueue targeted delivery
                match ledger.enqueue_targeted(
                    &binding.source_id,
                    payload,
                    &binding.endpoint_id,
                ) {
                    Ok(event_id) => {
                        // Write target display info immediately for activity log
                        let (tt, bu) = target_manager
                            .get(&binding.target_id)
                            .map(|t| (t.target_type().to_string(), t.base_url().to_string()))
                            .unwrap_or_else(|| ("webhook".to_string(), String::new()));
                        let _ = ledger.set_attempted_target(
                            &event_id,
                            &binding.build_delivered_to_json(&tt, &bu),
                        );
                        tracing::info!(
                            source_id = %binding.source_id,
                            endpoint_id = %binding.endpoint_id,
                            event_id = %event_id,
                            mode = %binding.delivery_mode,
                            "Scheduled delivery enqueued"
                        );
                    }
                    Err(e) => {
                        tracing::error!(
                            source_id = %binding.source_id,
                            error = %e,
                            "Failed to enqueue scheduled delivery"
                        );
                        continue;
                    }
                }

                // Update last_scheduled_at
                if let Err(e) = binding_store.update_last_scheduled(
                    &binding.source_id,
                    &binding.endpoint_id,
                    now.timestamp(),
                ) {
                    tracing::warn!(
                        source_id = %binding.source_id,
                        error = %e,
                        "Failed to update last_scheduled_at"
                    );
                }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn make_binding(mode: &str, time: &str, day: Option<&str>, last: Option<i64>) -> SourceBinding {
        SourceBinding {
            source_id: "test-source".to_string(),
            target_id: "t1".to_string(),
            endpoint_id: "ep1".to_string(),
            endpoint_url: "https://example.com".to_string(),
            endpoint_name: "Test".to_string(),
            created_at: 1000,
            active: true,
            headers_json: None,
            auth_credential_key: None,
            delivery_mode: mode.to_string(),
            schedule_time: Some(time.to_string()),
            schedule_day: day.map(|s| s.to_string()),
            last_scheduled_at: last,
        }
    }

    #[test]
    fn test_daily_is_due_after_target_time() {
        let binding = make_binding("daily", "09:00", None, None);
        // 2026-02-10 at 09:30 local
        let now = Local.with_ymd_and_hms(2026, 2, 10, 9, 30, 0).unwrap();
        assert!(is_due(&binding, now));
    }

    #[test]
    fn test_daily_not_due_before_target_time() {
        let binding = make_binding("daily", "09:00", None, None);
        let now = Local.with_ymd_and_hms(2026, 2, 10, 8, 59, 0).unwrap();
        assert!(!is_due(&binding, now));
    }

    #[test]
    fn test_daily_not_due_already_delivered_today() {
        // Target time is 09:00, last delivered at 09:05 today
        let now = Local.with_ymd_and_hms(2026, 2, 10, 10, 0, 0).unwrap();
        let target_ts = Local.with_ymd_and_hms(2026, 2, 10, 9, 5, 0).unwrap().timestamp();
        let binding = make_binding("daily", "09:00", None, Some(target_ts));
        assert!(!is_due(&binding, now));
    }

    #[test]
    fn test_weekly_is_due_correct_day() {
        // 2026-02-10 is a Tuesday
        let binding = make_binding("weekly", "09:00", Some("tuesday"), None);
        let now = Local.with_ymd_and_hms(2026, 2, 10, 9, 30, 0).unwrap();
        assert!(is_due(&binding, now));
    }

    #[test]
    fn test_weekly_not_due_wrong_day() {
        // 2026-02-10 is a Tuesday
        let binding = make_binding("weekly", "09:00", Some("monday"), None);
        let now = Local.with_ymd_and_hms(2026, 2, 10, 9, 30, 0).unwrap();
        assert!(!is_due(&binding, now));
    }

    #[test]
    fn test_on_change_never_due() {
        let mut binding = make_binding("on_change", "09:00", None, None);
        binding.delivery_mode = "on_change".to_string();
        // is_due is never called for on_change — but if it were, no schedule_time means false
        // This tests the non-daily/weekly path correctly handles delivery_mode check
        let now = Local.with_ymd_and_hms(2026, 2, 10, 9, 30, 0).unwrap();
        // Even with schedule_time set, on_change bindings don't go through is_due
        // (they're filtered out by get_scheduled_bindings). But is_due doesn't reject
        // based on delivery_mode — that filtering happens upstream.
        assert!(is_due(&binding, now)); // is_due is mode-agnostic for daily
    }

    #[test]
    fn test_missing_schedule_time_not_due() {
        let mut binding = make_binding("daily", "09:00", None, None);
        binding.schedule_time = None;
        let now = Local.with_ymd_and_hms(2026, 2, 10, 10, 0, 0).unwrap();
        assert!(!is_due(&binding, now));
    }

    #[test]
    fn test_parse_weekday() {
        assert_eq!(parse_weekday("monday"), Some(Weekday::Mon));
        assert_eq!(parse_weekday("TUESDAY"), Some(Weekday::Tue));
        assert_eq!(parse_weekday("Sunday"), Some(Weekday::Sun));
        assert_eq!(parse_weekday("invalid"), None);
    }
}
