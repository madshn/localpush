import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';
import { logger } from '../../utils/logger';

interface Target {
  id: string;
  name: string;
  target_type: string;
}

interface TargetInfo {
  id: string;
  name: string;
  target_type: string;
  base_url: string;
  connected: boolean;
  details?: {
    active_workflows?: number;
  };
}

interface Endpoint {
  id: string;
  name: string;
  url: string;
  authenticated: boolean;
  auth_type?: string;
  metadata?: Record<string, any>;
}

export function useTargets() {
  return useQuery<Target[]>({
    queryKey: ['targets'],
    queryFn: async () => {
      logger.debug('Fetching targets');
      const targets = await invoke<Target[]>('list_targets');
      logger.debug('Targets fetched', { count: targets.length });
      return targets;
    },
  });
}

export function useTargetEndpoints(targetId: string | null) {
  return useQuery<Endpoint[]>({
    queryKey: ['target-endpoints', targetId],
    queryFn: async () => {
      if (!targetId) return [];
      logger.debug('Fetching endpoints for target', { targetId });
      const endpoints = await invoke<Endpoint[]>('list_target_endpoints', { targetId });
      logger.debug('Endpoints fetched', { targetId, count: endpoints.length });
      return endpoints;
    },
    enabled: !!targetId,
  });
}

export function useConnectN8n() {
  const queryClient = useQueryClient();

  return useMutation<TargetInfo, Error, { instanceUrl: string; apiKey: string }>({
    mutationFn: async ({ instanceUrl, apiKey }) => {
      logger.debug('Connecting n8n target', { instanceUrl });
      const result = await invoke<TargetInfo>('connect_n8n_target', {
        instanceUrl,
        apiKey,
      });
      logger.debug('n8n target connected', { targetId: result.id });
      return result;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['targets'] });
    },
  });
}

export function useConnectNtfy() {
  const queryClient = useQueryClient();

  return useMutation<TargetInfo, Error, { serverUrl: string; topic?: string; authToken?: string }>({
    mutationFn: async ({ serverUrl, topic, authToken }) => {
      logger.debug('Connecting ntfy target', { serverUrl, topic });
      const result = await invoke<TargetInfo>('connect_ntfy_target', {
        serverUrl,
        topic: topic || undefined,
        authToken: authToken || undefined,
      });
      logger.debug('ntfy target connected', { targetId: result.id });
      return result;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['targets'] });
    },
  });
}

export function useTestTargetConnection() {
  return useMutation<TargetInfo, Error, string>({
    mutationFn: async (targetId) => {
      logger.debug('Testing target connection', { targetId });
      const result = await invoke<TargetInfo>('test_target_connection', { targetId });
      logger.debug('Target connection test complete', { targetId, connected: result.connected });
      return result;
    },
  });
}
