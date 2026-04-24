import type { QueryClient, UseMutationResult } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';
import { startTransition, useCallback, useRef, useState } from 'react';
import { toast } from 'sonner';
import type { Binding } from '../../api/hooks/useBindings';
import { useSourceStatusCounts } from '../../api/hooks/useDeliveryQueue';
import { logger } from '../../utils/logger';
import type {
  DeliveryMode,
  FlowState,
  SourceData,
  SourcePreview,
  TrafficLightStatus,
} from './types';
import { defaultFlowState } from './types';

interface CreateBindingParams {
  sourceId: string;
  targetId: string;
  endpointId: string;
  endpointUrl: string;
  endpointName: string;
  customHeaders?: [string, string][];
  authHeaderName?: string;
  authHeaderValue?: string;
  preserveAuthCredentialKey?: string;
  deliveryMode?: string;
  scheduleTime?: string;
  scheduleDay?: string;
}

interface RemoveBindingParams {
  sourceId: string;
  endpointId: string;
}

interface UsePipelineFlowProps {
  sources: SourceData[] | undefined;
  allBindings: Binding[] | undefined;
  queryClient: QueryClient;
  createBinding: UseMutationResult<void, Error, CreateBindingParams>;
  removeBinding: UseMutationResult<void, Error, RemoveBindingParams>;
}

