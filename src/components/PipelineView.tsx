import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { useSources } from "../api/hooks/useSources";
import {
  useCreateBinding,
  useRemoveBinding,
} from "../api/hooks/useBindings";
import { Plus } from "lucide-react";
import { SummaryStats } from "./SummaryStats";
import { PipelineCard } from "./PipelineCard";
import { SkeletonCard } from "./Skeleton";
import { logger } from "../utils/logger";

interface SourcePreview {
  title: string;
  summary: string;
  fields: Array<{ label: string; value: string; sensitive: boolean }>;
  lastUpdated: string | null;
}

interface DeliveryStatus {
  overall: "success" | "pending" | "failed" | "unknown";
  pending_count: number;
  failed_count: number;
  last_delivery: string | null;
}

type FlowStep =
  | "idle"
  | "preview"
  | "pick_endpoint"
  | "configure_delivery"
  | "security_check";

interface FlowState {
  sourceId: string;
  step: FlowStep;
  preview: SourcePreview | null;
  selectedTarget: string | null;
  selectedEndpoint: string | null;
  selectedEndpointUrl: string | null;
  selectedEndpointName: string | null;
  selectedAuthenticated: boolean;
  selectedAuthType: string | null;
  customHeaders: [string, string][];
  authHeaderName: string;
  authHeaderValue: string;
}

const defaultFlowState = (sourceId: string): FlowState => ({
  sourceId,
  step: "idle",
  preview: null,
  selectedTarget: null,
  selectedEndpoint: null,
  selectedEndpointUrl: null,
  selectedEndpointName: null,
  selectedAuthenticated: false,
  selectedAuthType: null,
  customHeaders: [],
  authHeaderName: "",
  authHeaderValue: "",
});

