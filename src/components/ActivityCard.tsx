import { useState } from "react";
import {
  CheckCircle2,
  Clock,
  AlertTriangle,
  Skull,
  ChevronDown,
  ChevronRight,
  RotateCcw,
} from "lucide-react";
import { toast } from "sonner";
import { invoke } from "@tauri-apps/api/core";
import type { ActivityEntry } from "../api/hooks/useActivityLog";

interface ActivityCardProps {
  entry: ActivityEntry;
}

const statusConfig = {
  delivered: {
    icon: CheckCircle2,
    color: "text-success",
    borderColor: "",
  },
  pending: {
    icon: Clock,
    color: "text-warning",
    borderColor: "",
  },
  in_flight: {
    icon: Clock,
    color: "text-warning",
    borderColor: "",
  },
  failed: {
    icon: AlertTriangle,
    color: "text-error",
    borderColor: "border-l-2 border-l-error",
  },
  dlq: {
    icon: Skull,
    color: "text-error",
    borderColor: "border-l-2 border-l-error",
  },
} as const;

const statusLabels: Record<string, string> = {
  delivered: "Delivered",
  pending: "Waiting to send",
  in_flight: "Sending...",
  failed: "Failed",
  dlq: "Gave up after 5 retries",
};

export function ActivityCard({ entry }: ActivityCardProps) {
  const [expanded, setExpanded] = useState(false);
  const config = statusConfig[entry.status];
  const Icon = config.icon;

  const formatTime = (date: Date): string =>
    date.toLocaleTimeString("en-US", {
      hour12: false,
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
    });

  const formatFullTimestamp = (date: Date): string =>
    date.toLocaleString("en-US", {
      year: "numeric",
      month: "short",
      day: "numeric",
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
      hour12: false,
    });

  const handleRetry = async () => {
    try {
      await invoke("retry_delivery", { entryId: entry.id });
      toast.success("Delivery queued for retry");
    } catch (error) {
      toast.error(`Retry failed: ${error}`);
    }
  };

  return (
    <div className={`${config.borderColor}`}>
      {/* Summary row */}
      <div
        onClick={() => setExpanded(!expanded)}
        className="flex items-center gap-2 px-3 py-2 rounded-md cursor-pointer hover:bg-bg-tertiary transition-colors"
      >
        <Icon size={14} className={config.color} />
        <span className="text-xs font-medium min-w-[80px] truncate">
          {entry.source}
        </span>
        <span className={`text-xs ${config.color} flex-1 truncate`}>
          {entry.error || statusLabels[entry.status]}
          {(entry.status === "failed" || entry.status === "dlq") &&
            entry.retryCount > 0 && (
              <span className="text-text-secondary ml-1">
                (retry {entry.retryCount}/5)
              </span>
            )}
        </span>
        <span className="text-[11px] font-mono text-text-secondary shrink-0">
          {formatTime(entry.timestamp)}
        </span>
        {expanded ? (
          <ChevronDown size={12} className="text-text-secondary shrink-0" />
        ) : (
          <ChevronRight size={12} className="text-text-secondary shrink-0" />
        )}
      </div>

      {/* Expanded detail */}
      {expanded && (
        <div className="mx-3 mt-1 mb-2 p-3 bg-bg-primary rounded-md text-xs font-mono text-text-secondary leading-relaxed">
          <div>
            <strong className="text-text-primary">ID:</strong> {entry.id}
          </div>
          <div>
            <strong className="text-text-primary">Source:</strong> {entry.source}
          </div>
          <div>
            <strong className="text-text-primary">Status:</strong> {entry.status}
          </div>
          <div>
            <strong className="text-text-primary">Created:</strong>{" "}
            {formatFullTimestamp(entry.timestamp)}
          </div>
          {entry.deliveredAt && (
            <div>
              <strong className="text-text-primary">Delivered:</strong>{" "}
              {formatFullTimestamp(entry.deliveredAt)}
            </div>
          )}
          <div>
            <strong className="text-text-primary">Retry count:</strong>{" "}
            {entry.retryCount}
          </div>
          {entry.error && (
            <div className="text-error mt-1">
              <strong>Error:</strong> {entry.error}
            </div>
          )}

          {/* Retry button for failed entries */}
          {(entry.status === "failed" || entry.status === "dlq") && (
            <button
              onClick={(e) => {
                e.stopPropagation();
                handleRetry();
              }}
              className="mt-2 flex items-center gap-1.5 text-[11px] font-medium text-accent hover:underline"
            >
              <RotateCcw size={12} />
              Retry
            </button>
          )}
        </div>
      )}
    </div>
  );
}
