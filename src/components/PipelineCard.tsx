import { useState } from "react";
import { Plus, Pencil, Info, X, Zap, SlidersHorizontal, AlertTriangle, RefreshCw } from "lucide-react";
import { useBindings, type Binding } from "../api/hooks/useBindings";
import { useTargetHealth, useReconnectTarget } from "../api/hooks/useTargets";
import { TransparencyPreview } from "./TransparencyPreview";
import { EndpointPicker } from "./EndpointPicker";
import { SecurityCoaching } from "./SecurityCoaching";
import { DeliveryConfig } from "./DeliveryConfig";
import { SourceSettings } from "./SourceSettings";

type SourceCategory = "active" | "paused" | "available";

interface SourcePreview {
  title: string;
  summary: string;
  fields: Array<{ label: string; value: string; sensitive: boolean }>;
  lastUpdated: string | null;
}

type DeliveryMode = "on_change" | "interval" | "daily" | "weekly";

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
  isEditing: boolean;
  existingAuthCredentialKey: string | null;
  deliveryMode: DeliveryMode;
  scheduleTime: string | undefined;
  scheduleDay: string | undefined;
}

interface PipelineCardProps {
  source: {
    id: string;
    name: string;
    description: string;
    enabled: boolean;
    last_sync: string | null;
    watch_path: string | null;
  };
  category: SourceCategory;
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
  onDeliveryConfigConfirm: (
    sourceId: string,
    customHeaders: [string, string][],
    authHeaderName: string,
    authHeaderValue: string,
    deliveryMode: DeliveryMode,
    scheduleTime: string | undefined,
    scheduleDay: string | undefined
  ) => void;
  onSecurityConfirm: (sourceId: string) => void;
  onCancelFlow: (sourceId: string) => void;
  onBackToEndpointPicker: (sourceId: string) => void;
  onBackToDeliveryConfig: (sourceId: string) => void;
  onUnbind: (sourceId: string, endpointId: string) => void;
  onPushNow: (sourceId: string) => void;
  onAddTarget: (sourceId: string) => void;
  onEditBinding: (sourceId: string, endpointId: string) => void;
  isPushing: boolean;
}

const statusConfig = {
  green: {
    stripe: "bg-success",
    badge: "Flowing",
    badgeClass: "bg-success-bg text-success",
    pulse: true,
  },
  yellow: {
    stripe: "bg-warning",
    badge: "Pending",
    badgeClass: "bg-warning-bg text-warning",
    pulse: false,
  },
  red: {
    stripe: "bg-error",
    badge: "Error",
    badgeClass: "bg-error-bg text-error",
    pulse: false,
  },
  grey: {
    stripe: "bg-text-secondary/30",
    badge: "Paused",
    badgeClass: "bg-bg-tertiary text-text-secondary",
    pulse: false,
  },
} as const;

function deliveryModeBadge(binding: Binding): string | null {
  if (!binding.delivery_mode || binding.delivery_mode === "on_change") return null;
  if (binding.delivery_mode === "interval") {
    const mins = binding.schedule_time || "15";
    return `Every ${mins}m`;
  }
  if (binding.delivery_mode === "daily") {
    return `Daily ${binding.schedule_time || "00:01"}`;
  }
  if (binding.delivery_mode === "weekly") {
    const day = binding.schedule_day
      ? binding.schedule_day.charAt(0).toUpperCase() + binding.schedule_day.slice(1, 3)
      : "Mon";
    return `Weekly ${day} ${binding.schedule_time || "00:01"}`;
  }
  return null;
}

