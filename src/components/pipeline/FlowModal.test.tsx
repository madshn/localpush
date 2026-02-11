import { render, screen } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { describe, it, expect, vi } from "vitest";
import { FlowModal } from "./FlowModal";
import { defaultFlowState } from "./types";
import type { FlowState } from "./types";

const defaultHandlers = {
  previewLoading: false,
  onPreviewEnable: vi.fn(),
  onPreviewRefresh: vi.fn(),
  onEndpointSelect: vi.fn(),
  onDeliveryConfigConfirm: vi.fn(),
  onSecurityConfirm: vi.fn(),
  onCancelFlow: vi.fn(),
  onBackToEndpointPicker: vi.fn(),
  onBackToDeliveryConfig: vi.fn(),
  onUnbind: vi.fn(),
};

function renderWithQuery(ui: React.ReactElement) {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(
    <QueryClientProvider client={queryClient}>{ui}</QueryClientProvider>
  );
}

describe("FlowModal", () => {
  it("returns null when step is idle", () => {
    const { container } = renderWithQuery(
      <FlowModal
        flowState={defaultFlowState("test_source")}
        {...defaultHandlers}
      />
    );

    expect(container.firstChild).toBeNull();
  });

  it("renders TransparencyPreview when step is preview", () => {
    const flowState: FlowState = {
      ...defaultFlowState("test_source"),
      step: "preview",
      preview: {
        title: "Claude Stats",
        summary: "Stats data preview",
        fields: [
          { label: "Tokens", value: "1234", sensitive: false },
        ],
        lastUpdated: "2026-01-01T00:00:00Z",
      },
    };

    renderWithQuery(
      <FlowModal flowState={flowState} {...defaultHandlers} />
    );

    expect(screen.getByText("Claude Stats")).toBeInTheDocument();
  });

  it("renders EndpointPicker when step is pick_endpoint", () => {
    const flowState: FlowState = {
      ...defaultFlowState("test_source"),
      step: "pick_endpoint",
    };

    renderWithQuery(
      <FlowModal flowState={flowState} {...defaultHandlers} />
    );

    expect(screen.getByText("Select Endpoint")).toBeInTheDocument();
  });
});
