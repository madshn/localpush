import { useQuery } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";

export interface Source {
  id: string;
  name: string;
  description: string;
  enabled: boolean;
  lastSync: string | null;
}

async function getSources(): Promise<Source[]> {
  return invoke("get_sources");
}

export function useSources() {
  return useQuery({
    queryKey: ["sources"],
    queryFn: getSources,
  });
}