export function PipelineCard({
  source,
  category,
  flowState,
  previewLoading,
  trafficLightStatus,
  onEnableClick,
  onPreviewEnable,
  onPreviewRefresh,
  onEndpointSelect,
  onDeliveryConfigConfirm,
  onSecurityConfirm,
  onCancelFlow,
  onBackToEndpointPicker,
  onBackToDeliveryConfig,
  onUnbind,
  onPushNow,
  onAddTarget,
  onEditBinding,
  isPushing,
}: PipelineCardProps) {
  const { data: bindings } = useBindings(source.id);
  const { data: healthData } = useTargetHealth();
  const reconnectTarget = useReconnectTarget();
  const [showInfo, setShowInfo] = useState(false);
  const [showProperties, setShowProperties] = useState(false);
  const [showDisableConfirm, setShowDisableConfirm] = useState(false);

  const degradedMap = new Map(
    (healthData ?? [])
      .filter((h) => h.status === "degraded")
      .map((h) => [h.target_id, h])
  );

  const effectiveStatus =
    category === "paused"
      ? "grey"
      : category === "available"
        ? "grey"
        : trafficLightStatus;
  const status = statusConfig[effectiveStatus];

  const isAvailable = category === "available";
  const isPaused = category === "paused";
  const isFlowActive = flowState.step !== "idle";

  const handleEnableDisable = () => {
    if (source.enabled) {
      setShowDisableConfirm(true);
    } else {
      onEnableClick(source.id, source.enabled);
    }
  };

  const confirmDisable = () => {
    setShowDisableConfirm(false);
    onEnableClick(source.id, source.enabled);
  };

  return (
    <div
      className={`relative bg-bg-secondary border border-border rounded-lg overflow-hidden`}
    >
      {/* Left colored stripe */}
      <div className={`absolute left-0 top-0 bottom-0 w-1 ${status.stripe}`} />

      <div className="pl-4 pr-3 py-3">
        {/* Header */}
        <div className="flex items-start justify-between">
          <div className="flex-1 min-w-0">
            <div className="flex items-center gap-2 mb-0.5 flex-wrap">
              <h3 className="text-sm font-semibold truncate">{source.name}</h3>
              {!isAvailable && (
                <span
                  className={`inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[10px] font-medium ${status.badgeClass}`}
                >
                  {status.pulse && (
                    <span className="w-1.5 h-1.5 rounded-full bg-current animate-pulse" />
                  )}
                  {status.badge}
                </span>
              )}
              {category === "active" && (() => {
                const hasScheduled = bindings?.some(
                  (b) => b.delivery_mode && b.delivery_mode !== "on_change"
                );
                const allScheduled = bindings?.every(
                  (b) => b.delivery_mode && b.delivery_mode !== "on_change"
                );
                if (allScheduled && bindings && bindings.length > 0) {
                  return (
                    <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[10px] font-medium bg-accent/10 text-accent">
                      Scheduled
                    </span>
                  );
                }
                if (hasScheduled) {
                  return (
                    <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[10px] font-medium bg-accent/10 text-accent">
                      <Zap size={8} />
                      Mixed
                    </span>
                  );
                }
                return (
                  <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[10px] font-medium bg-accent/10 text-accent">
                    <Zap size={8} />
                    Event-driven
                  </span>
                );
              })()}
              {!isAvailable && (
                <button
                  onClick={() => setShowInfo(!showInfo)}
                  className="p-0.5 text-text-secondary/50 hover:text-accent transition-colors rounded"
                  title="Source details"
                >
                  <Info size={13} />
                </button>
              )}
              {source.enabled && !isAvailable && (
                <button
                  onClick={() => setShowProperties(!showProperties)}
                  className="p-0.5 text-text-secondary/50 hover:text-accent transition-colors rounded"
                  title="Data Properties"
                >
                  <Pencil size={13} />
                </button>
              )}
            </div>
            <p className="text-xs text-text-secondary">{source.description}</p>
          </div>
          <button
            className={`ml-3 shrink-0 text-xs font-medium px-3 py-1.5 rounded-md transition-colors ${
              source.enabled
                ? "bg-bg-tertiary text-text-secondary border border-border hover:border-border-hover"
                : isAvailable
                  ? "bg-accent text-white hover:bg-accent/90 shadow-sm shadow-accent/20"
                  : "bg-accent text-white hover:bg-accent/90"
            }`}
            onClick={handleEnableDisable}
          >
            {source.enabled ? "Disable" : "Configure"}
          </button>
        </div>

        {/* Disable confirmation */}
        {showDisableConfirm && (
          <div className="mt-2 p-3 bg-warning-bg border border-warning/20 rounded-md">
            <p className="text-xs text-warning font-medium mb-2">
              Disable {source.name}?
            </p>
            <p className="text-[10px] text-text-secondary mb-3">
              This will stop watching for file changes and pause all deliveries.
              Your target bindings will be preserved.
            </p>
            <div className="flex items-center gap-2 justify-end">
              <button
                className="text-[11px] px-3 py-1 rounded-md bg-bg-tertiary text-text-secondary border border-border hover:border-border-hover transition-colors"
                onClick={() => setShowDisableConfirm(false)}
              >
                Cancel
              </button>
              <button
                className="text-[11px] px-3 py-1 rounded-md bg-warning text-bg-primary font-medium hover:bg-warning/90 transition-colors"
                onClick={confirmDisable}
              >
                Disable
              </button>
            </div>
          </div>
        )}

        {/* Source info panel */}
        {showInfo && (
          <div className="mt-2 p-3 bg-bg-primary border border-border rounded-md text-xs">
            <div className="flex items-center justify-between mb-2">
              <span className="font-medium text-text-primary">Source Details</span>
              <button
                onClick={() => setShowInfo(false)}
                className="p-0.5 text-text-secondary hover:text-text-primary transition-colors"
              >
                <X size={12} />
              </button>
            </div>
            <div className="flex flex-col gap-1.5 text-text-secondary">
              <div>
                <span className="text-text-secondary/60">Type:</span>{" "}
                <span className="text-text-primary">File watcher (event-driven)</span>
              </div>
              {source.watch_path && (
                <div>
                  <span className="text-text-secondary/60">Watching:</span>{" "}
                  <span className="font-mono text-[10px] text-accent break-all">
                    {source.watch_path}
                  </span>
                </div>
              )}
              <div>
                <span className="text-text-secondary/60">Trigger:</span>{" "}
                <span className="text-text-primary">On file change (FSEvents)</span>
              </div>
              <div>
                <span className="text-text-secondary/60">Delivery:</span>{" "}
                <span className="text-text-primary">Within 5 seconds of trigger</span>
              </div>
              {source.enabled && bindings && (
                <div>
                  <span className="text-text-secondary/60">Targets:</span>{" "}
                  <span className="text-text-primary">{bindings.length} bound</span>
                </div>
              )}
            </div>
            {source.enabled && !showProperties && (
              <button
                onClick={() => setShowProperties(!showProperties)}
                className="mt-2 flex items-center gap-1.5 text-[11px] text-text-secondary hover:text-accent transition-colors"
              >
                <SlidersHorizontal size={11} />
                Data Properties
              </button>
            )}
          </div>
        )}

        {/* Data Properties panel (accessible from header pencil or info panel) */}
        {showProperties && source.enabled && (
          <div className="mt-2">
            <SourceSettings
              sourceId={source.id}
              sourceName={source.name}
              onClose={() => setShowProperties(false)}
            />
          </div>
        )}

        {/* Paused state: no targets connected message */}
        {isPaused && flowState.step === "idle" && (
          <div className="mt-3 flex flex-col items-center gap-2 py-3 border border-dashed border-border rounded-md">
            <p className="text-xs text-text-secondary">
              No targets connected
            </p>
            <button
              className="inline-flex items-center gap-1 text-[11px] font-medium px-3 py-1.5 rounded-md bg-accent text-white hover:bg-accent/90 transition-colors"
              onClick={() => onAddTarget(source.id)}
            >
              <Plus size={12} />
              Add Target
            </button>
          </div>
        )}

        {/* Binding flow arrow + target (when active and idle) */}
        {category === "active" &&
          bindings &&
          bindings.length > 0 &&
          flowState.step === "idle" && (
            <div className="mt-3">
              <div className="flex flex-col gap-1.5">
                {bindings.map((binding) => {
                  const modeBadge = deliveryModeBadge(binding);
                  const degraded = degradedMap.get(binding.target_id);
                  return (
                  <div key={binding.endpoint_id}>
                    <div
                      className={`flex items-center justify-between bg-bg-primary rounded-md px-3 py-2 ${degraded ? "border border-warning/30" : ""}`}
                    >
                      <div className="min-w-0 flex-1">
                        <div className="flex items-center gap-1.5">
                          {degraded && (
                            <AlertTriangle size={12} className="shrink-0 text-warning" />
                          )}
                          <span className="text-xs font-medium truncate">
                            {binding.endpoint_name}
                          </span>
                          {modeBadge && (
                            <span className="shrink-0 inline-flex items-center px-1.5 py-0.5 rounded text-[9px] font-medium bg-accent/10 text-accent">
                              {modeBadge}
                            </span>
                          )}
                        </div>
                        <div className="text-[10px] text-text-secondary font-mono truncate">
                          {binding.endpoint_url}
                        </div>
                      </div>
                      <button
                        className="ml-2 shrink-0 p-1.5 text-text-secondary hover:text-accent transition-colors rounded hover:bg-bg-tertiary"
                        onClick={() => onEditBinding(source.id, binding.endpoint_id)}
                        disabled={isFlowActive}
                        title="Edit binding"
                      >
                        <Pencil size={12} />
                      </button>
                    </div>
                    {degraded && (
                      <div className="flex items-center justify-between px-3 py-1.5 bg-warning-bg rounded-b-md -mt-0.5 border border-t-0 border-warning/20">
                        <span className="text-[10px] text-warning">
                          {degraded.queued_count > 0
                            ? `${degraded.queued_count} deliveries queued`
                            : "Target unreachable"}
                          {degraded.reason ? ` — ${degraded.reason}` : ""}
                        </span>
                        <button
                          className="inline-flex items-center gap-1 text-[10px] font-medium px-2 py-0.5 rounded bg-warning text-bg-primary hover:bg-warning/90 transition-colors disabled:opacity-50"
                          onClick={() => reconnectTarget.mutate(degraded.target_id)}
                          disabled={reconnectTarget.isPending}
                        >
                          <RefreshCw size={9} className={reconnectTarget.isPending ? "animate-spin" : ""} />
                          Reconnect
                        </button>
                      </div>
                    )}
                  </div>
                  );
                })}
              </div>

              {/* Action buttons */}
              <div className="flex items-center justify-between mt-2">
                <button
                  className="inline-flex items-center gap-1 text-[10px] text-text-secondary hover:text-accent transition-colors"
                  onClick={() => onAddTarget(source.id)}
                  disabled={isFlowActive}
                >
                  <Plus size={10} />
                  Add Target
                </button>
                <button
                  className="text-[11px] font-medium px-3 py-1 rounded-md bg-accent text-white hover:bg-accent/90 transition-colors disabled:opacity-50"
                  onClick={() => onPushNow(source.id)}
                  disabled={isPushing}
                >
                  {isPushing ? "Pushing..." : "Push Now"}
                </button>
              </div>
            </div>
          )}

        {/* Flow Steps */}
        {flowState.step === "preview" && flowState.preview && (
          <div className="mt-3">
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
          <div className="mt-3">
            <EndpointPicker
              onSelect={(
                targetId,
                endpointId,
                endpointUrl,
                endpointName,
                authenticated,
                authType
              ) =>
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

        {flowState.step === "configure_delivery" &&
          flowState.selectedEndpointUrl &&
          flowState.selectedEndpointName && (
            <div className="mt-3">
              <DeliveryConfig
                sourceId={source.id}
                endpointName={flowState.selectedEndpointName}
                endpointUrl={flowState.selectedEndpointUrl}
                authenticated={flowState.selectedAuthenticated}
                authType={flowState.selectedAuthType || undefined}
                existingAuthConfigured={!!flowState.existingAuthCredentialKey}
                initialHeaders={flowState.customHeaders.length > 0 ? flowState.customHeaders : undefined}
                initialAuthName={flowState.authHeaderName || undefined}
                initialAuthValue={flowState.authHeaderValue || undefined}
                initialDeliveryMode={flowState.deliveryMode}
                initialScheduleTime={flowState.scheduleTime}
                initialScheduleDay={flowState.scheduleDay}
                onConfirm={(customHeaders, authHeaderName, authHeaderValue, deliveryMode, scheduleTime, scheduleDay) =>
                  onDeliveryConfigConfirm(
                    source.id,
                    customHeaders,
                    authHeaderName,
                    authHeaderValue,
                    deliveryMode,
                    scheduleTime,
                    scheduleDay
                  )
                }
                onBack={() =>
                  flowState.isEditing
                    ? onCancelFlow(source.id)
                    : onBackToEndpointPicker(source.id)
                }
                onUnbind={
                  flowState.isEditing && flowState.selectedEndpoint
                    ? () => onUnbind(source.id, flowState.selectedEndpoint!)
                    : undefined
                }
              />
            </div>
          )}

        {flowState.step === "security_check" &&
          flowState.selectedEndpointUrl &&
          flowState.selectedEndpointName && (
            <div className="mt-3">
              <SecurityCoaching
                endpointUrl={flowState.selectedEndpointUrl}
                authenticated={flowState.selectedAuthenticated}
                authType={flowState.selectedAuthType || undefined}
                isEditing={flowState.isEditing}
                onConfirm={() => onSecurityConfirm(source.id)}
                onBack={() => onBackToDeliveryConfig(source.id)}
              />
            </div>
          )}
      </div>
    </div>
  );
}
