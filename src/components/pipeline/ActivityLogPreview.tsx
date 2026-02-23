import {
  CheckCircle2,
  Clock,
  AlertCircle,
  ArrowRight,
} from "lucide-react";
import { useRecentActivityLog } from "../../api/hooks/useActivityLog";
import type { ActivityEntry } from "../../api/hooks/useActivityLog";

const statusIcon: Record<
  ActivityEntry["status"],
  { icon: typeof CheckCircle2; className: string }
> = {
  delivered: { icon: CheckCircle2, className: "text-success" },
  pending: { icon: Clock, className: "text-warning" },
  in_flight: { icon: Clock, className: "text-accent" },
  failed: { icon: AlertCircle, className: "text-error" },
  dlq: { icon: AlertCircle, className: "text-error" },
};

function timeAgo(date: Date): string {
  const seconds = Math.floor((Date.now() - date.getTime()) / 1000);
  if (seconds < 60) return "just now";
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  return `${days}d ago`;
}

interface ActivityLogPreviewProps {
  onViewAll: () => void;
}

export function ActivityLogPreview({ onViewAll }: ActivityLogPreviewProps) {
  const { data: recent } = useRecentActivityLog(3);
  const previewEntries = recent || [];

  if (previewEntries.length === 0) {
    return (
      <div className="bg-bg-secondary border border-border rounded-lg p-3">
        <span className="text-[11px] font-medium text-text-secondary uppercase tracking-wide">
          Recent Activity
        </span>
        <p className="text-xs text-text-secondary/60 mt-2">
          No deliveries yet
        </p>
      </div>
    );
  }

  return (
    <div className="bg-bg-secondary border border-border rounded-lg p-3">
      <div className="flex items-center justify-between mb-2">
        <span className="text-[11px] font-medium text-text-secondary uppercase tracking-wide">
          Recent Activity
        </span>
        <button
          onClick={onViewAll}
          className="inline-flex items-center gap-1 text-[10px] font-medium text-accent hover:underline"
        >
          View all
          <ArrowRight size={10} />
        </button>
      </div>

      <div className="flex flex-col gap-1.5">
        {previewEntries.map((entry) => {
          const { icon: StatusIcon, className } =
            statusIcon[entry.status] || statusIcon.pending;
          return (
            <div
              key={entry.id}
              className="flex items-center gap-2 px-2 py-1.5 bg-bg-primary rounded-md"
            >
              <StatusIcon size={12} className={`shrink-0 ${className}`} />
              <span className="text-[11px] font-medium truncate flex-1">
                {entry.source}
              </span>
              <span className="text-[10px] text-text-secondary shrink-0">
                {timeAgo(entry.timestamp)}
              </span>
            </div>
          );
        })}
      </div>
    </div>
  );
}
