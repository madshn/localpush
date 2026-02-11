import { useQueryClient } from "@tanstack/react-query";
import { Plus } from "lucide-react";
import { useSources } from "../api/hooks/useSources";
import {
  useAllBindings,
  useCreateBinding,
  useRemoveBinding,
  type Binding,
} from "../api/hooks/useBindings";
import { SummaryStats } from "./SummaryStats";
import { SkeletonCard } from "./Skeleton";
import { PipelineRow } from "./pipeline/PipelineRow";
import { FlowModal } from "./pipeline/FlowModal";
import { VelocityChart } from "./pipeline/VelocityChart";
import { ActivityLogPreview } from "./pipeline/ActivityLogPreview";
import { usePipelineFlow } from "./pipeline/usePipelineFlow";
import type {
  SourceData,
  SourceCategory,
  SourceWithCategory,
} from "./pipeline/types";

interface PipelineViewProps {
  onViewAllActivity?: () => void;
}

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

function buildActiveRows(
  activeSources: SourceWithCategory[],
  allBindings: Binding[] | undefined
): Array<{ source: SourceData; category: SourceCategory; binding: Binding }> {
  const rows: Array<{
    source: SourceData;
    category: SourceCategory;
    binding: Binding;
  }> = [];
  for (const { source, category } of activeSources) {
    const bindings =
      allBindings?.filter((b) => b.source_id === source.id) || [];
    for (const binding of bindings) {
      rows.push({ source, category, binding });
    }
  }
  return rows;
}

export function PipelineView({ onViewAllActivity }: PipelineViewProps) {
  const { data: sources, isLoading } = useSources();
  const { data: allBindings } = useAllBindings();
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

  // Find the active flow (if any source has a non-idle step)
  const activeFlowSourceId = sources?.find(
    (s) => flow.getFlowState(s.id).step !== "idle"
  )?.id;
  const activeFlowState = activeFlowSourceId
    ? flow.getFlowState(activeFlowSourceId)
    : null;

  if (isLoading) {
    return (
      <div>
        <SummaryStats />
        <div className="flex items-center justify-between mb-3">
          <h2 className="text-sm font-semibold">Active Pipelines</h2>
        </div>
        <div className="flex flex-col gap-2">
          <SkeletonCard />
          <SkeletonCard />
          <SkeletonCard />
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

  const activeRows = buildActiveRows(active, allBindings);
  const unboundSources = [...paused, ...available];

  return (
    <div>
      <SummaryStats />

      {/* Active Pipelines */}
      {activeRows.length > 0 && (
        <div className="mb-4">
          <div className="flex items-center gap-2 mb-2">
            <span className="w-2 h-2 rounded-full bg-success" />
            <h2 className="text-xs font-semibold text-text-secondary uppercase tracking-wider">
              Active Pipelines
            </h2>
            <span className="text-[10px] text-text-secondary/60">
              {activeRows.length}
            </span>
          </div>
          <div className="flex flex-col gap-2">
            {activeRows.map((row) => (
              <PipelineRow
                key={`${row.source.id}-${row.binding.endpoint_id}`}
                source={row.source}
                category={row.category}
                binding={row.binding}
                trafficLightStatus={flow.getTrafficLightStatus(
                  row.source.id,
                  row.source.enabled
                )}
                isPushing={flow.pushingSource === row.source.id}
                onAddTarget={flow.handleAddTarget}
                onEditBinding={flow.handleEditBinding}
                onPushNow={flow.handlePushNow}
                onEnableClick={flow.handleEnableClick}
              />
            ))}
          </div>
        </div>
      )}

      {/* Unbound Sources (paused + available) */}
      {unboundSources.length > 0 && (
        <div className="mb-4">
          <div className="flex items-center gap-2 mb-2">
            <h2 className="text-xs font-semibold text-text-secondary uppercase tracking-wider">
              {paused.length > 0 && available.length > 0
                ? "Unbound Sources"
                : paused.length > 0
                  ? "Paused"
                  : "Available Sources"}
            </h2>
            <span className="text-[10px] text-text-secondary/60">
              {unboundSources.length}
            </span>
          </div>
          <div className="flex flex-col gap-2">
            {unboundSources.map(({ source, category }) => (
              <PipelineRow
                key={source.id}
                source={source}
                category={category}
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

      {/* Empty state */}
      {activeRows.length === 0 && unboundSources.length === 0 && (
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

      {/* Velocity Chart */}
      <div className="mb-4">
        <VelocityChart />
      </div>

      {/* Activity Log Preview */}
      <div className="mb-4">
        <ActivityLogPreview onViewAll={onViewAllActivity ?? (() => {})} />
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
