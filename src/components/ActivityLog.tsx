import { useDeferredValue, useMemo, useState } from "react";
import { Search, X } from "lucide-react";
import {
  useActivityLog,
  type ActivityEntry,
} from "../api/hooks/useActivityLog";
import { ActivityCard } from "./ActivityCard";
import { HourlyGroupCard, type HourlyGroupData } from "./HourlyGroupCard";
import { DateDivider } from "./DateDivider";

type ActivityItem =
  | { type: "entry"; entry: ActivityEntry }
  | { type: "hourGroup"; group: HourlyGroupData };

/** Group key for collapsible hourly buckets: source + target + date + hour */
function buildGroupKey(entry: ActivityEntry): string | null {
  if (entry.triggerType !== "file_change") return null;
  if (entry.status !== "delivered") return null;
  if (!entry.deliveredTo) return null;
  const hour = entry.timestamp.getHours();
  const dateKey = entry.timestamp.toDateString();
  return `${entry.sourceId}|${entry.deliveredTo.target_type}|${dateKey}|${hour}`;
}

/** Collapse file_change delivered entries into hourly groups.
 *  Manual, scheduled, failed, pending entries stay ungrouped. */
function groupIntoHourlyBuckets(entries: ActivityEntry[]): ActivityItem[] {
  const groups = new Map<string, HourlyGroupData>();
  const entryGroupKey = new Map<string, string>();

  // First pass: collect groups
  for (const entry of entries) {
    const key = buildGroupKey(entry);
    if (!key) continue;
    entryGroupKey.set(entry.id, key);
    const existing = groups.get(key);
    if (existing) {
      existing.entries.push(entry);
      if (entry.timestamp < existing.earliestTime) {
        existing.earliestTime = entry.timestamp;
      }
    } else {
      groups.set(key, {
        key,
        source: entry.source,
        targetType: entry.deliveredTo!.target_type,
        targetUrl: entry.deliveredTo!.target_url,
        entries: [entry],
        latestTime: entry.timestamp,
        earliestTime: entry.timestamp,
      });
    }
  }

  // Second pass: build item list preserving chronological position
  const seenGroups = new Set<string>();
  const items: ActivityItem[] = [];

  for (const entry of entries) {
    const key = entryGroupKey.get(entry.id);
    if (key) {
      if (!seenGroups.has(key)) {
        seenGroups.add(key);
        const group = groups.get(key)!;
        if (group.entries.length === 1) {
          items.push({ type: "entry", entry: group.entries[0] });
        } else {
          items.push({ type: "hourGroup", group });
        }
      }
      // Skip subsequent entries — they're inside the group
    } else {
      items.push({ type: "entry", entry });
    }
  }

  return items;
}

function groupByDate(
  items: ActivityItem[]
): Map<string, ActivityItem[]> {
  const groups = new Map<string, ActivityItem[]>();
  for (const item of items) {
    const ts =
      item.type === "entry"
        ? item.entry.timestamp
        : item.group.latestTime;
    const key = ts.toDateString();
    const group = groups.get(key);
    if (group) {
      group.push(item);
    } else {
      groups.set(key, [item]);
    }
  }
  return groups;
}

export function ActivityLog() {
  const { data: entries, isLoading } = useActivityLog();
  const [searchFilter, setSearchFilter] = useState("");
  const deferredSearchFilter = useDeferredValue(searchFilter);

  if (isLoading) {
    return (
      <div className="text-center py-8 text-text-secondary text-sm">
        Loading activity...
      </div>
    );
  }

  if (!entries || entries.length === 0) {
    return (
      <div className="text-center py-12">
        <p className="text-text-secondary text-sm">
          No deliveries yet. Enable a source to start pushing data.
        </p>
      </div>
    );
  }

  const filtered = useMemo(() => {
    if (!entries) return [];
    if (!deferredSearchFilter) return entries;
    const needle = deferredSearchFilter.toLowerCase();
    return entries.filter((e) => e.source.toLowerCase().includes(needle));
  }, [entries, deferredSearchFilter]);

  const items = useMemo(() => groupIntoHourlyBuckets(filtered), [filtered]);
  const dateGroups = useMemo(() => groupByDate(items), [items]);

  return (
    <div>
      {/* Search */}
      <div className="relative mb-3">
        <Search
          size={14}
          className="absolute left-2.5 top-1/2 -translate-y-1/2 text-text-secondary"
        />
        <input
          type="text"
          placeholder="Search logs..."
          value={searchFilter}
          onChange={(e) => setSearchFilter(e.target.value)}
          className="w-full pl-8 pr-8 py-2 text-xs border border-border rounded-lg bg-bg-secondary text-text-primary placeholder:text-text-secondary/50 focus:outline-none focus:border-accent"
        />
        {searchFilter && (
          <button
            onClick={() => setSearchFilter("")}
            className="absolute right-2 top-1/2 -translate-y-1/2 text-text-secondary hover:text-text-primary"
          >
            <X size={14} />
          </button>
        )}
      </div>

      {/* Date-grouped entries */}
      <div className="flex flex-col gap-1">
        {Array.from(dateGroups.entries()).map(([dateKey, groupItems]) => {
          const firstDate =
            groupItems[0].type === "entry"
              ? groupItems[0].entry.timestamp
              : groupItems[0].group.latestTime;
          return (
            <div key={dateKey}>
              <DateDivider date={firstDate} />
              {groupItems.map((item) =>
                item.type === "entry" ? (
                  <ActivityCard key={item.entry.id} entry={item.entry} />
                ) : (
                  <HourlyGroupCard key={item.group.key} group={item.group} />
                )
              )}
            </div>
          );
        })}
      </div>

      {deferredSearchFilter && filtered.length === 0 && (
        <div className="text-center py-6 text-text-secondary text-xs">
          No entries matching &ldquo;{deferredSearchFilter}&rdquo;
        </div>
      )}
    </div>
  );
}
