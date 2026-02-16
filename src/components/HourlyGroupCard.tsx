import { useState } from "react";
import {
  CheckCircle2,
  ChevronDown,
  ChevronRight,
  ExternalLink,
} from "lucide-react";
import type { ActivityEntry } from "../api/hooks/useActivityLog";
import { openUrl } from "../utils/openUrl";
import { ActivityCard } from "./ActivityCard";

const targetLabel = (targetType: string): string => {
  const labels: Record<string, string> = {
    "google-sheets": "Google Sheets",
    n8n: "n8n",
    ntfy: "ntfy",
    make: "Make",
    zapier: "Zapier",
    webhook: "Webhook",
  };
  return labels[targetType] || targetType;
};

export interface HourlyGroupData {
  key: string;
  source: string;
  targetType: string;
  targetUrl?: string;
  entries: ActivityEntry[];
  latestTime: Date;
  earliestTime: Date;
}

interface HourlyGroupCardProps {
  group: HourlyGroupData;
}

export function HourlyGroupCard({ group }: HourlyGroupCardProps) {
  const [expanded, setExpanded] = useState(false);

  const formatHour = (date: Date): string =>
    date.toLocaleTimeString("en-US", {
      hour12: false,
      hour: "2-digit",
      minute: "2-digit",
    });

  return (
    <div>
      {/* Group summary row */}
      <div
        onClick={() => setExpanded(!expanded)}
        className="flex items-center gap-2 px-3 py-2 rounded-md cursor-pointer hover:bg-bg-tertiary transition-colors"
      >
        <CheckCircle2 size={14} className="text-success shrink-0" />
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2">
            <span className="text-xs font-medium truncate">
              {group.source}
            </span>
            <span
              className={`text-[10px] truncate ${group.targetUrl ? "text-accent hover:underline cursor-pointer" : "text-text-secondary"}`}
              onClick={(e) => {
                if (group.targetUrl) {
                  e.stopPropagation();
                  openUrl(group.targetUrl);
                }
              }}
            >
              → {targetLabel(group.targetType)}
              {group.targetUrl && (
                <ExternalLink size={9} className="inline ml-0.5 -mt-0.5" />
              )}
            </span>
            <span className="px-1.5 py-0.5 rounded text-[9px] font-medium bg-success/10 text-success shrink-0">
              {group.entries.length} pushes
            </span>
          </div>
        </div>
        <span className="text-[11px] font-mono text-text-secondary shrink-0">
          {formatHour(group.earliestTime)}
        </span>
        {expanded ? (
          <ChevronDown size={12} className="text-text-secondary shrink-0" />
        ) : (
          <ChevronRight size={12} className="text-text-secondary shrink-0" />
        )}
      </div>

      {/* Expanded: individual entries */}
      {expanded && (
        <div className="ml-4 border-l border-border/50">
          {group.entries.map((entry) => (
            <ActivityCard key={entry.id} entry={entry} />
          ))}
        </div>
      )}
    </div>
  );
}
