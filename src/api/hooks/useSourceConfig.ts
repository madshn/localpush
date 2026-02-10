import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";

export interface PropertyDef {
  key: string;
  label: string;
  description: string;
  enabled: boolean;
  privacy_sensitive: boolean;
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
