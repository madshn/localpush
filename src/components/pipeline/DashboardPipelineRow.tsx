import { memo } from "react";
import { Pencil, Zap, AlertTriangle } from "lucide-react";
import { SourceCard } from "./SourceCard";
import { TargetCard } from "./TargetCard";
import { AddTargetCard } from "./AddTargetCard";
import { PipelineConnector } from "./PipelineConnector";
import type { SourceData, SourceCategory, TrafficLightStatus } from "./types";
import type { Binding } from "../../api/hooks/useBindings";
import type { TimelineGap } from "../../api/hooks/useTimelineGaps";

interface DashboardPipelineRowProps {
  source: SourceData;
  category: SourceCategory;
  bindings: Binding[];
  trafficLightStatus: TrafficLightStatus;
  isPushing: boolean;
  gap: TimelineGap | null;
  onAddTarget: (sourceId: string) => void;
  onEditBinding: (sourceId: string, endpointId: string) => void;
  onPushNow: (sourceId: string) => void;
  onEnableClick: (sourceId: string, isEnabled: boolean) => void;
  onViewActivity?: () => void;
}

const statusStripe: Record<TrafficLightStatus, string> = {
  green: "bg-success",
  yellow: "bg-warning",
  red: "bg-error",
  grey: "bg-text-secondary/30",
};

function deliveryModeBadge(binding: Binding): string | null {
  if (!binding.delivery_mode || binding.delivery_mode === "on_change")
    return "Real-time";
  if (binding.delivery_mode === "interval") {
    const mins = binding.schedule_time || "15";
    return `Every ${mins}m`;
  }
  if (binding.delivery_mode === "daily") {
    return `Daily ${binding.schedule_time || "00:01"}`;
  }
  if (binding.delivery_mode === "weekly") {
    const day = binding.schedule_day
      ? binding.schedule_day.charAt(0).toUpperCase() +
        binding.schedule_day.slice(1, 3)
      : "Mon";
    return `Weekly ${day} ${binding.schedule_time || "00:01"}`;
  }
  return null;
}

function DashboardPipelineRowComponent({
  source,
  category,
  bindings,
  trafficLightStatus,
  isPushing,
  gap,
  onAddTarget,
  onEditBinding,
  onPushNow,
  onEnableClick,
  onViewActivity,
}: DashboardPipelineRowProps) {
  const isActive = category === "active" && bindings.length > 0;

  const formatGapDate = (isoDate: string): string => {
    const date = new Date(isoDate);
    return date.toLocaleDateString("en-US", {
      month: "short",
      day: "numeric",
    });
  };

  return (
    <div className="relative bg-bg-secondary border border-border rounded-lg overflow-hidden">
      {/* Left colored stripe */}
      <div
        className={`absolute left-0 top-0 bottom-0 w-1 ${statusStripe[trafficLightStatus]}`}
      />

      <div className="pl-4 pr-3 py-2.5">
        <div className="grid grid-cols-[1fr_auto_1fr] gap-3 items-center">
          {/* Source */}
          <SourceCard
            sourceId={source.id}
            name={source.name}
            category={category}
          />

          {/* Connector (fan-out) */}
          <PipelineConnector
            active={isActive}
            targetCount={bindings.length > 0 ? bindings.length : 1}
          />

          {/* Targets (stacked) or placeholder */}
          {bindings.length > 0 ? (
            <div className="flex flex-col gap-1.5">
              {bindings.map((binding) => (
                <div key={binding.endpoint_id} className="flex items-center gap-1">
                  <div className="flex-1 min-w-0">
                    <TargetCard
                      targetType={binding.target_id.split("-")[0] || "n8n"}
                      endpointName={binding.endpoint_name}
                      endpointUrl={binding.endpoint_url}
                    />
                  </div>
                  <button
                    className="p-1 text-text-secondary hover:text-accent transition-colors rounded hover:bg-bg-tertiary shrink-0"
                    onClick={() => onEditBinding(source.id, binding.endpoint_id)}
                    title="Edit binding"
                  >
                    <Pencil size={11} />
                  </button>
                </div>
              ))}
            </div>
          ) : (
            <AddTargetCard onClick={() => onAddTarget(source.id)} />
          )}
        </div>

        {/* Timeline gap warning */}
        {gap && (
          <div className="mt-2 px-2 py-1.5 bg-warning-bg border border-warning/20 rounded">
            <div className="flex items-start gap-1.5">
              <AlertTriangle size={12} className="text-warning mt-0.5 shrink-0" />
              <div className="flex-1 min-w-0">
                <p className="text-[10px] font-medium text-warning mb-0.5">
                  Missing: {gap.delivery_mode} delivery for{" "}
                  {formatGapDate(gap.expected_at)}
                </p>
                <p className="text-[9px] text-text-secondary">
                  Expected at {new Date(gap.expected_at).toLocaleTimeString("en-US", { hour12: false, hour: "2-digit", minute: "2-digit" })}
                  {gap.last_delivered_at && (
                    <>
                      {" "}
                      — last delivered{" "}
                      {formatGapDate(gap.last_delivered_at)}
                    </>
                  )}
                </p>
              </div>
              {onViewActivity && (
                <button
                  onClick={onViewActivity}
                  className="text-[9px] font-medium text-warning hover:underline shrink-0"
                >
                  View
                </button>
              )}
            </div>
          </div>
        )}

        {/* Action row */}
        <div className="flex items-center justify-between mt-1.5 pl-1">
          <div className="flex items-center gap-2">
            {bindings.length > 0 && bindings.some((b) => deliveryModeBadge(b)) && (
              <span className="inline-flex items-center px-1.5 py-0.5 rounded text-[9px] font-medium bg-accent/10 text-accent">
                {deliveryModeBadge(bindings.find((b) => deliveryModeBadge(b))!)}
              </span>
            )}
            {isActive && (
              <span className="inline-flex items-center gap-1 text-[9px] font-medium text-accent/70">
                <Zap size={8} />
                Event-driven
              </span>
            )}
            {!source.enabled && (
              <button
                onClick={() => onEnableClick(source.id, false)}
                className="text-[10px] font-medium text-accent hover:underline"
              >
                Enable
              </button>
            )}
          </div>

          <div className="flex items-center gap-1.5">
            {isActive && (
              <button
                className="text-[10px] font-medium px-2.5 py-1 rounded-md bg-accent text-white hover:bg-accent/90 transition-colors disabled:opacity-50"
                onClick={() => onPushNow(source.id)}
                disabled={isPushing}
              >
                {isPushing ? "Pushing..." : "Push Now"}
              </button>
            )}
            {bindings.length > 0 && (
              <button
                className="text-[10px] font-medium text-text-secondary hover:text-accent transition-colors"
                onClick={() => onAddTarget(source.id)}
              >
                + Add
              </button>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}

function areEqual(
  prev: DashboardPipelineRowProps,
  next: DashboardPipelineRowProps
) {
  return (
    prev.source === next.source &&
    prev.category === next.category &&
    prev.bindings === next.bindings &&
    prev.trafficLightStatus === next.trafficLightStatus &&
    prev.isPushing === next.isPushing &&
    prev.gap === next.gap
  );
}

export const DashboardPipelineRow = memo(DashboardPipelineRowComponent, areEqual);
