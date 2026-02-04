import { useQuery } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";

export interface DeliveryStatus {
  overall: "active" | "pending" | "error" | "unknown";
  pendingCount: number;
  failedCount: number;
  lastDelivery: string | null;
}

async function getDeliveryStatus(): Promise<DeliveryStatus> {
  return invoke("get_delivery_status");
}

export function useDeliveryStatus() {
  return useQuery({
    queryKey: ["deliveryStatus"],
    queryFn: getDeliveryStatus,
    refetchInterval: 5000, // Poll every 5 seconds
  });
}
