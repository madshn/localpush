import { useState } from "react";

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
    if (value.length <= 8) return "•".repeat(value.length);
    return value.slice(0, 4) + "•".repeat(value.length - 8) + value.slice(-4);
  };

  const parseTrend = (
    summary: string
  ): { metric: string; trend: string; direction: "up" | "down" | null } | null => {
    const match = summary.match(/([+-]\d+%)\s*(↑|↓)?/);
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
    <div className="transparency-preview card">
      {/* Header */}
      <div className="preview-header">
        <div className="preview-title">
          <h3>{preview.title}</h3>
          <span className="badge badge-highlight">This is YOUR real data</span>
        </div>
        {preview.lastUpdated && (
          <p className="preview-timestamp">
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
      <div className="preview-summary">
        <div className="summary-metric">
          {trendInfo ? trendInfo.metric : preview.summary}
        </div>
        {trendInfo && (
          <div className={`summary-trend trend-${trendInfo.direction}`}>
            {trendInfo.trend} {trendInfo.direction === "up" ? "↑" : "↓"}
          </div>
        )}
      </div>

      {/* Fields */}
      <div className="preview-fields">
        {preview.fields.map((field, index) => (
          <div key={index} className="preview-field">
            <span className="field-label">{field.label}</span>
            <div className="field-value-container">
              {field.sensitive && !revealedFields.has(field.label) ? (
                <>
                  <span className="field-value field-value-masked">
                    {maskValue(field.value)}
                  </span>
                  <button
                    className="btn-reveal"
                    onClick={() => toggleFieldReveal(field.label)}
                    aria-label={`Reveal ${field.label}`}
                  >
                    👁️
                  </button>
                </>
              ) : (
                <>
                  <span className="field-value">{field.value}</span>
                  {field.sensitive && (
                    <button
                      className="btn-reveal"
                      onClick={() => toggleFieldReveal(field.label)}
                      aria-label={`Hide ${field.label}`}
                    >
                      🙈
                    </button>
                  )}
                </>
              )}
            </div>
          </div>
        ))}
      </div>

      {/* Acknowledgment */}
      <div className="preview-acknowledgment">
        <label className="checkbox-label">
          <input
            type="checkbox"
            checked={acknowledged}
            onChange={(e) => setAcknowledged(e.target.checked)}
          />
          <span>I understand this data will be sent to my configured webhooks</span>
        </label>
      </div>

      {/* Actions */}
      <div className="preview-actions">
        <button
          className="btn btn-secondary"
          onClick={onRefresh}
          disabled={isLoading}
        >
          {isLoading ? "Refreshing..." : "Refresh Preview"}
        </button>
        <button
          className="btn"
          onClick={onEnable}
          disabled={!acknowledged || isLoading}
        >
          Enable Source
        </button>
      </div>
    </div>
  );
}
