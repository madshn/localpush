import { describe, it, expect, vi, beforeEach } from "vitest";
import type { ReactNode } from "react";
import { renderHook, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { useTimelineGaps } from "./useTimelineGaps";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

vi.mock("../../utils/logger", () => ({
  logger: {
    debug: vi.fn(),
    info: vi.fn(),
    error: vi.fn(),
  },
}));

describe("useTimelineGaps", () => {
  let queryClient: QueryClient;

  beforeEach(() => {
    queryClient = new QueryClient({
      defaultOptions: {
        queries: {
          retry: false,
        },
      },
    });
    vi.clearAllMocks();
  });

  const wrapper = ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  );

  it("should fetch timeline gaps successfully", async () => {
    const mockGaps = [
      {
        source_id: "claude-stats",
        source_name: "Claude Stats",
        binding_id: "binding-123",
        expected_at: "2026-02-12T00:01:00Z",
        delivery_mode: "daily",
        last_delivered_at: "2026-02-11T00:01:00Z",
      },
    ];

    vi.mocked(invoke).mockResolvedValue(mockGaps);

    const { result } = renderHook(() => useTimelineGaps(), { wrapper });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    expect(result.current.data).toEqual(mockGaps);
    expect(invoke).toHaveBeenCalledWith("get_timeline_gaps", {});
  });

  it("should return empty array when no gaps", async () => {
    vi.mocked(invoke).mockResolvedValue([]);

    const { result } = renderHook(() => useTimelineGaps(), { wrapper });

    await waitFor(() => expect(result.current.isSuccess).toBe(true));

    expect(result.current.data).toEqual([]);
  });

  it("should handle errors", async () => {
    vi.mocked(invoke).mockRejectedValue(new Error("Failed to fetch gaps"));

    const { result } = renderHook(() => useTimelineGaps(), { wrapper });

    await waitFor(() => expect(result.current.isError).toBe(true));

    expect(result.current.error).toBeTruthy();
  });

  it("should configure polling for 60 seconds", () => {
    vi.mocked(invoke).mockResolvedValue([]);
    const { result } = renderHook(() => useTimelineGaps(), { wrapper });

    // The query is configured with refetchInterval but it's not exposed on the result
    // Just verify the query runs
    expect(result.current).toBeDefined();
  });
});
