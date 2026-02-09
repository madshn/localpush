import { useState } from "react";
import { Search, X } from "lucide-react";
import {
  useActivityLog,
  type ActivityEntry,
} from "../api/hooks/useActivityLog";
import { ActivityCard } from "./ActivityCard";
import { DateDivider } from "./DateDivider";

function groupByDate(
  entries: ActivityEntry[]
): Map<string, ActivityEntry[]> {
  const groups = new Map<string, ActivityEntry[]>();
  for (const entry of entries) {
    const key = entry.timestamp.toDateString();
    const group = groups.get(key);
    if (group) {
      group.push(entry);
    } else {
      groups.set(key, [entry]);
    }
  }
  return groups;
}

export function ActivityLog() {
  const { data: entries, isLoading } = useActivityLog();
  const [searchFilter, setSearchFilter] = useState("");

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

  const filtered = searchFilter
    ? entries.filter((e) =>
        e.source.toLowerCase().includes(searchFilter.toLowerCase())
      )
    : entries;

  const dateGroups = groupByDate(filtered);

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
        {Array.from(dateGroups.entries()).map(([dateKey, groupEntries]) => (
          <div key={dateKey}>
            <DateDivider date={groupEntries[0].timestamp} />
            {groupEntries.map((entry) => (
              <ActivityCard key={entry.id} entry={entry} />
            ))}
          </div>
        ))}
      </div>

      {searchFilter && filtered.length === 0 && (
        <div className="text-center py-6 text-text-secondary text-xs">
          No entries matching &ldquo;{searchFilter}&rdquo;
        </div>
      )}
    </div>
  );
}
