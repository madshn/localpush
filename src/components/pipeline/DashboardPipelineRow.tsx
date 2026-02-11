import { Pencil, Zap } from "lucide-react";
import { SourceCard } from "./SourceCard";
import { TargetCard } from "./TargetCard";
import { AddTargetCard } from "./AddTargetCard";
import { PipelineConnector } from "./PipelineConnector";
import type { SourceData, SourceCategory, TrafficLightStatus } from "./types";
import type { Binding } from "../../api/hooks/useBindings";

interface DashboardPipelineRowProps {
  source: SourceData;
  category: SourceCategory;
  bindings: Binding[];
  trafficLightStatus: TrafficLightStatus;
  isPushing: boolean;
  onAddTarget: (sourceId: string) => void;
  onEditBinding: (sourceId: string, endpointId: string) => void;
  onPushNow: (sourceId: string) => void;
  onEnableClick: (sourceId: string, isEnabled: boolean) => void;
}

const statusStripe: Record<TrafficLightStatus, string> = {
  green: "bg-success",
  yellow: "bg-warning",
  red: "bg-error",
  grey: "bg-text-secondary/30",
};

function deliveryModeBadge(binding: Binding): string | null {
  if (!binding.delivery_mode || binding.delivery_mode === "on_change")
    return null;
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

export function DashboardPipelineRow({
  source,
  category,
  bindings,
  trafficLightStatus,
  isPushing,
  onAddTarget,
  onEditBinding,
  onPushNow,
  onEnableClick,
}: DashboardPipelineRowProps) {
  const isActive = category === "active" && bindings.length > 0;

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
                      targetType={binding.target_id.split("_")[0] || "n8n"}
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