export function PipelineView() {
  const { data: sources, isLoading } = useSources();
  const queryClient = useQueryClient();
  const createBinding = useCreateBinding();
  const removeBinding = useRemoveBinding();

  const [flowStates, setFlowStates] = useState<Record<string, FlowState>>({});
  const [previewLoading, setPreviewLoading] = useState<
    Record<string, boolean>
  >({});
  const [deliveryStatuses, setDeliveryStatuses] = useState<
    Record<string, DeliveryStatus>
  >({});
  const [pushingSource, setPushingSource] = useState<string | null>(null);

  useEffect(() => {
    loadDeliveryStatus();
    // loadDeliveryStatus is stable — intentionally not in deps
  }, [sources]); // eslint-disable-line react-hooks/exhaustive-deps

  const loadDeliveryStatus = async () => {
    try {
      const status = await invoke<DeliveryStatus>("get_delivery_status");
      if (sources) {
        const statuses: Record<string, DeliveryStatus> = {};
        sources.forEach((source) => {
          statuses[source.id] = status;
        });
        setDeliveryStatuses(statuses);
      }
    } catch (error) {
      logger.error("Failed to load delivery status", { error });
    }
  };

  const getFlowState = (sourceId: string): FlowState =>
    flowStates[sourceId] || defaultFlowState(sourceId);

  const updateFlowState = (sourceId: string, updates: Partial<FlowState>) => {
    setFlowStates((prev) => ({
      ...prev,
      [sourceId]: { ...(prev[sourceId] || defaultFlowState(sourceId)), ...updates },
    }));
  };

  const resetFlowState = (sourceId: string) => {
    setFlowStates((prev) => {
      const next = { ...prev };
      delete next[sourceId];
      return next;
    });
  };

  const handleDisable = async (sourceId: string) => {
    try {
      await invoke("disable_source", { sourceId });
      await queryClient.invalidateQueries({ queryKey: ["sources"] });
      resetFlowState(sourceId);
    } catch (error) {
      logger.error("Failed to disable source", { sourceId, error });
      toast.error(`Failed to disable source: ${error}`);
    }
  };

  const handleEnableClick = async (sourceId: string, isEnabled: boolean) => {
    if (isEnabled) {
      handleDisable(sourceId);
    } else {
      updateFlowState(sourceId, { step: "preview" });
      await loadPreview(sourceId);
    }
  };

  const loadPreview = async (sourceId: string) => {
    setPreviewLoading((prev) => ({ ...prev, [sourceId]: true }));
    try {
      const data = await invoke<SourcePreview>("get_source_preview", {
        sourceId,
      });
      updateFlowState(sourceId, { preview: data });
    } catch (error) {
      logger.error("Failed to load preview", { sourceId, error });
      toast.error(`Failed to load preview: ${error}`);
      resetFlowState(sourceId);
    } finally {
      setPreviewLoading((prev) => ({ ...prev, [sourceId]: false }));
    }
  };

  const handlePreviewEnable = (sourceId: string) => {
    updateFlowState(sourceId, { step: "pick_endpoint" });
  };

  const handlePreviewRefresh = async (sourceId: string) => {
    await loadPreview(sourceId);
  };

  const handleEndpointSelect = (
    sourceId: string,
    targetId: string,
    endpointId: string,
    endpointUrl: string,
    endpointName: string,
    authenticated: boolean,
    authType?: string
  ) => {
    updateFlowState(sourceId, {
      step: "configure_delivery",
      selectedTarget: targetId,
      selectedEndpoint: endpointId,
      selectedEndpointUrl: endpointUrl,
      selectedEndpointName: endpointName,
      selectedAuthenticated: authenticated,
      selectedAuthType: authType || null,
    });
  };

  const handleDeliveryConfigConfirm = (
    sourceId: string,
    customHeaders: [string, string][],
    authHeaderName: string,
    authHeaderValue: string
  ) => {
    updateFlowState(sourceId, {
      step: "security_check",
      customHeaders,
      authHeaderName,
      authHeaderValue,
    });
  };

  const handleBackToDeliveryConfig = (sourceId: string) => {
    updateFlowState(sourceId, { step: "configure_delivery" });
  };

  const handleSecurityConfirm = async (sourceId: string) => {
    const state = getFlowState(sourceId);
    if (
      !state.selectedTarget ||
      !state.selectedEndpoint ||
      !state.selectedEndpointUrl ||
      !state.selectedEndpointName
    ) {
      return;
    }

    try {
      await createBinding.mutateAsync({
        sourceId,
        targetId: state.selectedTarget,
        endpointId: state.selectedEndpoint,
        endpointUrl: state.selectedEndpointUrl,
        endpointName: state.selectedEndpointName,
        customHeaders:
          state.customHeaders.length > 0 ? state.customHeaders : undefined,
        authHeaderName: state.authHeaderName || undefined,
        authHeaderValue: state.authHeaderValue || undefined,
      });
      await invoke("enable_source", { sourceId });
      await queryClient.invalidateQueries({ queryKey: ["sources"] });
      await queryClient.invalidateQueries({
        queryKey: ["bindings", sourceId],
      });
      resetFlowState(sourceId);
      toast.success("Source enabled and connected");
    } catch (error) {
      logger.error("Failed to enable source", { sourceId, error });
      toast.error(`Failed to enable source: ${error}`);
    }
  };

  const handleCancelFlow = (sourceId: string) => {
    resetFlowState(sourceId);
  };

  const handleBackToEndpointPicker = (sourceId: string) => {
    updateFlowState(sourceId, { step: "pick_endpoint" });
  };

  const handleUnbind = async (sourceId: string, endpointId: string) => {
    try {
      await removeBinding.mutateAsync({ sourceId, endpointId });
    } catch (error) {
      logger.error("Failed to remove binding", { sourceId, endpointId, error });
      toast.error(`Failed to remove binding: ${error}`);
    }
  };

  const handlePushNow = async (sourceId: string) => {
    setPushingSource(sourceId);
    try {
      await invoke<string>("trigger_source_push", { sourceId });
      toast.success("Push enqueued — delivery worker will send within 5s");
      setTimeout(() => loadDeliveryStatus(), 1000);
    } catch (error) {
      logger.error("Manual push failed", { sourceId, error });
      toast.error(`Push failed: ${error}`);
    } finally {
      setPushingSource(null);
    }
  };

  const getTrafficLightStatus = (
    sourceId: string,
    enabled: boolean
  ): "green" | "yellow" | "red" | "grey" => {
    if (!enabled) return "grey";
    const status = deliveryStatuses[sourceId];
    if (!status) return "grey";
    if (status.failed_count > 0) return "red";
    if (status.pending_count > 0) return "yellow";
    if (status.overall === "success") return "green";
    return "grey";
  };

  if (isLoading) {
    return (
      <div>
        <SummaryStats />
        <div className="flex items-center justify-between mb-3">
          <h2 className="text-sm font-semibold">Active Pipelines</h2>
        </div>
        <div className="flex flex-col gap-3">
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
      </div>
    );
  }

  return (
    <div>
      <SummaryStats />

      <div className="flex items-center justify-between mb-3">
        <h2 className="text-sm font-semibold">Active Pipelines</h2>
      </div>

      <div className="flex flex-col gap-3">
        {sources.map((source) => (
          <PipelineCard
            key={source.id}
            source={source}
            flowState={getFlowState(source.id)}
            previewLoading={previewLoading[source.id] || false}
            trafficLightStatus={getTrafficLightStatus(
              source.id,
              source.enabled
            )}
            onEnableClick={handleEnableClick}
            onPreviewEnable={handlePreviewEnable}
            onPreviewRefresh={handlePreviewRefresh}
            onEndpointSelect={handleEndpointSelect}
            onDeliveryConfigConfirm={handleDeliveryConfigConfirm}
            onSecurityConfirm={handleSecurityConfirm}
            onCancelFlow={handleCancelFlow}
            onBackToEndpointPicker={handleBackToEndpointPicker}
            onBackToDeliveryConfig={handleBackToDeliveryConfig}
            onUnbind={handleUnbind}
            onPushNow={handlePushNow}
            isPushing={pushingSource === source.id}
          />
        ))}
      </div>
    </div>
  );
}
