import { useQueryClient } from "@tanstack/react-query";
import { Plus, Settings } from "lucide-react";
import { useDeliveryStatus } from "../api/hooks/useDeliveryStatus";
import { useSources } from "../api/hooks/useSources";
import {
  useAllBindings,
  useCreateBinding,
  useRemoveBinding,
  type Binding,
} from "../api/hooks/useBindings";
import { useTimelineGaps } from "../api/hooks/useTimelineGaps";
import { StatusIndicator } from "./StatusIndicator";
import { SummaryStats } from "./SummaryStats";
import { SkeletonCard } from "./Skeleton";
import { DashboardPipelineRow } from "./pipeline/DashboardPipelineRow";
import { FlowModal } from "./pipeline/FlowModal";
import { VelocityChart } from "./pipeline/VelocityChart";
import { ActivityLogPreview } from "./pipeline/ActivityLogPreview";
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

function getBindingsForSource(
  sourceId: string,
  allBindings: Binding[] | undefined
): Binding[] {
  return allBindings?.filter((b) => b.source_id === sourceId) || [];
}

function getGapForSource(
  sourceId: string,
  gaps: import("../api/hooks/useTimelineGaps").TimelineGap[] | undefined
): import("../api/hooks/useTimelineGaps").TimelineGap | null {
  return gaps?.find((g) => g.source_id === sourceId) || null;
}

export function DashboardView() {
  const { data: status } = useDeliveryStatus();
  const { data: sources, isLoading } = useSources();
  const { data: allBindings } = useAllBindings();
  const { data: gaps } = useTimelineGaps();
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

  const activeFlowSourceId = sources?.find(
    (s) => flow.getFlowState(s.id).step !== "idle"
  )?.id;
  const activeFlowState = activeFlowSourceId
    ? flow.getFlowState(activeFlowSourceId)
    : null;

  const categorized =
    sources && sources.length > 0
      ? categorizeAndSortSources(sources, allBindings)
      : null;

  const unboundSources = categorized
    ? [...categorized.paused, ...categorized.available]
    : [];

  return (
    <div className="min-h-screen flex flex-col bg-bg-primary">
      {/* Header */}
      <header className="flex items-center justify-between px-6 py-3 border-b border-border bg-bg-secondary">
        <div className="flex items-center gap-3">
          <h1 className="text-base font-semibold tracking-tight">LocalPush</h1>
          <StatusIndicator status={status?.overall ?? "unknown"} />
        </div>
        <button
          className="p-2 text-text-secondary hover:text-text-primary transition-colors rounded-md hover:bg-bg-tertiary"
          title="Settings"
        >
          <Settings size={16} />
        </button>
      </header>

      {/* Scrollable content */}
      <div className="flex-1 overflow-y-auto">
        <div className="max-w-4xl mx-auto px-6 py-6 space-y-6">
          {/* Summary Stats */}
          <SummaryStats />

          {/* Active Pipelines */}
          {isLoading ? (
            <div>
              <div className="flex items-center gap-2 mb-3">
                <span className="w-2 h-2 rounded-full bg-success" />
                <h2 className="text-xs font-semibold text-text-secondary uppercase tracking-wider">
                  Active Pipelines
                </h2>
              </div>
              <div className="flex flex-col gap-2">
                <SkeletonCard />
                <SkeletonCard />
              </div>
            </div>
          ) : categorized && categorized.active.length > 0 ? (
            <div>
              <div className="flex items-center gap-2 mb-3">
                <span className="w-2 h-2 rounded-full bg-success" />
                <h2 className="text-xs font-semibold text-text-secondary uppercase tracking-wider">
                  Active Pipelines
                </h2>
                <span className="text-[10px] text-text-secondary/60">
                  {categorized.active.length}
                </span>
              </div>
              <div className="flex flex-col gap-2">
                {categorized.active.map(({ source, category }) => (
                  <DashboardPipelineRow
                    key={source.id}
                    source={source}
                    category={category}
                    bindings={getBindingsForSource(source.id, allBindings)}
                    gap={getGapForSource(source.id, gaps)}
                    trafficLightStatus={flow.getTrafficLightStatus(
                      source.id,
                      source.enabled
                    )}
                    isPushing={flow.pushingSource === source.id}
                    onAddTarget={flow.handleAddTarget}
                    onEditBinding={flow.handleEditBinding}
                    onPushNow={flow.handlePushNow}
                    onEnableClick={flow.handleEnableClick}
                  />
                ))}
              </div>
            </div>
          ) : !isLoading && (!sources || sources.length === 0) ? (
            <div className="text-center py-12">
              <Plus
                size={32}
                className="mx-auto mb-3 text-text-secondary/40"
              />
              <p className="text-sm text-text-secondary mb-1">
                No sources configured
              </p>
              <p className="text-xs text-text-secondary/60">
                Enable your first source to start pushing data.
              </p>
            </div>
          ) : null}

          {/* Unbound Sources */}
          {unboundSources.length > 0 && (
            <div>
              <div className="flex items-center gap-2 mb-3">
                <h2 className="text-xs font-semibold text-text-secondary uppercase tracking-wider">
                  {categorized!.paused.length > 0 &&
                  categorized!.available.length > 0
                    ? "Unbound Sources"
                    : categorized!.paused.length > 0
                      ? "Paused"
                      : "Available Sources"}
                </h2>
                <span className="text-[10px] text-text-secondary/60">
                  {unboundSources.length}
                </span>
              </div>
              <div className="flex flex-col gap-2">
                {unboundSources.map(({ source, category }) => (
                  <DashboardPipelineRow
                    key={source.id}
                    source={source}
                    category={category}
                    bindings={[]}
                    gap={null}
                    trafficLightStatus="grey"
                    isPushing={false}
                    onAddTarget={flow.handleAddTarget}
                    onEditBinding={flow.handleEditBinding}
                    onPushNow={flow.handlePushNow}
                    onEnableClick={flow.handleEnableClick}
                  />
                ))}
              </div>
            </div>
          )}

          {/* Velocity Chart */}
          <VelocityChart />

          {/* Activity Log Preview */}
          <ActivityLogPreview onViewAll={() => {}} />
        </div>
      </div>

      {/* Flow Modal */}
      {activeFlowState && (
        <FlowModal
          flowState={activeFlowState}
          previewLoading={
            flow.previewLoading[activeFlowState.sourceId] || false
          }
          onPreviewEnable={flow.handlePreviewEnable}
          onPreviewRefresh={flow.handlePreviewRefresh}
          onEndpointSelect={flow.handleEndpointSelect}
          onDeliveryConfigConfirm={flow.handleDeliveryConfigConfirm}
          onSecurityConfirm={flow.handleSecurityConfirm}
          onCancelFlow={flow.handleCancelFlow}
          onBackToEndpointPicker={flow.handleBackToEndpointPicker}
          onBackToDeliveryConfig={flow.handleBackToDeliveryConfig}
          onUnbind={flow.handleUnbind}
        />
      )}
    </div>
  );
}
