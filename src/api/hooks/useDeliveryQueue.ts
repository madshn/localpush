import { useQuery } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { logger } from "../../utils/logger";
import { visibleRefetchInterval } from "./polling";

export interface DeliveryQueueItemRaw {
  id: string;
  event_type: string;
  status: "pending" | "in_flight" | "delivered" | "failed" | "dlq" | "target_paused";
  retry_count: number;
  last_error: string | null;
  created_at: string;
  delivered_at: string | null;
  payload: unknown;
  trigger_type: string | null;
  delivered_to: string | null;
}

export interface DeliveryItem {
  id: string;
  eventType: string;
  status: "pending" | "in_flight" | "delivered" | "failed" | "dlq" | "target_paused";
  retryCount: number;
  lastError: string | null;
  createdAt: string;
  deliveredAt: string | null;
}

export interface DeliveryQueueCounts {
  total: number;
  delivered: number;
  pending: number;
  inFlight: number;
  failed: number;
  dlq: number;
  targetPaused: number;
}

export const DELIVERY_QUEUE_QUERY_KEY = ["deliveryQueue"] as const;

export async function fetchDeliveryQueue(): Promise<DeliveryQueueItemRaw[]> {
  logger.debug("Fetching delivery queue");
  try {
    const result = await invoke<DeliveryQueueItemRaw[]>("get_delivery_queue");
    logger.debug("Delivery queue fetched", { count: result.length });
    return result;
  } catch (error) {
    logger.error("Failed to fetch delivery queue", { error });
    throw error;
  }
}

function normalizeDeliveryItem(item: DeliveryQueueItemRaw): DeliveryItem {
  return {
    id: item.id,
    eventType: item.event_type,
    status: item.status,
    retryCount: item.retry_count,
    lastError: item.last_error,
    createdAt: item.created_at,
    deliveredAt: item.delivered_at,
  };
}

function countQueue(items: DeliveryQueueItemRaw[]): DeliveryQueueCounts {
  const counts: DeliveryQueueCounts = {
    total: items.length,
    delivered: 0,
    pending: 0,
    inFlight: 0,
    failed: 0,
    dlq: 0,
    targetPaused: 0,
  };

  for (const item of items) {
    if (item.status === "in_flight") counts.inFlight += 1;
    else if (item.status === "delivered") counts.delivered += 1;
    else if (item.status === "failed") counts.failed += 1;
    else if (item.status === "dlq") counts.dlq += 1;
    else if (item.status === "target_paused") counts.targetPaused += 1;
    else counts.pending += 1;
  }

  return counts;
}

export function useDeliveryQueueRaw() {
  return useQuery({
    queryKey: DELIVERY_QUEUE_QUERY_KEY,
    queryFn: fetchDeliveryQueue,
    refetchInterval: () => visibleRefetchInterval(5000),
  });
}

export function useDeliveryQueue() {
  return useQuery({
    queryKey: DELIVERY_QUEUE_QUERY_KEY,
    queryFn: fetchDeliveryQueue,
    select: (items) => items.map(normalizeDeliveryItem),
    refetchInterval: () => visibleRefetchInterval(5000),
  });
}

export function useDeliveryQueueCounts() {
  return useQuery({
    queryKey: DELIVERY_QUEUE_QUERY_KEY,
    queryFn: fetchDeliveryQueue,
    select: countQueue,
    refetchInterval: () => visibleRefetchInterval(5000),
  });
}
