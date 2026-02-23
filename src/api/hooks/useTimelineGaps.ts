import { useQuery } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { logger } from "../../utils/logger";
import { visibleRefetchInterval } from "./polling";

export interface TimelineGap {
  source_id: string;
  source_name: string;
  binding_id: string;
  expected_at: string;
  delivery_mode: string;
  last_delivered_at: string | null;
}

async function fetchTimelineGaps(): Promise<TimelineGap[]> {
  logger.debug("Fetching timeline gaps");
  try {
    const gaps = await invoke<TimelineGap[]>("get_timeline_gaps", {});
    logger.info("Timeline gaps fetched", { count: gaps.length });
    return gaps;
  } catch (error) {
    logger.error("Failed to fetch timeline gaps", { error });
    throw error;
  }
}

/**
 * Hook to fetch timeline gaps for scheduled deliveries that didn't happen.
 * Polls every 60 seconds since gaps change slowly.
 */
export function useTimelineGaps() {
  return useQuery({
    queryKey: ["timelineGaps"],
    queryFn: fetchTimelineGaps,
    refetchInterval: () => visibleRefetchInterval(60 * 1000), // Poll every 60 seconds (visible only)
    staleTime: 30 * 1000, // Consider stale after 30s
  });
}
