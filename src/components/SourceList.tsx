import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useQueryClient } from "@tanstack/react-query";
import { useSources } from "../api/hooks/useSources";
import { useBindings, useCreateBinding, useRemoveBinding } from "../api/hooks/useBindings";
import { TransparencyPreview } from "./TransparencyPreview";
import { EndpointPicker } from "./EndpointPicker";
import { SecurityCoaching } from "./SecurityCoaching";
import { TrafficLight } from "./TrafficLight";
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

type FlowStep = "idle" | "preview" | "pick_endpoint" | "security_check";

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
}

export function SourceList() {
  const { data: sources, isLoading } = useSources();
  const queryClient = useQueryClient();
  const createBinding = useCreateBinding();
  const removeBinding = useRemoveBinding();

  const [flowStates, setFlowStates] = useState<Record<string, FlowState>>({});
  const [previewLoading, setPreviewLoading] = useState<Record<string, boolean>>({});
  const [deliveryStatuses, setDeliveryStatuses] = useState<Record<string, DeliveryStatus>>({});

  useEffect(() => {
    loadDeliveryStatus();
  }, [sources]);

  const loadDeliveryStatus = async () => {
    try {
      const status = await invoke<DeliveryStatus>("get_delivery_status");
      logger.debug("Delivery status loaded", { overall: status.overall });
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

  const getFlowState = (sourceId: string): FlowState => {
    return (
      flowStates[sourceId] || {
        sourceId,
        step: "idle",
        preview: null,
        selectedTarget: null,
        selectedEndpoint: null,
        selectedEndpointUrl: null,
        selectedEndpointName: null,
        selectedAuthenticated: false,
        selectedAuthType: null,
      }
    );
  };

  const updateFlowState = (sourceId: string, updates: Partial<FlowState>) => {
    setFlowStates((prev) => {
      const current = prev[sourceId] || {
        sourceId,
        step: "idle" as FlowStep,
        preview: null,
        selectedTarget: null,
        selectedEndpoint: null,
        selectedEndpointUrl: null,
        selectedEndpointName: null,
        selectedAuthenticated: false,
        selectedAuthType: null,
      };
      return {
        ...prev,
        [sourceId]: { ...current, ...updates },
      };
    });
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
      logger.debug("Disabling source", { sourceId });
      await invoke("disable_source", { sourceId });
      await queryClient.invalidateQueries({ queryKey: ["sources"] });
      resetFlowState(sourceId);
    } catch (error) {
      logger.error("Failed to disable source", { sourceId, error });
      alert(`Failed to disable source: ${error}`);
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
      logger.debug("Loading preview", { sourceId });
      const data = await invoke<SourcePreview>("get_source_preview", { sourceId });
      updateFlowState(sourceId, { preview: data });
    } catch (error) {
      logger.error("Failed to load preview", { sourceId, error });
      alert(`Failed to load preview: ${error}`);
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
    logger.debug("Endpoint selected", {
      sourceId,
      targetId,
      endpointId,
      endpointUrl,
      endpointName,
      authenticated,
    });
    updateFlowState(sourceId, {
      step: "security_check",
      selectedTarget: targetId,
      selectedEndpoint: endpointId,
      selectedEndpointUrl: endpointUrl,
      selectedEndpointName: endpointName,
      selectedAuthenticated: authenticated,
      selectedAuthType: authType || null,
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
      logger.error("Invalid flow state for security confirm", { sourceId, state });
      return;
    }

    try {
      logger.debug("Creating binding and enabling source", { sourceId, state });
      await createBinding.mutateAsync({
        sourceId,
        targetId: state.selectedTarget,
        endpointId: state.selectedEndpoint,
        endpointUrl: state.selectedEndpointUrl,
        endpointName: state.selectedEndpointName,
      });
      await invoke("enable_source", { sourceId });
      await queryClient.invalidateQueries({ queryKey: ["sources"] });
      await queryClient.invalidateQueries({ queryKey: ["bindings", sourceId] });
      resetFlowState(sourceId);
    } catch (error) {
      logger.error("Failed to enable source", { sourceId, error });
      alert(`Failed to enable source: ${error}`);
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
      logger.debug("Removing binding", { sourceId, endpointId });
      await removeBinding.mutateAsync({ sourceId, endpointId });
    } catch (error) {
      logger.error("Failed to remove binding", { sourceId, endpointId, error });
      alert(`Failed to remove binding: ${error}`);
    }
  };

  const [pushingSource, setPushingSource] = useState<string | null>(null);

  const handlePushNow = async (sourceId: string) => {
    setPushingSource(sourceId);
    try {
      logger.debug("Manual push triggered", { sourceId });
      await invoke<string>("trigger_source_push", { sourceId });
      logger.info("Manual push enqueued, delivery worker will send within 5s", { sourceId });
      // Refresh delivery status after a short delay to show the pending entry
      setTimeout(() => loadDeliveryStatus(), 1000);
    } catch (error) {
      logger.error("Manual push failed", { sourceId, error });
      alert(`Push failed: ${error}`);
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
      <div className="card">
        <h2 className="card-title">Data Sources</h2>
        <div style={{ color: "var(--text-secondary)", textAlign: "center", padding: "20px" }}>
          Loading sources...
        </div>
      </div>
    );
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "12px" }}>
      {sources?.map((source) => (
        <SourceCard
          key={source.id}
          source={source}
          flowState={getFlowState(source.id)}
          previewLoading={previewLoading[source.id] || false}
          trafficLightStatus={getTrafficLightStatus(source.id, source.enabled)}
          onEnableClick={handleEnableClick}
          onPreviewEnable={handlePreviewEnable}
          onPreviewRefresh={handlePreviewRefresh}
          onEndpointSelect={handleEndpointSelect}
          onSecurityConfirm={handleSecurityConfirm}
          onCancelFlow={handleCancelFlow}
          onBackToEndpointPicker={handleBackToEndpointPicker}
          onUnbind={handleUnbind}
          onPushNow={handlePushNow}
          isPushing={pushingSource === source.id}
        />
      ))}
    </div>
  );
}

interface SourceCardProps {
  source: {
    id: string;
    name: string;
    description: string;
    enabled: boolean;
    lastSync: string | null;
  };
  flowState: FlowState;
  previewLoading: boolean;
  trafficLightStatus: "green" | "yellow" | "red" | "grey";
  onEnableClick: (sourceId: string, isEnabled: boolean) => void;
  onPreviewEnable: (sourceId: string) => void;
  onPreviewRefresh: (sourceId: string) => void;
  onEndpointSelect: (
    sourceId: string,
    targetId: string,
    endpointId: string,
    endpointUrl: string,
    endpointName: string,
    authenticated: boolean,
    authType?: string
  ) => void;
  onSecurityConfirm: (sourceId: string) => void;
  onCancelFlow: (sourceId: string) => void;
  onBackToEndpointPicker: (sourceId: string) => void;
  onUnbind: (sourceId: string, endpointId: string) => void;
  onPushNow: (sourceId: string) => void;
  isPushing: boolean;
}

function SourceCard({
  source,
  flowState,
  previewLoading,
  trafficLightStatus,
  onEnableClick,
  onPreviewEnable,
  onPreviewRefresh,
  onEndpointSelect,
  onSecurityConfirm,
  onCancelFlow,
  onBackToEndpointPicker,
  onUnbind,
  onPushNow,
  isPushing,
}: SourceCardProps) {
  const { data: bindings } = useBindings(source.id);

  return (
    <div className="card">
      {/* Card Header */}
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start" }}>
        <div style={{ flex: 1 }}>
          <div style={{ display: "flex", alignItems: "center", gap: "8px", marginBottom: "4px" }}>
            <TrafficLight status={trafficLightStatus} />
            <h3 className="card-title" style={{ marginBottom: 0 }}>
              {source.name}
            </h3>
          </div>
          <p style={{ fontSize: "12px", color: "var(--text-secondary)", marginBottom: "8px" }}>
            {source.description}
          </p>
          {source.lastSync && (
            <p style={{ fontSize: "11px", color: "var(--text-secondary)" }}>
              Last sync: {new Date(source.lastSync).toLocaleString()}
            </p>
          )}
        </div>
        <button
          className={source.enabled ? "btn btn-secondary" : "btn"}
          onClick={() => onEnableClick(source.id, source.enabled)}
          style={{ marginLeft: "12px" }}
        >
          {source.enabled ? "Disable" : "Enable"}
        </button>
      </div>

      {/* Flow Steps */}
      {flowState.step === "preview" && flowState.preview && (
        <div style={{ marginTop: "16px" }}>
          <TransparencyPreview
            sourceId={source.id}
            preview={flowState.preview}
            onEnable={() => onPreviewEnable(source.id)}
            onRefresh={() => onPreviewRefresh(source.id)}
            isLoading={previewLoading}
          />
        </div>
      )}

      {flowState.step === "pick_endpoint" && (
        <div style={{ marginTop: "16px" }}>
          <EndpointPicker
            onSelect={(targetId, endpointId, endpointUrl, endpointName, authenticated, authType) =>
              onEndpointSelect(
                source.id,
                targetId,
                endpointId,
                endpointUrl,
                endpointName,
                authenticated,
                authType
              )
            }
            onCancel={() => onCancelFlow(source.id)}
          />
        </div>
      )}

      {flowState.step === "security_check" &&
        flowState.selectedEndpointUrl &&
        flowState.selectedEndpointName && (
          <div style={{ marginTop: "16px" }}>
            <SecurityCoaching
              endpointUrl={flowState.selectedEndpointUrl}
              authenticated={flowState.selectedAuthenticated}
              authType={flowState.selectedAuthType || undefined}
              onConfirm={() => onSecurityConfirm(source.id)}
              onBack={() => onBackToEndpointPicker(source.id)}
            />
          </div>
        )}

      {/* Bindings List (when enabled) */}
      {source.enabled && bindings && bindings.length > 0 && flowState.step === "idle" && (
        <div style={{ marginTop: "16px", paddingTop: "16px", borderTop: "1px solid var(--border)" }}>
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "8px" }}>
            <h4 style={{ fontSize: "12px", fontWeight: 600, color: "var(--text-secondary)", margin: 0 }}>
              Bound Endpoints
            </h4>
            <button
              className="btn"
              style={{ fontSize: "11px", padding: "4px 10px" }}
              onClick={() => onPushNow(source.id)}
              disabled={isPushing}
            >
              {isPushing ? "Pushing..." : "Push Now"}
            </button>
          </div>
          {bindings.map((binding) => (
            <div
              key={binding.endpoint_id}
              style={{
                display: "flex",
                justifyContent: "space-between",
                alignItems: "center",
                padding: "8px",
                background: "var(--bg-primary)",
                borderRadius: "4px",
                marginBottom: "4px",
              }}
            >
              <div>
                <div style={{ fontSize: "12px", fontWeight: 500 }}>{binding.endpoint_name}</div>
                <div
                  style={{
                    fontSize: "10px",
                    color: "var(--text-secondary)",
                    fontFamily: "'SF Mono', Monaco, 'Cascadia Code', monospace",
                  }}
                >
                  {binding.endpoint_url}
                </div>
              </div>
              <button
                className="btn btn-secondary"
                style={{ fontSize: "11px", padding: "4px 8px" }}
                onClick={() => onUnbind(source.id, binding.endpoint_id)}
              >
                Unbind
              </button>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
