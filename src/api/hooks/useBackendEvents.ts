import { useQueryClient } from '@tanstack/react-query';
import { listen } from '@tauri-apps/api/event';
import { useEffect } from 'react';

/**
 * Listens for backend Tauri events and invalidates relevant React Query caches.
 * This replaces aggressive 5s polling with event-driven updates.
 * Mount once in App.tsx.
 */
export function useBackendEvents(): void {
  const queryClient = useQueryClient();

  useEffect(() => {
    const unlisteners = [
      listen('delivery:status-changed', () => {
        queryClient.invalidateQueries({ queryKey: ['deliveryStatus'] });
        queryClient.invalidateQueries({ queryKey: ['deliveryQueue'] });
        queryClient.invalidateQueries({ queryKey: ['sourceStatusCounts'] });
      }),
      listen('source:data-updated', () => {
        queryClient.invalidateQueries({ queryKey: ['sources'] });
      }),
      listen('dlq:changed', () => {
        queryClient.invalidateQueries({ queryKey: ['dlqCount'] });
      }),
      listen('target:health-changed', () => {
        queryClient.invalidateQueries({ queryKey: ['target-health'] });
        // Target degradation pauses deliveries — refresh delivery views too
        queryClient.invalidateQueries({ queryKey: ['deliveryStatus'] });
        queryClient.invalidateQueries({ queryKey: ['deliveryQueue'] });
        queryClient.invalidateQueries({ queryKey: ['sourceStatusCounts'] });
      }),
    ];

    return () => {
      unlisteners.forEach((p) => p.then((u) => u()));
    };
  }, [queryClient]);
}
