import { beforeEach, describe, expect, it, vi } from 'vitest';
import {
  mockDeliveryStatusActive,
  mockDeliveryStatusError,
  mockDeliveryStatusPending,
} from '../../test/mocks';
import { mockInvoke } from '../../test/setup';
import { renderHook, waitFor } from '../../test/utils.tsx';
import { useDeliveryStatus } from './useDeliveryStatus';

describe('useDeliveryStatus', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('fetches delivery status successfully', async () => {
    mockInvoke.mockResolvedValue(mockDeliveryStatusActive);

    const { result } = renderHook(() => useDeliveryStatus());

    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true);
    });

    expect(result.current.data).toEqual(mockDeliveryStatusActive);
    expect(mockInvoke).toHaveBeenCalledWith('get_delivery_status');
  });

  it('handles pending status correctly', async () => {
    mockInvoke.mockResolvedValue(mockDeliveryStatusPending);

    const { result } = renderHook(() => useDeliveryStatus());

    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true);
    });

    expect(result.current.data).toEqual(mockDeliveryStatusPending);
    expect(result.current.data?.pendingCount).toBe(3);
    expect(result.current.data?.failedCount).toBe(0);
  });

  it('handles error status correctly', async () => {
    mockInvoke.mockResolvedValue(mockDeliveryStatusError);

    const { result } = renderHook(() => useDeliveryStatus());

    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true);
    });

    expect(result.current.data?.overall).toBe('error');
    expect(result.current.data?.failedCount).toBe(2);
  });

  it('handles Tauri IPC errors gracefully', async () => {
    const error = new Error('IPC communication failed');
    mockInvoke.mockRejectedValue(error);

    const { result } = renderHook(() => useDeliveryStatus());

    await waitFor(() => {
      expect(result.current.isError).toBe(true);
    });

    expect(result.current.error).toEqual(error);
  });

  it('calls invoke with correct command name', async () => {
    mockInvoke.mockResolvedValue(mockDeliveryStatusActive);

    const { result } = renderHook(() => useDeliveryStatus());

    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true);
    });

    // Verify the correct IPC command was called
    expect(mockInvoke).toHaveBeenCalledWith('get_delivery_status');
  });
});
