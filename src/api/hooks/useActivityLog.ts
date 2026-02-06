import { useQuery } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';
import { logger } from '../../utils/logger';

interface DeliveryQueueItem {
  id: string;
  eventType: string;
  status: string;
  retryCount: number;
  lastError: string | null;
  createdAt: string;
  deliveredAt: string | null;
}

export interface ActivityEntry {
  id: string;
  source: string;
  status: "delivered" | "pending" | "in_flight" | "failed" | "dlq";
  statusCode?: string;
  error?: string;
  timestamp: Date;
  deliveredAt?: Date;
  retryCount: number;
}

const prettifyEventType = (eventType: string): string => {
  const prettyNames: Record<string, string> = {
    'claude_code_stats': 'Claude Stats',
    'claude_code_sessions': 'Claude Sessions',
    'apple_podcasts': 'Apple Podcasts',
    'apple_notes': 'Apple Notes',
    'apple_photos': 'Apple Photos',
  };

  if (prettyNames[eventType]) {
    return prettyNames[eventType];
  }

  // Default: capitalize and replace underscores
  return eventType
    .split('_')
    .map(word => word.charAt(0).toUpperCase() + word.slice(1))
    .join(' ');
};

const transformToActivityEntry = (item: DeliveryQueueItem): ActivityEntry => {
  return {
    id: item.id,
    source: prettifyEventType(item.eventType),
    status: item.status as ActivityEntry['status'],
    statusCode: item.status === 'delivered' ? '200 OK' : undefined,
    error: item.lastError || undefined,
    timestamp: new Date(item.createdAt),
    deliveredAt: item.deliveredAt ? new Date(item.deliveredAt) : undefined,
    retryCount: item.retryCount,
  };
};

export const useActivityLog = () => {
  return useQuery({
    queryKey: ['activityLog'],
    queryFn: async (): Promise<ActivityEntry[]> => {
      try {
        const queue = await invoke<DeliveryQueueItem[]>('get_delivery_queue');
        logger.debug('Fetched delivery queue', { count: queue.length });

        // Transform and sort by createdAt descending (most recent first)
        const entries = queue
          .map(transformToActivityEntry)
          .sort((a, b) => b.timestamp.getTime() - a.timestamp.getTime());

        return entries;
      } catch (error) {
        logger.error('Failed to fetch delivery queue', { error });
        throw error;
      }
    },
    refetchInterval: 5000, // Poll every 5 seconds
  });
};
