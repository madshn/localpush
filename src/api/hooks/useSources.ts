import { useQuery } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { logger } from "../../utils/logger";

export interface Source {
  id: string;
  name: string;
  description: string;
  enabled: boolean;
  lastSync: string | null;
}

async function getSources(): Promise<Source[]> {
  logger.debug("Fetching sources");
  try {
    const result = await invoke<Source[]>("get_sources");
    logger.debug("Sources fetched", { count: result.length });
    return result;
  } catch (error) {
    logger.error("Failed to fetch sources", { error });
    throw error;
  }
}

export function useSources() {
  return useQuery({
    queryKey: ["sources"],
    queryFn: getSources,
  });
}
