import { useQuery } from '@tanstack/react-query';
import {
  DELIVERY_QUEUE_QUERY_KEY,
  fetchDeliveryQueue,
  type DeliveryQueueItemRaw,
} from './useDeliveryQueue';
import { visibleRefetchInterval } from './polling';

export interface DeliveredToInfo {
  endpoint_id: string;
  endpoint_name: string;
  target_type: string;
  target_url?: string;
}

export interface ActivityEntry {
  id: string;
  source: string;
  sourceId: string;
  status: "delivered" | "pending" | "in_flight" | "failed" | "dlq";
  statusCode?: string;
  error?: string;
  timestamp: Date;
  deliveredAt?: Date;
  retryCount: number;
  payload: unknown;
  payloadSummary: string;
  triggerType: "file_change" | "manual" | "scheduled";
  deliveredTo: DeliveredToInfo | null;
}

const prettifyEventType = (eventType: string): string => {
  const prettyNames: Record<string, string> = {
    'claude-stats': 'Claude Stats',
    'claude-sessions': 'Claude Sessions',
    'apple-podcasts': 'Apple Podcasts',
    'apple-notes': 'Apple Notes',
    'apple-photos': 'Apple Photos',
  };

  if (prettyNames[eventType]) {
    return prettyNames[eventType];
  }

  // Default: capitalize and replace hyphens/underscores
  return eventType
    .split(/[-_]/)
    .map(word => word.charAt(0).toUpperCase() + word.slice(1))
    .join(' ');
};

const summarizePayload = (payload: unknown): string => {
  if (!payload || typeof payload !== 'object') return '';
  const obj = payload as Record<string, unknown>;
  const keys = Object.keys(obj);
  if (keys.length === 0) return '';
  // Show first 2-3 meaningful key-value pairs
  const summary = keys.slice(0, 3).map(k => {
    const v = obj[k];
    if (typeof v === 'string') return `${k}: ${v.slice(0, 30)}${v.length > 30 ? '...' : ''}`;
    if (typeof v === 'number') return `${k}: ${v}`;
    if (Array.isArray(v)) return `${k}: [${v.length} items]`;
    if (typeof v === 'object' && v !== null) return `${k}: {...}`;
    return `${k}: ${String(v)}`;
  }).join(', ');
  const extra = keys.length > 3 ? ` +${keys.length - 3} more` : '';
  return summary + extra;
};

const parseDeliveredTo = (raw: string | null): DeliveredToInfo | null => {
  if (!raw) return null;
  try {
    const parsed = JSON.parse(raw);
    if (parsed.endpoint_id && parsed.target_type) return parsed as DeliveredToInfo;
    return null;
  } catch {
    return null;
  }
};

const transformToActivityEntry = (item: DeliveryQueueItemRaw): ActivityEntry => {
  return {
    id: item.id,
    source: prettifyEventType(item.event_type),
    sourceId: item.event_type,
    status: item.status as ActivityEntry['status'],
    statusCode: item.status === 'delivered' ? '200 OK' : undefined,
    error: item.last_error || undefined,
    timestamp: new Date(item.created_at),
    deliveredAt: item.delivered_at ? new Date(item.delivered_at) : undefined,
    retryCount: item.retry_count,
    payload: item.payload,
    payloadSummary: summarizePayload(item.payload),
    triggerType: (item.trigger_type as ActivityEntry['triggerType']) || 'file_change',
    deliveredTo: parseDeliveredTo(item.delivered_to),
  };
};

function sortByCreatedAtDesc<T extends { created_at: string }>(items: T[]): T[] {
  return [...items].sort((a, b) => b.created_at.localeCompare(a.created_at));
}

export const useActivityLog = () => {
  return useQuery({
    queryKey: DELIVERY_QUEUE_QUERY_KEY,
    queryFn: fetchDeliveryQueue,
    select: (queue): ActivityEntry[] =>
      sortByCreatedAtDesc(queue).map(transformToActivityEntry),
    refetchInterval: () => visibleRefetchInterval(5000), // Poll every 5 seconds (visible only)
  });
};

export const useRecentActivityLog = (limit = 3) => {
  return useQuery({
    queryKey: DELIVERY_QUEUE_QUERY_KEY,
    queryFn: fetchDeliveryQueue,
    select: (queue): ActivityEntry[] =>
      sortByCreatedAtDesc(queue).slice(0, limit).map(transformToActivityEntry),
    refetchInterval: () => visibleRefetchInterval(5000),
  });
};
