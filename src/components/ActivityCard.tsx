import { useState } from "react";
import {
  CheckCircle2,
  Clock,
  ChevronDown,
  ChevronRight,
  RotateCcw,
  Copy,
  Check,
  ExternalLink,
} from "lucide-react";
import { toast } from "sonner";
import { invoke } from "@tauri-apps/api/core";
import { useQueryClient } from "@tanstack/react-query";
import type { ActivityEntry } from "../api/hooks/useActivityLog";
import { logger } from "../utils/logger";
import { openUrl } from "../utils/openUrl";
import { FailedDeliveryCard } from "./FailedDeliveryCard";

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
  const [payloadExpanded, setPayloadExpanded] = useState(false);
  const [copied, setCopied] = useState(false);
  const queryClient = useQueryClient();

  // Route failed/dlq entries to the enhanced FailedDeliveryCard
  if (entry.status === "failed" || entry.status === "dlq") {
    return <FailedDeliveryCard entry={entry} />;
  }

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

  const handleReplay = async () => {
    try {
      await invoke("replay_delivery", {
        eventType: entry.sourceId,
        payload: entry.payload,
      });
      await queryClient.invalidateQueries({ queryKey: ["activityLog"] });
      toast.success("Replay enqueued — will deliver within 5s");
      logger.info("Delivery replayed", { id: entry.id, source: entry.sourceId });
    } catch (error) {
      toast.error(`Replay failed: ${error}`);
      logger.error("Replay failed", { id: entry.id, error });
    }
  };

  const copyPayload = () => {
    const json = JSON.stringify(entry.payload, null, 2);
    const textarea = document.createElement("textarea");
    textarea.value = json;
    textarea.style.position = "fixed";
    textarea.style.opacity = "0";
    document.body.appendChild(textarea);
    textarea.select();
    const ok = document.execCommand("copy");
    document.body.removeChild(textarea);
    if (ok) {
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    }
  };

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

  const payloadJson = payloadExpanded
    ? JSON.stringify(entry.payload, null, 2)
    : "";

  return (
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
              <span
                className={`text-[10px] truncate ${entry.deliveredTo.target_url ? "text-accent hover:underline cursor-pointer" : "text-text-secondary"}`}
                onClick={(e) => {
                  if (entry.deliveredTo?.target_url) {
                    e.stopPropagation();
                    openUrl(entry.deliveredTo.target_url);
                  }
                }}
              >
                → {targetLabel(entry.deliveredTo.target_type)}
                {entry.deliveredTo.target_url && (
                  <ExternalLink size={9} className="inline ml-0.5 -mt-0.5" />
                )}
              </span>
            )}
            {entry.triggerType === "manual" && (
              <span className="px-1.5 py-0.5 rounded text-[9px] font-medium bg-accent/10 text-accent shrink-0">
                Manual
              </span>
            )}
            <span className={`text-[10px] ${config.color}`}>
              {statusLabels[entry.status]}
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

      {/* Expanded detail */}
      {expanded && (
        <div className="mx-3 mt-1 mb-2 p-3 bg-bg-primary rounded-md text-xs leading-relaxed">
          <div className="flex flex-col gap-1 text-text-secondary font-mono mb-3">
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
            {entry.deliveredTo && (
              <div>
                <strong className="text-text-primary">Target:</strong>{" "}
                {entry.deliveredTo.target_url ? (
                  <span
                    className="text-accent hover:underline cursor-pointer"
                    onClick={(e) => {
                      e.stopPropagation();
                      openUrl(entry.deliveredTo!.target_url!);
                    }}
                  >
                    {entry.deliveredTo.endpoint_name} ({entry.deliveredTo.target_type})
                    <ExternalLink size={10} className="inline ml-1 -mt-0.5" />
                  </span>
                ) : (
                  <>{entry.deliveredTo.endpoint_name} ({entry.deliveredTo.target_type})</>
                )}
              </div>
            )}
            {entry.error && (
              <div className="text-error">
                <strong>Error:</strong> {entry.error}
              </div>
            )}
            {entry.retryCount > 0 && (
              <div>
                <strong className="text-text-primary">Retries:</strong>{" "}
                {entry.retryCount}
              </div>
            )}
          </div>

          {/* Payload section */}
          {entry.payload != null && (
            <div className="border-t border-border pt-2 mt-2">
              <div className="flex items-center justify-between mb-1.5">
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    setPayloadExpanded(!payloadExpanded);
                  }}
                  className="text-[11px] font-medium text-accent hover:underline"
                >
                  {payloadExpanded ? "Hide Payload" : "View Payload"}
                </button>
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    copyPayload();
                  }}
                  className="inline-flex items-center gap-1 text-[10px] text-text-secondary hover:text-text-primary transition-colors"
                >
                  {copied ? (
                    <>
                      <Check size={10} className="text-success" /> Copied
                    </>
                  ) : (
                    <>
                      <Copy size={10} /> Copy
                    </>
                  )}
                </button>
              </div>
              {payloadExpanded && (
                <pre className="text-[10px] font-mono text-text-secondary bg-bg-secondary rounded p-2 overflow-x-auto max-h-64 overflow-y-auto whitespace-pre-wrap break-all">
                  {payloadJson}
                </pre>
              )}
            </div>
          )}

          {/* Action buttons */}
          <div className="flex items-center gap-3 mt-3 pt-2 border-t border-border">
            <button
              onClick={(e) => {
                e.stopPropagation();
                handleReplay();
              }}
              className="flex items-center gap-1.5 text-[11px] font-medium text-accent hover:underline"
            >
              <RotateCcw size={12} />
              Replay
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
