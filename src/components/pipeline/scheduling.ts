import type { Binding } from "../../api/hooks/useBindings";

const SCHEDULER_TICK_SECONDS = 60;

interface IntervalPhase {
  intervalSeconds: number;
  offsetSeconds: number;
}

function bindingKey(binding: Pick<Binding, "source_id" | "endpoint_id">): string {
  return `${binding.source_id}::${binding.endpoint_id}`;
}

function parseIntervalMinutes(binding: Binding): number {
  const parsed = Number(binding.schedule_time ?? "15");
  return Number.isFinite(parsed) && parsed > 0 ? parsed : 15;
}

function truncateToMinute(timestampSeconds: number): number {
  return timestampSeconds - (timestampSeconds % SCHEDULER_TICK_SECONDS);
}

function buildIntervalPhases(bindings: Binding[]): Map<string, IntervalPhase> {
  const byInterval = new Map<number, Binding[]>();

  for (const binding of bindings) {
    if (binding.delivery_mode !== "interval") continue;
    const intervalMinutes = parseIntervalMinutes(binding);
    const group = byInterval.get(intervalMinutes) ?? [];
    group.push(binding);
    byInterval.set(intervalMinutes, group);
  }

  const phases = new Map<string, IntervalPhase>();

  for (const [intervalMinutes, group] of byInterval) {
    const intervalSeconds = intervalMinutes * 60;
    const slotCount = Math.max(1, Math.floor(intervalSeconds / SCHEDULER_TICK_SECONDS));
    const sorted = [...group].sort((a, b) => {
      const createdA = Number(a.created_at) || 0;
      const createdB = Number(b.created_at) || 0;
      return (
        a.source_id.localeCompare(b.source_id) ||
        a.endpoint_id.localeCompare(b.endpoint_id) ||
        a.target_id.localeCompare(b.target_id) ||
        createdA - createdB
      );
    });

    sorted.forEach((binding, index) => {
      const slotIndex = Math.floor((index * slotCount) / sorted.length);
      phases.set(bindingKey(binding), {
        intervalSeconds,
        offsetSeconds: slotIndex * SCHEDULER_TICK_SECONDS,
      });
    });
  }

  return phases;
}

function mostRecentIntervalSlot(minuteTimestamp: number, phase: IntervalPhase): number {
  const cycleStart = minuteTimestamp - (minuteTimestamp % phase.intervalSeconds);
  const candidate = cycleStart + phase.offsetSeconds;
  return candidate <= minuteTimestamp ? candidate : candidate - phase.intervalSeconds;
}

function nextIntervalPushAt(
  binding: Binding,
  phase: IntervalPhase,
  nowSeconds: number,
): number {
  const minuteTimestamp = truncateToMinute(nowSeconds);
  const recentSlot = mostRecentIntervalSlot(minuteTimestamp, phase);
  const lastScheduledAt = binding.last_scheduled_at;

  if (lastScheduledAt == null) {
    return recentSlot === minuteTimestamp ? recentSlot : recentSlot + phase.intervalSeconds;
  }

  return recentSlot > lastScheduledAt ? recentSlot : recentSlot + phase.intervalSeconds;
}

function nextDailyPushAt(binding: Binding, now: Date): number | null {
  if (!binding.schedule_time) return null;
  const [hourStr, minuteStr] = binding.schedule_time.split(":");
  const hour = Number(hourStr);
  const minute = Number(minuteStr);
  if (!Number.isFinite(hour) || !Number.isFinite(minute)) return null;

  const candidate = new Date(now);
  candidate.setSeconds(0, 0);
  candidate.setHours(hour, minute, 0, 0);

  const lastScheduledMs =
    binding.last_scheduled_at != null ? binding.last_scheduled_at * 1000 : null;

  if (candidate.getTime() > now.getTime()) {
    return candidate.getTime();
  }

  if (lastScheduledMs == null || lastScheduledMs < candidate.getTime()) {
    return candidate.getTime();
  }

  candidate.setDate(candidate.getDate() + 1);
  return candidate.getTime();
}

const weekdayMap: Record<string, number> = {
  sunday: 0,
  monday: 1,
  tuesday: 2,
  wednesday: 3,
  thursday: 4,
  friday: 5,
  saturday: 6,
};

function nextWeeklyPushAt(binding: Binding, now: Date): number | null {
  if (!binding.schedule_time || !binding.schedule_day) return null;
  const [hourStr, minuteStr] = binding.schedule_time.split(":");
  const hour = Number(hourStr);
  const minute = Number(minuteStr);
  const targetDay = weekdayMap[binding.schedule_day.toLowerCase()];
  if (!Number.isFinite(hour) || !Number.isFinite(minute) || targetDay == null) return null;

  const candidate = new Date(now);
  candidate.setSeconds(0, 0);
  candidate.setHours(hour, minute, 0, 0);
  const dayDelta = (targetDay - candidate.getDay() + 7) % 7;
  candidate.setDate(candidate.getDate() + dayDelta);

  const lastScheduledMs =
    binding.last_scheduled_at != null ? binding.last_scheduled_at * 1000 : null;

  if (candidate.getTime() > now.getTime()) {
    return candidate.getTime();
  }

  if (lastScheduledMs == null || lastScheduledMs < candidate.getTime()) {
    return candidate.getTime();
  }

  candidate.setDate(candidate.getDate() + 7);
  return candidate.getTime();
}

function nextPushAt(binding: Binding, intervalPhases: Map<string, IntervalPhase>, nowMs: number): number | null {
  if (binding.delivery_mode === "interval") {
    const phase = intervalPhases.get(bindingKey(binding));
    if (!phase) return null;
    return nextIntervalPushAt(binding, phase, Math.floor(nowMs / 1000)) * 1000;
  }

  if (binding.delivery_mode === "daily") {
    return nextDailyPushAt(binding, new Date(nowMs));
  }

  if (binding.delivery_mode === "weekly") {
    return nextWeeklyPushAt(binding, new Date(nowMs));
  }

  return null;
}

export function getNextPushBySource(bindings: Binding[], nowMs: number): Map<string, number> {
  const intervalPhases = buildIntervalPhases(bindings);
  const nextBySource = new Map<string, number>();

  for (const binding of bindings) {
    const nextAt = nextPushAt(binding, intervalPhases, nowMs);
    if (nextAt == null) continue;

    const current = nextBySource.get(binding.source_id);
    if (current == null || nextAt < current) {
      nextBySource.set(binding.source_id, nextAt);
    }
  }

  return nextBySource;
}

export function formatNextPushLabel(nextPushAtMs: number | null | undefined, nowMs: number): string | null {
  if (nextPushAtMs == null) return null;

  const diffMs = nextPushAtMs - nowMs;
  if (diffMs <= 30_000) {
    return "Next push due now";
  }

  const diffMinutes = Math.ceil(diffMs / 60_000);
  if (diffMinutes < 60) {
    return `Next push in ${diffMinutes}m`;
  }

  const diffHours = Math.floor(diffMinutes / 60);
  const remainingMinutes = diffMinutes % 60;
  if (diffHours < 24) {
    return remainingMinutes > 0
      ? `Next push in ${diffHours}h ${remainingMinutes}m`
      : `Next push in ${diffHours}h`;
  }

  return `Next push ${new Date(nextPushAtMs).toLocaleString("en-US", {
    weekday: "short",
    hour: "2-digit",
    minute: "2-digit",
    hour12: false,
  })}`;
}
