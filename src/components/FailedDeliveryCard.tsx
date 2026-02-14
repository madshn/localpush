import { useState } from "react";
import {
  AlertTriangle,
  Skull,
  ChevronDown,
  ChevronRight,
  RotateCcw,
  Trash2,
} from "lucide-react";
import type { ActivityEntry } from "../api/hooks/useActivityLog";
import {
  useErrorDiagnosis,
  useRetryHistory,
} from "../api/hooks/useErrorDiagnosis";
import { useDismissDlq, useReplayDelivery } from "../api/hooks/useDlqActions";
import { ReplayConfirmDialog } from "./ReplayConfirmDialog";

interface FailedDeliveryCardProps {
  entry: ActivityEntry;
}

const statusConfig = {
  failed: {
    icon: AlertTriangle,
    color: "text-error",
    borderColor: "border-l-2 border-l-error",
    label: "Failed",
  },
  dlq: {
    icon: Skull,
    color: "text-error",
    borderColor: "border-l-2 border-l-error",
    label: "Gave up after 5 retries",
  },
} as const;

export function FailedDeliveryCard({ entry }: FailedDeliveryCardProps) {
  const [expanded, setExpanded] = useState(false);
  const [showReplayDialog, setShowReplayDialog] = useState(false);

  const { data: diagnosis } = useErrorDiagnosis(
    expanded ? entry.id : null
  );
  const { data: retryHistory } = useRetryHistory(
    expanded ? entry.id : null
  );

  const dismissMutation = useDismissDlq();
  const replayMutation = useReplayDelivery();

  const config = statusConfig[entry.status as "failed" | "dlq"];
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

  const formatTimestampFromUnix = (unixTimestamp: number): string => {
    const date = new Date(unixTimestamp * 1000);
    return formatFullTimestamp(date);
  };

  const handleDismiss = () => {
    dismissMutation.mutate(entry.id);
  };

  const handleReplay = () => {
    setShowReplayDialog(true);
  };

  const handleConfirmReplay = () => {
    replayMutation.mutate(entry.id);
    setShowReplayDialog(false);
  };

  return (
    <>
      <div className={`${config.borderColor}`}>
        {/* Summary row */}
        <div
          onClick={() => setExpanded(!expanded)}
          className="flex items-center gap-2 px-3 py-2 rounded-md cursor-pointer hover:bg-bg-tertiary transition-colors"
        >
          <Icon size={14} className={`${config.color} shrink-0`} />
          <div className="flex-1 min-w-0">
            <div className="flex items-center gap-2">
              <span className="text-xs font-medium truncate">
                {entry.source}
              </span>
              {entry.deliveredTo && (
                <span className="text-[10px] text-text-secondary truncate">
                  → {entry.deliveredTo.endpoint_name}
                </span>
              )}
              {entry.triggerType === "manual" && (
                <span className="px-1.5 py-0.5 rounded text-[9px] font-medium bg-accent/10 text-accent shrink-0">
                  Manual
                </span>
              )}
              <span className={`text-[10px] ${config.color}`}>
                {config.label}
              </span>
            </div>
            {entry.payloadSummary && (
              <div className="text-[10px] text-text-secondary truncate mt-0.5">
                {entry.payloadSummary}
              </div>
            )}
          </div>
          <span className="text-[11px] font-mono text-text-secondary shrink-0">
            {formatTime(entry.timestamp)}
          </span>
          {expanded ? (
            <ChevronDown size={12} className="text-text-secondary shrink-0" />
          ) : (
            <ChevronRight size={12} className="text-text-secondary shrink-0" />
          )}
        </div>

        {/* Expanded detail with structured diagnosis */}
        {expanded && (
          <div className="mx-3 mt-1 mb-2 p-3 bg-bg-primary rounded-md text-xs leading-relaxed">
            {diagnosis ? (
              <>
                {/* What happened */}
                <div className="mb-3">
                  <h4 className="text-[11px] font-semibold text-text-primary mb-1.5">
                    What happened
                  </h4>
                  <p className="text-text-secondary leading-relaxed">
                    {diagnosis.user_message}
                  </p>
                </div>

                {/* What to do */}
                <div className="mb-3">
                  <h4 className="text-[11px] font-semibold text-text-primary mb-1.5">
                    What to do
                  </h4>
                  <p className="text-text-secondary leading-relaxed">
                    {diagnosis.guidance}
                  </p>
                </div>

                {/* What's at risk (if present) */}
                {diagnosis.risk_summary && (
                  <div className="mb-3 p-2 bg-warning-bg border border-warning/20 rounded">
                    <h4 className="text-[11px] font-semibold text-warning mb-1">
                      What's at risk
                    </h4>
                    <p className="text-[11px] text-text-secondary leading-relaxed">
                      {diagnosis.risk_summary}
                    </p>
                  </div>
                )}

                {/* Timeline (if retry history available) */}
                {retryHistory && retryHistory.length > 0 && (
                  <div className="mb-3 border-t border-border pt-3">
                    <h4 className="text-[11px] font-semibold text-text-primary mb-2">
                      Timeline
                    </h4>
                    <div className="space-y-1.5">
                      {retryHistory.map((attempt) => (
                        <div
                          key={attempt.attempt}
                          className="flex items-start gap-2 text-[10px] font-mono"
                        >
                          <span className="text-text-secondary min-w-[140px]">
                            {formatTimestampFromUnix(attempt.at)}
                          </span>
                          <span className="text-text-secondary">
                            {attempt.attempt === 0
                              ? "First attempt"
                              : `Retry ${attempt.attempt}/5`}{" "}
                            — {attempt.error}
                          </span>
                        </div>
                      ))}
                      {entry.status === "dlq" && (
                        <div className="flex items-start gap-2 text-[10px] font-mono">
                          <span className="text-text-secondary min-w-[140px]">
                            {formatFullTimestamp(entry.timestamp)}
                          </span>
                          <span className="text-error font-medium">
                            Moved to dead letter queue
                          </span>
                        </div>
                      )}
                    </div>
                  </div>
                )}
              </>
            ) : (
              // Fallback to raw error if diagnosis not available
              <div className="text-error text-xs">
                <strong>Error:</strong> {entry.error || "Unknown error"}
              </div>
            )}

            {/* Action buttons */}
            <div className="flex items-center gap-3 mt-3 pt-3 border-t border-border">
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  handleReplay();
                }}
                disabled={replayMutation.isPending}
                className="flex items-center gap-1.5 text-[11px] font-medium text-accent hover:underline disabled:opacity-50 disabled:cursor-not-allowed"
              >
                <RotateCcw size={12} />
                {replayMutation.isPending ? "Replaying..." : "Replay"}
              </button>
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  handleDismiss();
                }}
                disabled={dismissMutation.isPending}
                className="flex items-center gap-1.5 text-[11px] font-medium text-text-secondary hover:text-text-primary hover:underline disabled:opacity-50 disabled:cursor-not-allowed"
              >
                <Trash2 size={12} />
                {dismissMutation.isPending ? "Dismissing..." : "Dismiss"}
              </button>
            </div>
          </div>
        )}
      </div>

      {/* Replay confirmation dialog */}
      {showReplayDialog && (
        <ReplayConfirmDialog
          entry={entry}
          onConfirm={handleConfirmReplay}
          onCancel={() => setShowReplayDialog(false)}
        />
      )}
    </>
  );
}
