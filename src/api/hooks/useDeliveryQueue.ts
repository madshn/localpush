import { useQuery } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { logger } from "../../utils/logger";

export interface DeliveryItem {
  id: string;
  eventType: string;
  status: "pending" | "in_flight" | "delivered" | "failed" | "dlq";
  retryCount: number;
  lastError: string | null;
  createdAt: string;
  deliveredAt: string | null;
}

async function getDeliveryQueue(): Promise<DeliveryItem[]> {
  logger.debug("Fetching delivery queue");
  try {
    const result = await invoke<DeliveryItem[]>("get_delivery_queue");
    logger.debug("Delivery queue fetched", { count: result.length });
    return result;
  } catch (error) {
    logger.error("Failed to fetch delivery queue", { error });
    throw error;
  }
}

export function useDeliveryQueue() {
  return useQuery({
    queryKey: ["deliveryQueue"],
    queryFn: getDeliveryQueue,
    refetchInterval: 5000,
  });
}
