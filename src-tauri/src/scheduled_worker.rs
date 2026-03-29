//! Scheduled delivery worker for daily/weekly push cadence
//!
//! Runs on a 60-second interval, checks scheduled bindings, and enqueues
//! targeted deliveries when they become due. The existing delivery worker
//! handles the actual HTTP dispatch with full WAL/retry guarantees.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::time::Duration;

use chrono::{Datelike, Local, NaiveTime, Timelike, Weekday};

use crate::bindings::{BindingStore, SourceBinding};
use crate::source_manager::{PreparedPayload, SourceManager};
use crate::sources::SourceError;
use crate::target_manager::TargetManager;
use crate::traits::DeliveryLedgerTrait;

const SCHEDULER_TICK_SECS: i64 = 60;
const SCHEDULED_TARGET_STAGGER_SECS: i64 = 10;
const TRANSIENT_FAILURE_BACKOFF_SECS: i64 = 5 * 60;
const PERMISSION_FAILURE_BACKOFF_SECS: i64 = 15 * 60;
const MAX_FAILURE_BACKOFF_SECS: i64 = 60 * 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct IntervalPhase {
    interval_secs: i64,
    offset_secs: i64,
}

#[derive(Debug, Clone)]
struct SourceFailureBackoff {
    consecutive_failures: u32,
    retry_after_ts: i64,
}

fn binding_key(source_id: &str, endpoint_id: &str) -> String {
    format!("{source_id}::{endpoint_id}")
}

fn parse_interval_minutes(binding: &SourceBinding) -> i64 {
    match &binding.schedule_time {
        Some(t) => t.parse::<i64>().ok().filter(|mins| *mins > 0).unwrap_or(15),
        None => 15,
    }
}

fn truncate_to_minute(timestamp: i64) -> i64 {
    timestamp - timestamp.rem_euclid(SCHEDULER_TICK_SECS)
}

fn build_interval_phases(bindings: &[SourceBinding]) -> HashMap<String, IntervalPhase> {
    let mut by_interval: BTreeMap<i64, Vec<&SourceBinding>> = BTreeMap::new();
    for binding in bindings.iter().filter(|b| b.delivery_mode == "interval") {
        by_interval
            .entry(parse_interval_minutes(binding))
            .or_default()
            .push(binding);
    }

    let mut phases = HashMap::new();

    for (interval_mins, mut group) in by_interval {
        let interval_secs = interval_mins * 60;
        let slot_count = (interval_secs / SCHEDULER_TICK_SECS).max(1) as usize;
        let total = group.len();

        group.sort_by(|a, b| {
            (
                a.source_id.as_str(),
                a.endpoint_id.as_str(),
                a.target_id.as_str(),
                a.created_at,
            )
                .cmp(&(
                    b.source_id.as_str(),
                    b.endpoint_id.as_str(),
                    b.target_id.as_str(),
                    b.created_at,
                ))
        });

        for (index, binding) in group.into_iter().enumerate() {
            let slot_index = (index * slot_count) / total;
            phases.insert(
                binding_key(&binding.source_id, &binding.endpoint_id),
                IntervalPhase {
                    interval_secs,
                    offset_secs: slot_index as i64 * SCHEDULER_TICK_SECS,
                },
            );
        }
    }

    phases
}

fn most_recent_interval_slot(minute_timestamp: i64, phase: IntervalPhase) -> i64 {
    let cycle_start = minute_timestamp - minute_timestamp.rem_euclid(phase.interval_secs);
    let candidate = cycle_start + phase.offset_secs;
    if candidate <= minute_timestamp {
        candidate
    } else {
        candidate - phase.interval_secs
    }
}

fn interval_due_at(
    binding: &SourceBinding,
    phase: IntervalPhase,
    minute_timestamp: i64,
) -> Option<i64> {
    let slot = most_recent_interval_slot(minute_timestamp, phase);

    match binding.last_scheduled_at {
        Some(last) if slot > last => Some(slot),
        Some(_) => None,
        None if slot == minute_timestamp => Some(slot),
        None => None,
    }
}

