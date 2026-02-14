import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { logger } from "../../utils/logger";

export interface Binding {
  source_id: string;
  target_id: string;
  endpoint_id: string;
  endpoint_url: string;
  endpoint_name: string;
  created_at: string;
  active: boolean;
  headers_json: string | null;
  auth_credential_key: string | null;
  delivery_mode: string;
  schedule_time: string | null;
  schedule_day: string | null;
  last_scheduled_at: number | null;
}

async function getSourceBindings(sourceId: string): Promise<Binding[]> {
  logger.debug("Fetching bindings for source", { sourceId });
  try {
    const result = await invoke<Binding[]>("get_source_bindings", { sourceId });
    logger.debug("Source bindings fetched", { sourceId, count: result.length });
    return result;
  } catch (error) {
    logger.error("Failed to fetch source bindings", { sourceId, error });
    throw error;
  }
}

async function getAllBindings(): Promise<Binding[]> {
  logger.debug("Fetching all bindings");
  try {
    const result = await invoke<Binding[]>("list_all_bindings");
    logger.debug("All bindings fetched", { count: result.length });
    return result;
  } catch (error) {
    logger.error("Failed to fetch all bindings", { error });
    throw error;
  }
}

async function createBinding(params: {
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
}): Promise<void> {
  logger.debug("Creating binding", {
    sourceId: params.sourceId,
    targetId: params.targetId,
    endpointId: params.endpointId,
    endpointUrl: params.endpointUrl,
    endpointName: params.endpointName,
    hasAuth: !!params.authHeaderName,
    deliveryMode: params.deliveryMode,
  });
  try {
    await invoke("create_binding", {
      sourceId: params.sourceId,
      targetId: params.targetId,
      endpointId: params.endpointId,
      endpointUrl: params.endpointUrl,
      endpointName: params.endpointName,
      customHeaders: params.customHeaders,
      authHeaderName: params.authHeaderName,
      authHeaderValue: params.authHeaderValue,
      preserveAuthCredentialKey: params.preserveAuthCredentialKey,
      deliveryMode: params.deliveryMode,
      scheduleTime: params.scheduleTime,
      scheduleDay: params.scheduleDay,
    });
    logger.info("Binding created", {
      sourceId: params.sourceId,
      endpointId: params.endpointId,
      hasAuth: !!params.authHeaderName,
    });
  } catch (error) {
    logger.error("Failed to create binding", {
      sourceId: params.sourceId,
      endpointId: params.endpointId,
      error,
    });
    throw error;
  }
}

async function removeBinding(params: { sourceId: string; endpointId: string }): Promise<void> {
  logger.debug("Removing binding", params);
  try {
    await invoke("remove_binding", {
      sourceId: params.sourceId,
      endpointId: params.endpointId,
    });
    logger.info("Binding removed", params);
  } catch (error) {
    logger.error("Failed to remove binding", { ...params, error });
    throw error;
  }
}

export function useBindings(sourceId: string) {
  return useQuery({
    queryKey: ["bindings", sourceId],
    queryFn: () => getSourceBindings(sourceId),
  });
}

export function useAllBindings() {
  return useQuery({
    queryKey: ["bindings"],
    queryFn: getAllBindings,
  });
}

export function useCreateBinding() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: createBinding,
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({ queryKey: ["bindings", variables.sourceId] });
      queryClient.invalidateQueries({ queryKey: ["bindings"] });
      queryClient.invalidateQueries({ queryKey: ["sources"] });
    },
  });
}

export function useRemoveBinding() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: removeBinding,
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({ queryKey: ["bindings", variables.sourceId] });
      queryClient.invalidateQueries({ queryKey: ["bindings"] });
      queryClient.invalidateQueries({ queryKey: ["sources"] });
    },
  });
}
