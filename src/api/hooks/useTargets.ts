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
  metadata?: Record<string, unknown>;
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

export function useConnectMake() {
  const queryClient = useQueryClient();

  return useMutation<TargetInfo, Error, { zoneUrl: string; apiKey: string }>({
    mutationFn: async ({ zoneUrl, apiKey }) => {
      logger.debug('Connecting Make.com target', { zoneUrl });
      const result = await invoke<TargetInfo>('connect_make_target', {
        zoneUrl,
        apiKey,
      });
      logger.debug('Make.com target connected', { targetId: result.id });
      return result;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['targets'] });
    },
  });
}

export function useConnectGoogleSheets() {
  const queryClient = useQueryClient();

  return useMutation<
    TargetInfo,
    Error,
    {
      email: string;
      accessToken: string;
      refreshToken: string;
      expiresAt: number;
      clientId: string;
      clientSecret: string;
    }
  >({
    mutationFn: async ({ email, accessToken, refreshToken, expiresAt, clientId, clientSecret }) => {
      logger.debug('Connecting Google Sheets target', { email });
      const result = await invoke<TargetInfo>('connect_google_sheets_target', {
        email,
        accessToken,
        refreshToken,
        expiresAt,
        clientId,
        clientSecret,
      });
      logger.debug('Google Sheets target connected', { targetId: result.id });
      return result;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['targets'] });
    },
  });
}

export function useConnectZapier() {
  const queryClient = useQueryClient();

  return useMutation<TargetInfo, Error, { name: string; webhookUrl: string }>({
    mutationFn: async ({ name, webhookUrl }) => {
      logger.debug('Connecting Zapier target', { name, webhookUrl });
      const result = await invoke<TargetInfo>('connect_zapier_target', {
        name,
        webhookUrl,
      });
      logger.debug('Zapier target connected', { targetId: result.id });
      return result;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['targets'] });
    },
  });
}

export function useConnectCustom() {
  const queryClient = useQueryClient();

  return useMutation<
    TargetInfo,
    Error,
    {
      name: string;
      webhookUrl: string;
      authType: string;
      authToken?: string;
      authHeaderName?: string;
      authHeaderValue?: string;
      authUsername?: string;
      authPassword?: string;
    }
  >({
    mutationFn: async ({
      name,
      webhookUrl,
      authType,
      authToken,
      authHeaderName,
      authHeaderValue,
      authUsername,
      authPassword,
    }) => {
      logger.debug('Connecting Custom target', { name, webhookUrl, authType });
      const result = await invoke<TargetInfo>('connect_custom_target', {
        name,
        webhookUrl,
        authType,
        authToken,
        authHeaderName,
        authHeaderValue,
        authUsername,
        authPassword,
      });
      logger.debug('Custom target connected', { targetId: result.id });
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
