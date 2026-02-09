import { ArrowDown } from "lucide-react";
import { useBindings } from "../api/hooks/useBindings";
import { TransparencyPreview } from "./TransparencyPreview";
import { EndpointPicker } from "./EndpointPicker";
import { SecurityCoaching } from "./SecurityCoaching";
import { DeliveryConfig } from "./DeliveryConfig";

interface SourcePreview {
  title: string;
  summary: string;
  fields: Array<{ label: string; value: string; sensitive: boolean }>;
  lastUpdated: string | null;
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

interface PipelineCardProps {
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
  onDeliveryConfigConfirm: (
    sourceId: string,
    customHeaders: [string, string][],
    authHeaderName: string,
    authHeaderValue: string
  ) => void;
  onSecurityConfirm: (sourceId: string) => void;
  onCancelFlow: (sourceId: string) => void;
  onBackToEndpointPicker: (sourceId: string) => void;
  onBackToDeliveryConfig: (sourceId: string) => void;
  onUnbind: (sourceId: string, endpointId: string) => void;
  onPushNow: (sourceId: string) => void;
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

export function PipelineCard({
  source,
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
  isPushing,
}: PipelineCardProps) {
  const { data: bindings } = useBindings(source.id);
  const status = statusConfig[trafficLightStatus];

  return (
    <div className="relative bg-bg-secondary border border-border rounded-lg overflow-hidden">
      {/* Left colored stripe */}
      <div className={`absolute left-0 top-0 bottom-0 w-1 ${status.stripe}`} />

      <div className="pl-4 pr-3 py-3">
        {/* Header */}
        <div className="flex items-start justify-between">
          <div className="flex-1 min-w-0">
            <div className="flex items-center gap-2 mb-0.5">
              <h3 className="text-sm font-semibold truncate">{source.name}</h3>
              <span
                className={`inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[10px] font-medium ${status.badgeClass}`}
              >
                {status.pulse && (
                  <span className="w-1.5 h-1.5 rounded-full bg-current animate-pulse" />
                )}
                {status.badge}
              </span>
            </div>
            <p className="text-xs text-text-secondary">{source.description}</p>
          </div>
          <button
            className={`ml-3 shrink-0 text-xs font-medium px-3 py-1.5 rounded-md transition-colors ${
              source.enabled
                ? "bg-bg-tertiary text-text-secondary border border-border hover:border-border-hover"
                : "bg-accent text-white hover:bg-accent/90"
            }`}
            onClick={() => onEnableClick(source.id, source.enabled)}
          >
            {source.enabled ? "Disable" : "Enable"}
          </button>
        </div>

        {/* Binding flow arrow + target (when enabled and idle) */}
        {source.enabled &&
          bindings &&
          bindings.length > 0 &&
          flowState.step === "idle" && (
            <div className="mt-3">
              {bindings.map((binding) => (
                <div key={binding.endpoint_id}>
                  {/* Flow arrow */}
                  <div className="flex items-center gap-2 py-1.5 pl-2">
                    <div className="w-px h-4 bg-border" />
                    <ArrowDown size={12} className="text-text-secondary" />
                  </div>
                  {/* Target card */}
                  <div className="flex items-center justify-between bg-bg-primary rounded-md px-3 py-2">
                    <div className="min-w-0 flex-1">
                      <div className="text-xs font-medium truncate">
                        {binding.endpoint_name}
                      </div>
                      <div className="text-[10px] text-text-secondary font-mono truncate">
                        {binding.endpoint_url}
                      </div>
                    </div>
                    <button
                      className="ml-2 text-[10px] text-text-secondary hover:text-error transition-colors"
                      onClick={() => onUnbind(source.id, binding.endpoint_id)}
                    >
                      Unbind
                    </button>
                  </div>
                </div>
              ))}

              {/* Push Now button */}
              <div className="flex justify-end mt-2">
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
                endpointName={flowState.selectedEndpointName}
                endpointUrl={flowState.selectedEndpointUrl}
                authenticated={flowState.selectedAuthenticated}
                onConfirm={(customHeaders, authHeaderName, authHeaderValue) =>
                  onDeliveryConfigConfirm(
                    source.id,
                    customHeaders,
                    authHeaderName,
                    authHeaderValue
                  )
                }
                onBack={() => onBackToEndpointPicker(source.id)}
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
                onConfirm={() => onSecurityConfirm(source.id)}
                onBack={() => onBackToDeliveryConfig(source.id)}
              />
            </div>
          )}
      </div>
    </div>
  );
}
