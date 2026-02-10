import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { useSources } from "../api/hooks/useSources";
import {
  useAllBindings,
  useCreateBinding,
  useRemoveBinding,
  type Binding,
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
  overall: "active" | "success" | "pending" | "failed" | "unknown";
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

type DeliveryMode = "on_change" | "daily" | "weekly";

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
  isEditing: boolean;
  existingAuthCredentialKey: string | null;
  deliveryMode: DeliveryMode;
  scheduleTime: string | undefined;
  scheduleDay: string | undefined;
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
  isEditing: false,
  existingAuthCredentialKey: null,
  deliveryMode: "on_change",
  scheduleTime: undefined,
  scheduleDay: undefined,
});

type SourceCategory = "active" | "paused" | "available";

interface SourceWithCategory {
  source: {
    id: string;
    name: string;
    description: string;
    enabled: boolean;
    last_sync: string | null;
    watch_path: string | null;
  };
  category: SourceCategory;
}

function categorizeAndSortSources(
  sources: Array<{ id: string; name: string; description: string; enabled: boolean; last_sync: string | null; watch_path: string | null }>,
  allBindings: Binding[] | undefined
): { active: SourceWithCategory[]; paused: SourceWithCategory[]; available: SourceWithCategory[] } {
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
  const { data: sources, isLoading } = useSources();
  const { data: allBindings } = useAllBindings();
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
    logger.debug("Disabling source", { sourceId });
    try {
      await invoke("disable_source", { sourceId });
      await queryClient.invalidateQueries({ queryKey: ["sources"] });
      resetFlowState(sourceId);
      logger.info("Source disabled", { sourceId });
    } catch (error) {
      logger.error("Failed to disable source", { sourceId, error });
      toast.error(`Failed to disable source: ${error}`);
    }
  };

  const handleEnableClick = async (sourceId: string, isEnabled: boolean) => {
    logger.debug("Enable click", { sourceId, isEnabled });
    if (isEnabled) {
      handleDisable(sourceId);
    } else {
      // If source already has bindings, just re-enable — no need for full flow
      const existingBindings = allBindings?.filter((b) => b.source_id === sourceId) || [];
      if (existingBindings.length > 0) {
        try {
          await invoke("enable_source", { sourceId });
          await queryClient.invalidateQueries({ queryKey: ["sources"] });
          toast.success("Source re-enabled");
          logger.info("Source re-enabled with existing bindings", { sourceId, bindingCount: existingBindings.length });
        } catch (error) {
          logger.error("Failed to re-enable source", { sourceId, error });
          toast.error(`Failed to enable source: ${error}`);
        }
      } else {
        updateFlowState(sourceId, { step: "preview" });
        await loadPreview(sourceId);
      }
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
    authHeaderValue: string,
    deliveryMode: DeliveryMode,
    scheduleTime: string | undefined,
    scheduleDay: string | undefined
  ) => {
    updateFlowState(sourceId, {
      step: "security_check",
      customHeaders,
      authHeaderName,
      authHeaderValue,
      deliveryMode,
      scheduleTime,
      scheduleDay,
    });
  };

  const handleBackToDeliveryConfig = (sourceId: string) => {
    updateFlowState(sourceId, { step: "configure_delivery" });
  };

  const handleAddTarget = (sourceId: string) => {
    logger.info("Add Target flow started", { sourceId });
    updateFlowState(sourceId, { step: "pick_endpoint" });
  };

  const handleEditBinding = (sourceId: string, endpointId: string) => {
    logger.info("Edit binding started", { sourceId, endpointId });
    // Find the binding from the allBindings query to get headers_json
    const binding = allBindings?.find(
      (b) => b.source_id === sourceId && b.endpoint_id === endpointId
    );
    if (!binding) {
      logger.error("Binding not found for editing", { sourceId, endpointId });
      return;
    }

    // Parse headers_json back into [string, string][] if present
    let existingHeaders: [string, string][] = [];
    let existingAuthName = "";
    let existingAuthValue = "";
    if (binding.headers_json) {
      try {
        const parsed: [string, string][] = JSON.parse(binding.headers_json);
        // Separate auth header (empty value = credential placeholder) from custom headers
        const authHeader = parsed.find(([, v]) => v === "");
        if (authHeader) {
          existingAuthName = authHeader[0];
          // Auth value is in credential store — user will need to re-enter
        }
        existingHeaders = parsed.filter(([, v]) => v !== "");
      } catch {
        logger.warn("Failed to parse binding headers_json", { sourceId, endpointId });
      }
    }

    updateFlowState(sourceId, {
      step: "configure_delivery",
      selectedTarget: binding.target_id,
      selectedEndpoint: binding.endpoint_id,
      selectedEndpointUrl: binding.endpoint_url,
      selectedEndpointName: binding.endpoint_name,
      selectedAuthenticated: !!binding.auth_credential_key,
      selectedAuthType: binding.auth_credential_key ? "custom" : null,
      customHeaders: existingHeaders,
      authHeaderName: existingAuthName,
      authHeaderValue: existingAuthValue,
      isEditing: true,
      existingAuthCredentialKey: binding.auth_credential_key || null,
      deliveryMode: (binding.delivery_mode || "on_change") as DeliveryMode,
      scheduleTime: binding.schedule_time || undefined,
      scheduleDay: binding.schedule_day || undefined,
    });
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

    const source = sources?.find((s) => s.id === sourceId);
    const alreadyEnabled = source?.enabled ?? false;
    const isEditing = state.isEditing;

    try {
      // If editing and auth value is empty but credential key exists, preserve it
      const preserveKey =
        isEditing && !state.authHeaderValue && state.existingAuthCredentialKey
          ? state.existingAuthCredentialKey
          : undefined;

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
        preserveAuthCredentialKey: preserveKey,
        deliveryMode: state.deliveryMode !== "on_change" ? state.deliveryMode : undefined,
        scheduleTime: state.scheduleTime,
        scheduleDay: state.scheduleDay,
      });
      if (!alreadyEnabled) {
        await invoke("enable_source", { sourceId });
      }
      await queryClient.invalidateQueries({ queryKey: ["sources"] });
      await queryClient.invalidateQueries({
        queryKey: ["bindings", sourceId],
      });
      await queryClient.invalidateQueries({ queryKey: ["bindings"] });
      resetFlowState(sourceId);
      toast.success(
        isEditing
          ? "Binding updated"
          : alreadyEnabled
            ? "Additional target connected"
            : "Source enabled and connected"
      );
      logger.info("Binding saved", { sourceId, isEditing, alreadyEnabled, endpointId: state.selectedEndpoint });
    } catch (error) {
      logger.error("Failed to save binding", { sourceId, error });
      toast.error(`Failed to connect target: ${error}`);
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
    logger.info("Push Now triggered", { sourceId });
    setPushingSource(sourceId);
    try {
      const result = await invoke<string>("trigger_source_push", { sourceId });
      logger.debug("Push enqueued", { sourceId, result });
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
    if (status.overall === "active" || status.overall === "success") return "green";
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

  const { active, paused, available } = categorizeAndSortSources(
    sources,
    allBindings
  );

  const renderCard = ({ source, category }: SourceWithCategory) => (
    <PipelineCard
      key={source.id}
      source={source}
      category={category}
      flowState={getFlowState(source.id)}
      previewLoading={previewLoading[source.id] || false}
      trafficLightStatus={getTrafficLightStatus(source.id, source.enabled)}
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
      onAddTarget={handleAddTarget}
      onEditBinding={handleEditBinding}
      isPushing={pushingSource === source.id}
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
