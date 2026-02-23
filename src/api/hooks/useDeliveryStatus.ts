import { useQuery } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { logger } from "../../utils/logger";
import { visibleRefetchInterval } from "./polling";

export interface DeliveryStatus {
  overall: "active" | "pending" | "error" | "unknown";
  pendingCount: number;
  failedCount: number;
  lastDelivery: string | null;
}

async function getDeliveryStatus(): Promise<DeliveryStatus> {
  logger.debug("Fetching delivery status");
  try {
    const result = await invoke<DeliveryStatus>("get_delivery_status");
    logger.debug("Delivery status fetched", {
      overall: result.overall,
      pendingCount: result.pendingCount,
      failedCount: result.failedCount,
    });
    return result;
  } catch (error) {
    logger.error("Failed to fetch delivery status", { error });
    throw error;
  }
}

export function useDeliveryStatus() {
  return useQuery({
    queryKey: ["deliveryStatus"],
    queryFn: getDeliveryStatus,
    refetchInterval: () => visibleRefetchInterval(5000), // Poll every 5 seconds (visible only)
  });
}
