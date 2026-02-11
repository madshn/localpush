import { TransparencyPreview } from "../TransparencyPreview";
import { EndpointPicker } from "../EndpointPicker";
import { DeliveryConfig } from "../DeliveryConfig";
import { SecurityCoaching } from "../SecurityCoaching";
import type { FlowState, DeliveryMode } from "./types";

interface FlowModalProps {
  flowState: FlowState;
  previewLoading: boolean;
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
}

export function FlowModal({
  flowState,
  previewLoading,
  onPreviewEnable,
  onPreviewRefresh,
  onEndpointSelect,
  onDeliveryConfigConfirm,
  onSecurityConfirm,
  onCancelFlow,
  onBackToEndpointPicker,
  onBackToDeliveryConfig,
  onUnbind,
}: FlowModalProps) {
  if (flowState.step === "idle") return null;

  const sourceId = flowState.sourceId;

  return (
    <div
      className="fixed inset-0 z-50 flex items-start justify-center pt-12 px-4"
      onClick={(e) => {
        if (e.target === e.currentTarget) onCancelFlow(sourceId);
      }}
    >
      {/* Backdrop */}
      <div className="absolute inset-0 bg-black/50 backdrop-blur-sm" />

      {/* Content */}
      <div className="relative w-full max-w-md max-h-[80vh] overflow-y-auto rounded-xl bg-bg-primary border border-border shadow-2xl p-4">
        {flowState.step === "preview" && flowState.preview && (
          <TransparencyPreview
            sourceId={sourceId}
            preview={flowState.preview}
            onEnable={() => onPreviewEnable(sourceId)}
            onRefresh={() => onPreviewRefresh(sourceId)}
            isLoading={previewLoading}
          />
        )}

        {flowState.step === "pick_endpoint" && (
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
                sourceId,
                targetId,
                endpointId,
                endpointUrl,
                endpointName,
                authenticated,
                authType
              )
            }
            onCancel={() => onCancelFlow(sourceId)}
          />
        )}

        {flowState.step === "configure_delivery" &&
          flowState.selectedEndpointUrl &&
          flowState.selectedEndpointName && (
            <DeliveryConfig
              sourceId={sourceId}
              endpointName={flowState.selectedEndpointName}
              endpointUrl={flowState.selectedEndpointUrl}
              authenticated={flowState.selectedAuthenticated}
              existingAuthConfigured={!!flowState.existingAuthCredentialKey}
              initialHeaders={
                flowState.customHeaders.length > 0
                  ? flowState.customHeaders
                  : undefined
              }
              initialAuthName={flowState.authHeaderName || undefined}
              initialAuthValue={flowState.authHeaderValue || undefined}
              initialDeliveryMode={flowState.deliveryMode}
              initialScheduleTime={flowState.scheduleTime}
              initialScheduleDay={flowState.scheduleDay}
              onConfirm={(
                customHeaders,
                authHeaderName,
                authHeaderValue,
                deliveryMode,
                scheduleTime,
                scheduleDay
              ) =>
                onDeliveryConfigConfirm(
                  sourceId,
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
                  ? onCancelFlow(sourceId)
                  : onBackToEndpointPicker(sourceId)
              }
              onUnbind={
                flowState.isEditing && flowState.selectedEndpoint
                  ? () => onUnbind(sourceId, flowState.selectedEndpoint!)
                  : undefined
              }
            />
          )}

        {flowState.step === "security_check" &&
          flowState.selectedEndpointUrl &&
          flowState.selectedEndpointName && (
            <SecurityCoaching
              endpointUrl={flowState.selectedEndpointUrl}
              authenticated={flowState.selectedAuthenticated}
              authType={flowState.selectedAuthType || undefined}
              isEditing={flowState.isEditing}
              onConfirm={() => onSecurityConfirm(sourceId)}
              onBack={() => onBackToDeliveryConfig(sourceId)}
            />
          )}
      </div>
    </div>
  );
}
