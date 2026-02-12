import { X, AlertTriangle } from "lucide-react";
import type { ActivityEntry } from "../api/hooks/useActivityLog";

interface ReplayConfirmDialogProps {
  entry: ActivityEntry;
  onConfirm: () => void;
  onCancel: () => void;
}

export function ReplayConfirmDialog({
  entry,
  onConfirm,
  onCancel,
}: ReplayConfirmDialogProps) {
  const summarizePayload = (payload: unknown): string => {
    if (!payload || typeof payload !== "object") return "No data";
    const obj = payload as Record<string, unknown>;
    const keys = Object.keys(obj);
    if (keys.length === 0) return "Empty payload";

    // Show payload size summary
    const entries = Object.entries(obj);
    const summary: string[] = [];

    // Count array entries
    const arrayFields = entries.filter(([_, v]) => Array.isArray(v));
    if (arrayFields.length > 0) {
      arrayFields.forEach(([k, v]) => {
        summary.push(`${k}: ${(v as unknown[]).length} items`);
      });
    }

    // Show other field types
    const otherFields = entries.filter(([_, v]) => !Array.isArray(v));
    if (otherFields.length > 0 && summary.length < 3) {
      otherFields.slice(0, 3 - summary.length).forEach(([k, v]) => {
        if (typeof v === "string") {
          summary.push(`${k}: "${v.slice(0, 30)}${v.length > 30 ? "..." : ""}"`);
        } else if (typeof v === "number") {
          summary.push(`${k}: ${v}`);
        } else {
          summary.push(`${k}: ${typeof v}`);
        }
      });
    }

    const extra = keys.length > summary.length ? ` +${keys.length - summary.length} more fields` : "";
    return summary.join(", ") + extra;
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-sm">
      <div className="bg-bg-secondary border border-border rounded-lg shadow-2xl max-w-md w-full mx-4 overflow-hidden">
        {/* Header */}
        <div className="flex items-center justify-between px-4 py-3 border-b border-border">
          <h3 className="text-sm font-semibold text-text-primary">
            Replay delivery for {entry.source}?
          </h3>
          <button
            onClick={onCancel}
            className="text-text-secondary hover:text-text-primary transition-colors"
          >
            <X size={16} />
          </button>
        </div>

        {/* Body */}
        <div className="p-4 space-y-3">
          <p className="text-xs text-text-secondary leading-relaxed">
            This will re-send the original payload that failed. A new delivery
            will be queued and sent within 5 seconds.
          </p>

          <div className="bg-bg-tertiary border border-border rounded p-3 space-y-2">
            <div className="text-[11px]">
              <span className="text-text-secondary">Payload:</span>{" "}
              <span className="text-text-primary font-mono">
                {summarizePayload(entry.payload)}
              </span>
            </div>
            <div className="text-[11px]">
              <span className="text-text-secondary">Source:</span>{" "}
              <span className="text-text-primary">{entry.source}</span>
            </div>
            <div className="text-[11px]">
              <span className="text-text-secondary">Original attempt:</span>{" "}
              <span className="text-text-primary">
                {entry.timestamp.toLocaleString("en-US", {
                  month: "short",
                  day: "numeric",
                  hour: "2-digit",
                  minute: "2-digit",
                  hour12: false,
                })}
              </span>
            </div>
          </div>

          {/* Warning */}
          <div className="flex items-start gap-2 p-3 bg-warning-bg border border-warning/20 rounded">
            <AlertTriangle size={14} className="text-warning shrink-0 mt-0.5" />
            <p className="text-[11px] text-text-secondary leading-relaxed">
              Make sure authentication is fixed before replaying, or it will
              fail again. Check your API keys in Settings → Targets.
            </p>
          </div>
        </div>

        {/* Footer */}
        <div className="flex items-center justify-end gap-2 px-4 py-3 border-t border-border bg-bg-primary">
          <button
            onClick={onCancel}
            className="px-3 py-1.5 text-xs font-medium text-text-secondary hover:text-text-primary transition-colors"
          >
            Cancel
          </button>
          <button
            onClick={onConfirm}
            className="px-3 py-1.5 text-xs font-medium bg-accent text-white rounded hover:bg-accent/90 transition-colors"
          >
            Replay Now
          </button>
        </div>
      </div>
    </div>
  );
}
