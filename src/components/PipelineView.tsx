import { useMemo } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { Plus, Loader2 } from "lucide-react";
import { useSources } from "../api/hooks/useSources";
import {
  useAllBindings,
  useCreateBinding,
  useRemoveBinding,
  type Binding,
} from "../api/hooks/useBindings";
import { useTargetHealth } from "../api/hooks/useTargets";
import { SummaryStats } from "./SummaryStats";
import { PipelineCard } from "./PipelineCard";
import { usePipelineFlow } from "./pipeline/usePipelineFlow";
import type {
  SourceData,
  SourceWithCategory,
} from "./pipeline/types";

function categorizeAndSortSources(
  sources: SourceData[],
  allBindings: Binding[] | undefined
): {
  active: SourceWithCategory[];
  paused: SourceWithCategory[];
  available: SourceWithCategory[];
} {
  const bindingsBySource = new Map<string, Binding[]>();
  if (allBindings) {
    for (const binding of allBindings) {
      const existing = bindingsBySource.get(binding.source_id) || [];
      existing.push(binding);
      bindingsBySource.set(binding.source_id, existing);
    }
  }

  const active: SourceWithCategory[] = [];
  const paused: SourceWithCategory[] = [];
  const available: SourceWithCategory[] = [];

  for (const source of sources) {
    const sourceBindings = bindingsBySource.get(source.id) || [];
    if (source.enabled && sourceBindings.length > 0) {
      active.push({ source, category: "active" });
    } else if (source.enabled) {
      paused.push({ source, category: "paused" });
    } else {
      available.push({ source, category: "available" });
    }
  }

  return { active, paused, available };
}

export function PipelineView() {
  const { data: sources, isLoading: sourcesLoading } = useSources();
  const { data: allBindings, isLoading: bindingsLoading } = useAllBindings();
  const { data: targetHealth } = useTargetHealth();
  const queryClient = useQueryClient();
  const createBinding = useCreateBinding();
  const removeBinding = useRemoveBinding();

  const flow = usePipelineFlow({
    sources,
    allBindings,
    queryClient,
    createBinding,
    removeBinding,
  });

  const isLoading = sourcesLoading || bindingsLoading;

  const bindingsBySource = useMemo(() => {
    const map = new Map<string, Binding[]>();
    if (!allBindings) return map;
    for (const binding of allBindings) {
      const existing = map.get(binding.source_id);
      if (existing) {
        existing.push(binding);
      } else {
        map.set(binding.source_id, [binding]);
      }
    }
    return map;
  }, [allBindings]);

  if (isLoading) {
    const step = sourcesLoading ? 0 : 1;
    const steps = ["Loading pipelines", "Checking queues", "Validating connections"];
    return (
      <div className="flex flex-col items-center justify-center py-16 gap-4">
        <Loader2 size={24} className="text-accent animate-spin" />
        <div className="flex flex-col items-center gap-2">
          {steps.map((label, i) => (
            <div key={label} className="flex items-center gap-2">
              <div className={`w-1.5 h-1.5 rounded-full ${
                i < step ? "bg-success" : i === step ? "bg-accent animate-pulse" : "bg-border"
              }`} />
              <span className={`text-xs ${
                i <= step ? "text-text-primary" : "text-text-secondary/50"
              }`}>
                {label}{i === step ? "..." : ""}
              </span>
            </div>
          ))}
        </div>
        {/* Progress bar */}
        <div className="w-48 h-1 bg-bg-tertiary rounded-full overflow-hidden">
          <div
            className="h-full bg-accent rounded-full transition-all duration-500"
            style={{ width: `${((step + 1) / steps.length) * 100}%` }}
          />
        </div>
      </div>
    );
  }

  if (!sources || sources.length === 0) {
    return (
      <div>
        <SummaryStats />
        <div className="text-center py-12">
          <Plus size={32} className="mx-auto mb-3 text-text-secondary/40" />
          <p className="text-sm text-text-secondary mb-1">
            No sources configured
          </p>
          <p className="text-xs text-text-secondary/60">
            Enable your first source to start pushing data.
          </p>
        </div>
      </div>
    );
  }

  const { active, paused, available } = categorizeAndSortSources(
    sources,
    allBindings
  );

  const renderCard = ({ source, category }: SourceWithCategory) => (
    <PipelineCard
      key={source.id}
      source={source}
      category={category}
      bindings={bindingsBySource.get(source.id) || []}
      targetHealth={targetHealth || []}
      flowState={flow.getFlowState(source.id)}
      previewLoading={flow.previewLoading[source.id] || false}
      trafficLightStatus={flow.getTrafficLightStatus(
        source.id,
        source.enabled
      )}
      onEnableClick={flow.handleEnableClick}
      onPreviewEnable={flow.handlePreviewEnable}
      onPreviewRefresh={flow.handlePreviewRefresh}
      onEndpointSelect={flow.handleEndpointSelect}
      onDeliveryConfigConfirm={flow.handleDeliveryConfigConfirm}
      onSecurityConfirm={flow.handleSecurityConfirm}
      onCancelFlow={flow.handleCancelFlow}
      onBackToEndpointPicker={flow.handleBackToEndpointPicker}
      onBackToDeliveryConfig={flow.handleBackToDeliveryConfig}
      onUnbind={flow.handleUnbind}
      onPushNow={flow.handlePushNow}
      onAddTarget={flow.handleAddTarget}
      onEditBinding={flow.handleEditBinding}
      isPushing={flow.pushingSource === source.id}
    />
  );

  return (
    <div>
      <SummaryStats />

      {active.length > 0 && (
        <div className="mb-4">
          <div className="flex items-center gap-2 mb-2">
            <span className="w-2 h-2 rounded-full bg-success" />
            <h2 className="text-xs font-semibold text-text-secondary uppercase tracking-wider">
              Active Pipelines
            </h2>
            <span className="text-[10px] text-text-secondary/60">
              {active.length}
            </span>
          </div>
          <div className="flex flex-col gap-3">{active.map(renderCard)}</div>
        </div>
      )}

      {paused.length > 0 && (
        <div className="mb-4">
          <div className="flex items-center gap-2 mb-2">
            <span className="w-2 h-2 rounded-full bg-text-secondary/40" />
            <h2 className="text-xs font-semibold text-text-secondary uppercase tracking-wider">
              Paused
            </h2>
            <span className="text-[10px] text-text-secondary/60">
              {paused.length}
            </span>
          </div>
          <div className="flex flex-col gap-3">{paused.map(renderCard)}</div>
        </div>
      )}

      {available.length > 0 && (
        <div className="mb-4">
          <div className="flex items-center gap-2 mb-2">
            <h2 className="text-xs font-semibold text-text-secondary uppercase tracking-wider">
              Available Sources
            </h2>
            <span className="text-[10px] text-text-secondary/60">
              {available.length}
            </span>
          </div>
          <div className="flex flex-col gap-3">
            {available.map(renderCard)}
          </div>
        </div>
      )}

      {active.length === 0 && paused.length === 0 && available.length === 0 && (
        <div className="text-center py-12">
          <Plus size={32} className="mx-auto mb-3 text-text-secondary/40" />
          <p className="text-sm text-text-secondary mb-1">
            No sources configured
          </p>
          <p className="text-xs text-text-secondary/60">
            Enable your first source to start pushing data.
          </p>
        </div>
      )}
    </div>
  );
}
