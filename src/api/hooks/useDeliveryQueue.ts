import { useQuery } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";

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
  return invoke("get_delivery_queue");
}

export function useDeliveryQueue() {
  return useQuery({
    queryKey: ["deliveryQueue"],
    queryFn: getDeliveryQueue,
    refetchInterval: 5000,
  });
}