/// Check if a daily/weekly scheduled binding is due for delivery.
fn daily_or_weekly_due_at(binding: &SourceBinding, now: chrono::DateTime<Local>) -> Option<i64> {
    if binding.delivery_mode == "interval" || binding.delivery_mode == "on_change" {
        return None;
    }

    // Daily/weekly mode: check if target time has passed today
    let schedule_time = match &binding.schedule_time {
        Some(t) => t,
        None => return None,
    };

    let target_time = match NaiveTime::parse_from_str(schedule_time, "%H:%M") {
        Ok(t) => t,
        Err(_) => {
            tracing::warn!(
                source_id = %binding.source_id,
                schedule_time = %schedule_time,
                "Invalid schedule_time format"
            );
            return None;
        }
    };

    // Build today's target datetime in local timezone
    let today_target = now.date_naive().and_time(target_time);
    let today_target_ts = today_target
        .and_local_timezone(now.timezone())
        .single()
        .map(|dt| dt.timestamp());

    let today_target_ts = today_target_ts?;

    // Not yet reached target time today
    if now.timestamp() < today_target_ts {
        return None;
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
                    return None;
                }
            },
            None => return None,
        };

        if now.weekday() != target_day {
            return None;
        }
    }

    // Already delivered after today's target time?
    if let Some(last) = binding.last_scheduled_at {
        if last >= today_target_ts {
            return None;
        }
    }

    Some(today_target_ts)
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

fn next_backoff_delay_secs(previous_failures: u32, base_delay_secs: i64) -> i64 {
    let exponent = previous_failures.min(3);
    let multiplier = 1_i64 << exponent;
    (base_delay_secs * multiplier).min(MAX_FAILURE_BACKOFF_SECS)
}

fn classify_parse_failure(error: &crate::source_manager::SourceManagerError) -> i64 {
    match error {
        crate::source_manager::SourceManagerError::SourceError(SourceError::PermissionDenied(
            _,
        )) => PERMISSION_FAILURE_BACKOFF_SECS,
        crate::source_manager::SourceManagerError::SourceError(SourceError::ParseError(
            message,
        )) if message.contains("SQLite open")
            || message.contains("unable to open database")
            || message.contains("not authorized")
            || message.contains("JXA error") =>
        {
            PERMISSION_FAILURE_BACKOFF_SECS
        }
        _ => TRANSIENT_FAILURE_BACKOFF_SECS,
    }
}

