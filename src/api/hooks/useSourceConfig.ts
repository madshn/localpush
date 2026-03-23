import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";

export interface PropertyDef {
  key: string;
  label: string;
  description: string;
  enabled: boolean;
  privacy_sensitive: boolean;
}

export interface WindowSetting {
  label: string;
  description: string;
  days: number;
  default_days: number;
  min_days: number;
  max_days: number;
  recommended_days: number[];
}

/**
 * Fetch configurable properties for a source
 */
export function useSourceProperties(sourceId: string) {
  return useQuery<PropertyDef[]>({
    queryKey: ["source-properties", sourceId],
    queryFn: async () => {
      return await invoke<PropertyDef[]>("get_source_properties", {
        sourceId,
      });
    },
    enabled: !!sourceId,
  });
}

export function useSourceWindowSetting(sourceId: string) {
  return useQuery<WindowSetting | null>({
    queryKey: ["source-window", sourceId],
    queryFn: async () => {
      return await invoke<WindowSetting | null>("get_source_window_setting", {
        sourceId,
      });
    },
    enabled: !!sourceId,
  });
}

/**
 * Mutation to enable/disable a source property
 */
export function useSetSourceProperty() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async ({
      sourceId,
      property,
      enabled,
    }: {
      sourceId: string;
      property: string;
      enabled: boolean;
    }) => {
      await invoke("set_source_property", {
        sourceId,
        property,
        enabled,
      });
    },
    onSuccess: (_data, variables) => {
      // Invalidate the specific source's properties to refetch
      queryClient.invalidateQueries({
        queryKey: ["source-properties", variables.sourceId],
      });
    },
  });
}

export function useSetSourceWindowDays() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async ({
      sourceId,
      days,
    }: {
      sourceId: string;
      days: number;
    }) => {
      await invoke("set_source_window_days", {
        sourceId,
        days,
      });
    },
    onSuccess: (_data, variables) => {
      queryClient.invalidateQueries({
        queryKey: ["source-window", variables.sourceId],
      });
      queryClient.invalidateQueries({
        queryKey: ["source-preview", variables.sourceId],
      });
      queryClient.invalidateQueries({
        queryKey: ["sources"],
      });
    },
  });
}
