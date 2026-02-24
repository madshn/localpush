import { useState } from "react";
import { Eye, EyeOff } from "lucide-react";

interface PreviewField {
  label: string;
  value: string;
  sensitive: boolean;
}

interface SourcePreview {
  title: string;
  summary: string;
  fields: PreviewField[];
  lastUpdated: string | null;
}

interface TransparencyPreviewProps {
  sourceId: string;
  preview: SourcePreview;
  onEnable: () => void;
  onRefresh: () => void;
  isLoading?: boolean;
}

export function TransparencyPreview({
  preview,
  onEnable,
  onRefresh,
  isLoading = false,
}: TransparencyPreviewProps) {
  const [acknowledged, setAcknowledged] = useState(false);
  const [revealedFields, setRevealedFields] = useState<Set<string>>(new Set());

  const toggleFieldReveal = (label: string) => {
    setRevealedFields((prev) => {
      const next = new Set(prev);
      if (next.has(label)) {
        next.delete(label);
      } else {
        next.add(label);
      }
      return next;
    });
  };

  const maskValue = (value: string): string => {
    if (value.length <= 8) return "\u2022".repeat(value.length);
    // Cap bullets at 12 so long values (paths, titles) don't overflow the container
    return value.slice(0, 4) + "\u2022".repeat(Math.min(value.length - 8, 12)) + value.slice(-4);
  };

  const parseTrend = (
    summary: string
  ): { metric: string; trend: string; direction: "up" | "down" | null } | null => {
    const match = summary.match(/([+-]\d+%)\s*(\u2191|\u2193)?/);
    if (!match) return null;
    const trend = match[1];
    const isPositive = trend.startsWith("+");
    return {
      metric: summary.split(/[+-]\d+%/)[0].trim(),
      trend,
      direction: isPositive ? "up" : "down",
    };
  };

  const trendInfo = parseTrend(preview.summary);

  return (
    <div className="bg-bg-secondary border border-border rounded-lg p-4">
      {/* Header */}
      <div className="mb-3 pb-3 border-b border-border">
        <div className="flex items-center gap-2 mb-1">
          <h3 className="text-sm font-semibold">{preview.title}</h3>
          <span className="px-2 py-0.5 rounded text-[10px] font-semibold uppercase tracking-wide bg-accent text-white">
            Your real data
          </span>
        </div>
        {preview.lastUpdated && (
          <p className="text-xs text-text-secondary">
            Last updated:{" "}
            {new Date(preview.lastUpdated).toLocaleString("en-US", {
              month: "short",
              day: "numeric",
              hour: "numeric",
              minute: "2-digit",
            })}
          </p>
        )}
      </div>

      {/* Summary */}
      <div className="flex items-baseline gap-3 px-3 py-3 bg-bg-primary rounded-md mb-3">
        <span className="text-xl font-semibold">
          {trendInfo ? trendInfo.metric : preview.summary}
        </span>
        {trendInfo && (
          <span
            className={`text-sm font-semibold px-2 py-0.5 rounded ${
              trendInfo.direction === "up"
                ? "text-success bg-success-bg"
                : "text-error bg-error-bg"
            }`}
          >
            {trendInfo.trend} {trendInfo.direction === "up" ? "\u2191" : "\u2193"}
          </span>
        )}
      </div>

      {/* Fields */}
      <div className="flex flex-col gap-2 mb-3">
        {preview.fields.map((field, index) => (
          <div
            key={index}
            className="flex items-center justify-between py-1.5 gap-3"
          >
            <span className="text-xs text-text-secondary min-w-[100px]">
              {field.label}
            </span>
            <div className="flex items-center gap-1.5 flex-1 justify-end">
              {field.sensitive && !revealedFields.has(field.label) ? (
                <>
                  <span className="text-xs text-text-secondary tracking-wider font-mono">
                    {maskValue(field.value)}
                  </span>
                  <button
                    className="p-0.5 opacity-60 hover:opacity-100 transition-opacity"
                    onClick={() => toggleFieldReveal(field.label)}
                    aria-label={`Reveal ${field.label}`}
                  >
                    <Eye size={14} className="text-text-secondary" />
                  </button>
                </>
              ) : (
                <>
                  <span className="text-xs font-mono text-right break-all">
                    {field.value}
                  </span>
                  {field.sensitive && (
                    <button
                      className="p-0.5 opacity-60 hover:opacity-100 transition-opacity"
                      onClick={() => toggleFieldReveal(field.label)}
                      aria-label={`Hide ${field.label}`}
                    >
                      <EyeOff size={14} className="text-text-secondary" />
                    </button>
                  )}
                </>
              )}
            </div>
          </div>
        ))}
      </div>

      {/* Acknowledgment */}
      <div className="p-3 bg-bg-primary rounded-md mb-3">
        <label className="flex items-center gap-2 text-xs text-text-secondary cursor-pointer select-none">
          <input
            type="checkbox"
            checked={acknowledged}
            onChange={(e) => setAcknowledged(e.target.checked)}
            className="w-4 h-4 rounded cursor-pointer accent-accent"
          />
          <span>I understand this data will be sent to my configured webhooks</span>
        </label>
      </div>

      {/* Actions */}
      <div className="flex gap-2 justify-end">
        <button
          className="text-xs font-medium px-3 py-1.5 rounded-md bg-bg-tertiary text-text-secondary border border-border hover:border-border-hover transition-colors disabled:opacity-50"
          onClick={onRefresh}
          disabled={isLoading}
        >
          {isLoading ? "Refreshing..." : "Refresh Preview"}
        </button>
        <button
          className="text-xs font-medium px-3 py-1.5 rounded-md bg-accent text-white hover:bg-accent/90 transition-colors disabled:opacity-50"
          onClick={onEnable}
          disabled={!acknowledged || isLoading}
        >
          Enable Source
        </button>
      </div>
    </div>
  );
}