fn update_last_scheduled_for_due_bindings(
    binding_store: &BindingStore,
    due_bindings: &[(SourceBinding, i64)],
) {
    for (binding, due_at) in due_bindings {
        if let Err(e) =
            binding_store.update_last_scheduled(&binding.source_id, &binding.endpoint_id, *due_at)
        {
            tracing::warn!(
                source_id = %binding.source_id,
                error = %e,
                "Failed to update last_scheduled_at"
            );
        }
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
        let mut source_backoff: HashMap<String, SourceFailureBackoff> = HashMap::new();
        let now = Local::now();
        let nanos_until_next_minute =
            60_000_000_000_u64 - ((now.second() as u64 * 1_000_000_000) + now.nanosecond() as u64);
        let first_tick_delay = if nanos_until_next_minute == 0 {
            Duration::from_secs(SCHEDULER_TICK_SECS as u64)
        } else {
            Duration::from_nanos(nanos_until_next_minute)
        };
        let mut interval = tokio::time::interval_at(
            tokio::time::Instant::now() + first_tick_delay,
            Duration::from_secs(SCHEDULER_TICK_SECS as u64),
        );

        loop {
            interval.tick().await;

            let scheduled = binding_store.get_scheduled_bindings();
            if scheduled.is_empty() {
                continue;
            }

            let now = Local::now();
            let current_minute_ts = truncate_to_minute(now.timestamp());
            let interval_phases = build_interval_phases(&scheduled);
            let mut due_by_source: BTreeMap<String, Vec<(SourceBinding, i64)>> = BTreeMap::new();

            for binding in scheduled {
                let due_at = if binding.delivery_mode == "interval" {
                    interval_phases
                        .get(&binding_key(&binding.source_id, &binding.endpoint_id))
                        .and_then(|phase| interval_due_at(&binding, *phase, current_minute_ts))
                } else {
                    daily_or_weekly_due_at(&binding, now)
                };

                if let Some(due_at) = due_at {
                    due_by_source
                        .entry(binding.source_id.clone())
                        .or_default()
                        .push((binding, due_at));
                }
            }

            for (source_id, mut due_bindings) in due_by_source {
                if !source_manager.is_enabled(&source_id) {
                    tracing::debug!(
                        source_id = %source_id,
                        "Skipping scheduled delivery — source disabled"
                    );
                    continue;
                }

                if let Some(backoff) = source_backoff.get(&source_id) {
                    if backoff.retry_after_ts > now.timestamp() {
                        tracing::debug!(
                            source_id = %source_id,
                            retry_after = backoff.retry_after_ts,
                            consecutive_failures = backoff.consecutive_failures,
                            "Skipping scheduled source during parse backoff"
                        );
                        continue;
                    }
                }

                due_bindings.sort_by(|(binding_a, due_a), (binding_b, due_b)| {
                    (
                        *due_a,
                        binding_a.endpoint_id.as_str(),
                        binding_a.target_id.as_str(),
                    )
                        .cmp(&(
                            *due_b,
                            binding_b.endpoint_id.as_str(),
                            binding_b.target_id.as_str(),
                        ))
                });

                let payload = match source_manager.prepare_payload_for_delivery(&source_id) {
                    Ok(PreparedPayload::Deliver(payload)) => payload,
                    Ok(PreparedPayload::Skip(reason)) => {
                        tracing::debug!(
                            source_id = %source_id,
                            reason = reason.as_str(),
                            "Skipping scheduled delivery with no new deliverable payload"
                        );
                        update_last_scheduled_for_due_bindings(&binding_store, &due_bindings);
                        continue;
                    }
                    Err(e) => {
                        let base_delay_secs = classify_parse_failure(&e);
                        let next_state = {
                            let previous_failures = source_backoff
                                .get(&source_id)
                                .map(|state| state.consecutive_failures)
                                .unwrap_or(0);
                            let delay_secs =
                                next_backoff_delay_secs(previous_failures, base_delay_secs);
                            SourceFailureBackoff {
                                consecutive_failures: previous_failures + 1,
                                retry_after_ts: now.timestamp() + delay_secs,
                            }
                        };
                        let delay_minutes = (next_state.retry_after_ts - now.timestamp()) / 60;
                        source_backoff.insert(source_id.clone(), next_state.clone());
                        tracing::error!(
                            source_id = %source_id,
                            error = %e,
                            retry_after = next_state.retry_after_ts,
                            backoff_minutes = delay_minutes,
                            consecutive_failures = next_state.consecutive_failures,
                            "Failed to parse source for scheduled delivery; backing off"
                        );
                        continue;
                    }
                };
                source_backoff.remove(&source_id);

                let mut successful_bindings: Vec<(SourceBinding, i64)> = Vec::new();

                for (index, (binding, due_at)) in due_bindings.iter().enumerate() {
                    let available_at =
                        current_minute_ts + index as i64 * SCHEDULED_TARGET_STAGGER_SECS;

                    match ledger.enqueue_targeted_at(
                        &binding.source_id,
                        payload.clone(),
                        &binding.endpoint_id,
                        available_at,
                    ) {
                        Ok(event_id) => {
                            if let Err(e) = source_manager.handle_delivery_queued(
                                &binding.source_id,
                                &event_id,
                                &payload,
                            ) {
                                tracing::warn!(
                                    source_id = %binding.source_id,
                                    event_id = %event_id,
                                    error = %e,
                                    "Failed to record source delivery bookkeeping"
                                );
                            }
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
                                due_at = *due_at,
                                available_at = available_at,
                                "Scheduled delivery enqueued"
                            );
                            successful_bindings.push((binding.clone(), *due_at));
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
                }

                if !successful_bindings.is_empty() {
                    if let Err(e) =
                        source_manager.remember_payload_fingerprint(&source_id, &payload)
                    {
                        tracing::warn!(
                            source_id = %source_id,
                            error = %e,
                            "Failed to persist payload fingerprint after scheduled delivery"
                        );
                    }

                    update_last_scheduled_for_due_bindings(&binding_store, &successful_bindings);
                }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source_manager::SourceManagerError;
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
        assert_eq!(
            daily_or_weekly_due_at(&binding, now),
            Some(
                Local
                    .with_ymd_and_hms(2026, 2, 10, 9, 0, 0)
                    .unwrap()
                    .timestamp()
            )
        );
    }

    #[test]
    fn test_daily_not_due_before_target_time() {
        let binding = make_binding("daily", "09:00", None, None);
        let now = Local.with_ymd_and_hms(2026, 2, 10, 8, 59, 0).unwrap();
        assert_eq!(daily_or_weekly_due_at(&binding, now), None);
    }

    #[test]
    fn test_daily_not_due_already_delivered_today() {
        // Target time is 09:00, last delivered at 09:05 today
        let now = Local.with_ymd_and_hms(2026, 2, 10, 10, 0, 0).unwrap();
        let target_ts = Local
            .with_ymd_and_hms(2026, 2, 10, 9, 5, 0)
            .unwrap()
            .timestamp();
        let binding = make_binding("daily", "09:00", None, Some(target_ts));
        assert_eq!(daily_or_weekly_due_at(&binding, now), None);
    }

    #[test]
    fn test_weekly_is_due_correct_day() {
        // 2026-02-10 is a Tuesday
        let binding = make_binding("weekly", "09:00", Some("tuesday"), None);
        let now = Local.with_ymd_and_hms(2026, 2, 10, 9, 30, 0).unwrap();
        assert_eq!(
            daily_or_weekly_due_at(&binding, now),
            Some(
                Local
                    .with_ymd_and_hms(2026, 2, 10, 9, 0, 0)
                    .unwrap()
                    .timestamp()
            )
        );
    }

    #[test]
    fn test_weekly_not_due_wrong_day() {
        // 2026-02-10 is a Tuesday
        let binding = make_binding("weekly", "09:00", Some("monday"), None);
        let now = Local.with_ymd_and_hms(2026, 2, 10, 9, 30, 0).unwrap();
        assert_eq!(daily_or_weekly_due_at(&binding, now), None);
    }

    #[test]
    fn test_on_change_never_due() {
        let mut binding = make_binding("on_change", "09:00", None, None);
        binding.delivery_mode = "on_change".to_string();
        let now = Local.with_ymd_and_hms(2026, 2, 10, 9, 30, 0).unwrap();
        assert_eq!(daily_or_weekly_due_at(&binding, now), None);
    }

    #[test]
    fn test_interval_binding_without_history_waits_for_its_assigned_slot() {
        let binding = make_binding("interval", "10", None, None);
        let phase = IntervalPhase {
            interval_secs: 600,
            offset_secs: 240,
        };

        assert_eq!(interval_due_at(&binding, phase, 1_800), None);
        assert_eq!(interval_due_at(&binding, phase, 2_040), Some(2_040));
    }

    #[test]
    fn test_interval_schedule_self_heals_bunched_last_run_timestamps() {
        let binding = make_binding("interval", "10", None, Some(2_005));

        let early_phase = IntervalPhase {
            interval_secs: 600,
            offset_secs: 0,
        };
        let late_phase = IntervalPhase {
            interval_secs: 600,
            offset_secs: 240,
        };

        assert_eq!(interval_due_at(&binding, early_phase, 2_040), None);
        assert_eq!(interval_due_at(&binding, late_phase, 2_040), Some(2_040));
    }

    #[test]
    fn test_interval_phases_evenly_spread_ten_bindings_over_ten_minutes() {
        let bindings: Vec<SourceBinding> = (0..10)
            .map(|index| SourceBinding {
                source_id: format!("source-{index}"),
                target_id: "t1".to_string(),
                endpoint_id: "ep1".to_string(),
                endpoint_url: "https://example.com".to_string(),
                endpoint_name: "Test".to_string(),
                created_at: index,
                active: true,
                headers_json: None,
                auth_credential_key: None,
                delivery_mode: "interval".to_string(),
                schedule_time: Some("10".to_string()),
                schedule_day: None,
                last_scheduled_at: None,
            })
            .collect();

        let phases = build_interval_phases(&bindings);
        let mut offsets: Vec<i64> = bindings
            .iter()
            .map(|binding| {
                phases[&binding_key(&binding.source_id, &binding.endpoint_id)].offset_secs
            })
            .collect();
        offsets.sort_unstable();

        assert_eq!(offsets, vec![0, 60, 120, 180, 240, 300, 360, 420, 480, 540]);
    }

    #[test]
    fn test_permission_failures_use_longer_backoff() {
        let error = SourceManagerError::SourceError(SourceError::PermissionDenied(
            "Cannot access Apple Photos library".to_string(),
        ));

        assert_eq!(
            classify_parse_failure(&error),
            PERMISSION_FAILURE_BACKOFF_SECS
        );
        assert_eq!(
            next_backoff_delay_secs(0, classify_parse_failure(&error)),
            PERMISSION_FAILURE_BACKOFF_SECS
        );
        assert_eq!(
            next_backoff_delay_secs(2, classify_parse_failure(&error)),
            60 * 60
        );
    }

    #[test]
    fn test_transient_failures_use_shorter_backoff() {
        let error = SourceManagerError::SourceError(SourceError::ParseError(
            "unexpected schema drift".to_string(),
        ));

        assert_eq!(
            classify_parse_failure(&error),
            TRANSIENT_FAILURE_BACKOFF_SECS
        );
        assert_eq!(
            next_backoff_delay_secs(0, classify_parse_failure(&error)),
            TRANSIENT_FAILURE_BACKOFF_SECS
        );
        assert_eq!(
            next_backoff_delay_secs(1, classify_parse_failure(&error)),
            10 * 60
        );
    }

    #[test]
    fn test_missing_schedule_time_not_due() {
        let mut binding = make_binding("daily", "09:00", None, None);
        binding.schedule_time = None;
        let now = Local.with_ymd_and_hms(2026, 2, 10, 10, 0, 0).unwrap();
        assert_eq!(daily_or_weekly_due_at(&binding, now), None);
    }

    #[test]
    fn test_parse_weekday() {
        assert_eq!(parse_weekday("monday"), Some(Weekday::Mon));
        assert_eq!(parse_weekday("TUESDAY"), Some(Weekday::Tue));
        assert_eq!(parse_weekday("Sunday"), Some(Weekday::Sun));
        assert_eq!(parse_weekday("invalid"), None);
    }
}