export function usePipelineFlow({
  sources,
  allBindings,
  queryClient,
  createBinding,
  removeBinding,
}: UsePipelineFlowProps) {
  const [flowStates, setFlowStates] = useState<Record<string, FlowState>>({});
  const [previewLoading, setPreviewLoading] = useState<Record<string, boolean>>({});
  const [pushingSource, setPushingSource] = useState<string | null>(null);
  const [isConfirming, setIsConfirming] = useState<string | null>(null);
  const [isEnabling, setIsEnabling] = useState<string | null>(null);
  const [isDisabling, setIsDisabling] = useState<string | null>(null);
  const idleFlowStatesRef = useRef<Record<string, FlowState>>({});
  const { data: sourceStatusCounts } = useSourceStatusCounts();

  const getFlowState = useCallback(
    (sourceId: string): FlowState => {
      if (flowStates[sourceId]) return flowStates[sourceId];
      const existing = idleFlowStatesRef.current[sourceId];
      if (existing) return existing;
      const created = defaultFlowState(sourceId);
      idleFlowStatesRef.current[sourceId] = created;
      return created;
    },
    [flowStates],
  );

  const updateFlowState = (sourceId: string, updates: Partial<FlowState>) => {
    setFlowStates((prev) => ({
      ...prev,
      [sourceId]: {
        ...(prev[sourceId] || defaultFlowState(sourceId)),
        ...updates,
      },
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
    logger.debug('Disabling source', { sourceId });
    setIsDisabling(sourceId);
    try {
      await invoke('disable_source', { sourceId });
      await queryClient.invalidateQueries({ queryKey: ['sources'] });
      resetFlowState(sourceId);
      logger.info('Source disabled', { sourceId });
    } catch (error) {
      logger.error('Failed to disable source', { sourceId, error });
      toast.error(`Failed to disable source: ${error}`);
    } finally {
      setIsDisabling(null);
    }
  };

  const handleEnableClick = async (sourceId: string, isEnabled: boolean) => {
    logger.debug('Enable click', { sourceId, isEnabled });
    if (isEnabled) {
      await handleDisable(sourceId);
    } else {
      const existingBindings = allBindings?.filter((b) => b.source_id === sourceId) || [];
      if (existingBindings.length > 0) {
        setIsEnabling(sourceId);
        try {
          await invoke('enable_source', { sourceId });
          await queryClient.invalidateQueries({ queryKey: ['sources'] });
          toast.success('Source re-enabled');
          logger.info('Source re-enabled with existing bindings', {
            sourceId,
            bindingCount: existingBindings.length,
          });
        } catch (error) {
          logger.error('Failed to re-enable source', { sourceId, error });
          toast.error(`Failed to enable source: ${error}`);
        } finally {
          setIsEnabling(null);
        }
      } else {
        updateFlowState(sourceId, { step: 'preview' });
        await loadPreview(sourceId);
      }
    }
  };

  const loadPreview = async (sourceId: string) => {
    setPreviewLoading((prev) => ({ ...prev, [sourceId]: true }));
    try {
      const data = await invoke<SourcePreview>('get_source_preview', {
        sourceId,
      });
      updateFlowState(sourceId, { preview: data });
    } catch (error) {
      logger.error('Failed to load preview', { sourceId, error });
      toast.error(`Failed to load preview: ${error}`);
      resetFlowState(sourceId);
    } finally {
      setPreviewLoading((prev) => ({ ...prev, [sourceId]: false }));
    }
  };

  const handlePreviewEnable = (sourceId: string) => {
    updateFlowState(sourceId, { step: 'pick_endpoint' });
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
    authType?: string,
  ) => {
    updateFlowState(sourceId, {
      step: 'configure_delivery',
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
    scheduleDay: string | undefined,
  ) => {
    updateFlowState(sourceId, {
      step: 'security_check',
      customHeaders,
      authHeaderName,
      authHeaderValue,
      deliveryMode,
      scheduleTime,
      scheduleDay,
    });
  };

  const handleBackToDeliveryConfig = (sourceId: string) => {
    updateFlowState(sourceId, { step: 'configure_delivery' });
  };

  const handleAddTarget = (sourceId: string) => {
    logger.info('Add Target flow started', { sourceId });
    // Reset to clean state to avoid stale fields from prior flows
    resetFlowState(sourceId);
    updateFlowState(sourceId, { step: 'pick_endpoint' });
  };

  const handleEditBinding = (sourceId: string, endpointId: string) => {
    logger.info('Edit binding started', { sourceId, endpointId });
    const binding = allBindings?.find(
      (b) => b.source_id === sourceId && b.endpoint_id === endpointId,
    );
    if (!binding) {
      logger.error('Binding not found for editing', { sourceId, endpointId });
      return;
    }

    let existingHeaders: [string, string][] = [];
    let existingAuthName = '';
    const existingAuthValue = '';
    if (binding.headers_json) {
      try {
        const parsed: [string, string][] = JSON.parse(binding.headers_json);
        const authHeader = parsed.find(([, v]) => v === '');
        if (authHeader) {
          existingAuthName = authHeader[0];
        }
        existingHeaders = parsed.filter(([, v]) => v !== '');
      } catch {
        logger.warn('Failed to parse binding headers_json', {
          sourceId,
          endpointId,
        });
      }
    }

    updateFlowState(sourceId, {
      step: 'configure_delivery',
      selectedTarget: binding.target_id,
      selectedEndpoint: binding.endpoint_id,
      selectedEndpointUrl: binding.endpoint_url,
      selectedEndpointName: binding.endpoint_name,
      selectedAuthenticated: !!binding.auth_credential_key,
      selectedAuthType: binding.auth_credential_key ? 'custom' : null,
      customHeaders: existingHeaders,
      authHeaderName: existingAuthName,
      authHeaderValue: existingAuthValue,
      isEditing: true,
      existingAuthCredentialKey: binding.auth_credential_key || null,
      deliveryMode: (binding.delivery_mode || 'on_change') as DeliveryMode,
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
      logger.error('Security confirm aborted — missing flow state', {
        sourceId,
        hasTarget: !!state.selectedTarget,
        hasEndpoint: !!state.selectedEndpoint,
        hasUrl: !!state.selectedEndpointUrl,
        hasName: !!state.selectedEndpointName,
        step: state.step,
      });
      toast.error('Something went wrong. Please try again.');
      resetFlowState(sourceId);
      return;
    }

    const source = sources?.find((s) => s.id === sourceId);
    const alreadyEnabled = source?.enabled ?? false;
    const isEditing = state.isEditing;

    setIsConfirming(sourceId);
    try {
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
        customHeaders: state.customHeaders.length > 0 ? state.customHeaders : undefined,
        authHeaderName: state.authHeaderName || undefined,
        authHeaderValue: state.authHeaderValue || undefined,
        preserveAuthCredentialKey: preserveKey,
        deliveryMode: state.deliveryMode !== 'on_change' ? state.deliveryMode : undefined,
        scheduleTime: state.scheduleTime,
        scheduleDay: state.scheduleDay,
      });
      if (!alreadyEnabled) {
        await invoke('enable_source', { sourceId });
      }
      await queryClient.invalidateQueries({ queryKey: ['sources'] });
      await queryClient.invalidateQueries({
        queryKey: ['bindings', sourceId],
      });
      await queryClient.invalidateQueries({ queryKey: ['bindings'] });
      resetFlowState(sourceId);
      toast.success(
        isEditing
          ? 'Binding updated'
          : alreadyEnabled
            ? 'Additional target connected'
            : 'Source enabled and connected',
      );
      logger.info('Binding saved', {
        sourceId,
        isEditing,
        alreadyEnabled,
        endpointId: state.selectedEndpoint,
      });
    } catch (error) {
      logger.error('Failed to save binding', { sourceId, error });
      toast.error(`Failed to connect target: ${error}`);
    } finally {
      setIsConfirming(null);
    }
  };

  const handleCancelFlow = (sourceId: string) => {
    resetFlowState(sourceId);
  };

  const handleBackToEndpointPicker = (sourceId: string) => {
    updateFlowState(sourceId, { step: 'pick_endpoint' });
  };

  const handleUnbind = async (sourceId: string, endpointId: string) => {
    try {
      await removeBinding.mutateAsync({ sourceId, endpointId });
    } catch (error) {
      logger.error('Failed to remove binding', {
        sourceId,
        endpointId,
        error,
      });
      toast.error(`Failed to remove binding: ${error}`);
    }
  };

  const handlePushNow = (sourceId: string) => {
    logger.info('Push Now triggered', { sourceId });
    setPushingSource(sourceId);
    const toastId = toast('Checking for new data...');
    const pushedAt = Date.now();
    const scheduleAfterPaint =
      typeof requestAnimationFrame === 'function'
        ? requestAnimationFrame
        : (cb: FrameRequestCallback) => window.setTimeout(() => cb(0), 0);

    scheduleAfterPaint(() => {
      invoke<string>('trigger_source_push', { sourceId })
        .then((result) => {
          const elapsed = Date.now() - pushedAt;
          const remaining = Math.max(0, 800 - elapsed);

          if (
            result === 'skipped:no_data' ||
            result === 'skipped:unchanged' ||
            result === 'skipped:claimed_pending' ||
            result === 'skipped:no_bindings'
          ) {
            logger.debug('Push skipped', { sourceId, result });
            setTimeout(() => {
              toast.success(
                result === 'skipped:no_data'
                  ? 'Nothing new to push'
                  : result === 'skipped:unchanged'
                    ? 'No changes since last push'
                    : result === 'skipped:claimed_pending'
                      ? 'An older queued item is still blocking this source'
                    : 'No active bindings for this source',
                { id: toastId },
              );
              setPushingSource(null);
            }, remaining);
            return;
          }

          logger.debug('Push enqueued', { sourceId, result });
          startTransition(() => {
            void queryClient.invalidateQueries({ queryKey: ['deliveryQueue'] });
            void queryClient.invalidateQueries({ queryKey: ['deliveryStatus'] });
          });
          // Keep "Pushing..." visible for at least 800ms so the state change is perceptible
          setTimeout(() => {
            toast.success('Push enqueued — delivering shortly', {
              id: toastId,
            });
            setPushingSource(null);
          }, remaining);
        })
        .catch((error) => {
          logger.error('Manual push failed', { sourceId, error });
          toast.error(`Push failed: ${error}`, { id: toastId });
          setPushingSource(null);
        });
    });
  };

  const getTrafficLightStatus = useCallback(
    (sourceId: string, enabled: boolean): TrafficLightStatus => {
      if (!enabled) return 'grey';
      if (!sourceStatusCounts) return 'grey';
      const counts = sourceStatusCounts.filter((c) => c.source_id === sourceId);
      if (counts.length === 0) return 'grey';
      const hasFailed = counts.some(
        (c) => (c.status === 'failed' || c.status === 'dlq') && c.count > 0,
      );
      if (hasFailed) return 'red';
      const hasTargetPaused = counts.some((c) => c.status === 'target_paused' && c.count > 0);
      if (hasTargetPaused) return 'orange';
      const hasPending = counts.some(
        (c) => (c.status === 'pending' || c.status === 'in_flight') && c.count > 0,
      );
      if (hasPending) return 'yellow';
      const hasDelivered = counts.some((c) => c.status === 'delivered' && c.count > 0);
      if (hasDelivered) return 'green';
      return 'grey';
    },
    [sourceStatusCounts],
  );

  return {
    flowStates,
    previewLoading,
    pushingSource,
    isConfirming,
    isEnabling,
    isDisabling,
    getFlowState,
    getTrafficLightStatus,
    handleEnableClick,
    handlePreviewEnable,
    handlePreviewRefresh,
    handleEndpointSelect,
    handleDeliveryConfigConfirm,
    handleSecurityConfirm,
    handleCancelFlow,
    handleBackToEndpointPicker,
    handleBackToDeliveryConfig,
    handleUnbind,
    handlePushNow,
    handleAddTarget,
    handleEditBinding,
  };
}
