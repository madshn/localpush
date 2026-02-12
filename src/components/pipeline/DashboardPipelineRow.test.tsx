import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { DashboardPipelineRow } from "./DashboardPipelineRow";
import type { SourceData } from "./types";
import type { TimelineGap } from "../../api/hooks/useTimelineGaps";

describe("DashboardPipelineRow", () => {
  const mockSource: SourceData = {
    id: "claude-stats",
    name: "Claude Stats",
    description: "Parse Claude stats",
    enabled: true,
    last_sync: null,
    watch_path: null,
  };

  const mockHandlers = {
    onAddTarget: vi.fn(),
    onEditBinding: vi.fn(),
    onPushNow: vi.fn(),
    onEnableClick: vi.fn(),
    onViewActivity: vi.fn(),
  };

  it("should render without gap indicator when gap is null", () => {
    render(
      <DashboardPipelineRow
        source={mockSource}
        category="active"
        bindings={[]}
        gap={null}
        trafficLightStatus="green"
        isPushing={false}
        {...mockHandlers}
      />
    );

    expect(screen.queryByText(/Missing:/)).not.toBeInTheDocument();
  });

  it("should show gap indicator when gap exists", () => {
    const mockGap: TimelineGap = {
      source_id: "claude-stats",
      source_name: "Claude Stats",
      binding_id: "binding-123",
      expected_at: "2026-02-12T00:01:00Z",
      delivery_mode: "daily",
      last_delivered_at: "2026-02-11T00:01:00Z",
    };

    render(
      <DashboardPipelineRow
        source={mockSource}
        category="active"
        bindings={[]}
        gap={mockGap}
        trafficLightStatus="yellow"
        isPushing={false}
        {...mockHandlers}
      />
    );

    expect(screen.getByText(/Missing: daily delivery/i)).toBeInTheDocument();
  });

  it("should format gap date correctly", () => {
    const mockGap: TimelineGap = {
      source_id: "claude-stats",
      source_name: "Claude Stats",
      binding_id: "binding-123",
      expected_at: "2026-02-12T00:01:00Z",
      delivery_mode: "daily",
      last_delivered_at: null,
    };

    render(
      <DashboardPipelineRow
        source={mockSource}
        category="active"
        bindings={[]}
        gap={mockGap}
        trafficLightStatus="yellow"
        isPushing={false}
        {...mockHandlers}
      />
    );

    expect(screen.getByText(/Missing: daily delivery for/i)).toBeInTheDocument();
  });

  it("should show last delivered date when available", () => {
    const mockGap: TimelineGap = {
      source_id: "claude-stats",
      source_name: "Claude Stats",
      binding_id: "binding-123",
      expected_at: "2026-02-12T00:01:00Z",
      delivery_mode: "weekly",
      last_delivered_at: "2026-02-05T00:01:00Z",
    };

    render(
      <DashboardPipelineRow
        source={mockSource}
        category="active"
        bindings={[]}
        gap={mockGap}
        trafficLightStatus="yellow"
        isPushing={false}
        {...mockHandlers}
      />
    );

    expect(screen.getByText(/last delivered/i)).toBeInTheDocument();
  });

  it("should render view button when onViewActivity is provided", () => {
    const mockGap: TimelineGap = {
      source_id: "claude-stats",
      source_name: "Claude Stats",
      binding_id: "binding-123",
      expected_at: "2026-02-12T00:01:00Z",
      delivery_mode: "daily",
      last_delivered_at: null,
    };

    render(
      <DashboardPipelineRow
        source={mockSource}
        category="active"
        bindings={[]}
        gap={mockGap}
        trafficLightStatus="yellow"
        isPushing={false}
        {...mockHandlers}
      />
    );

    expect(screen.getByText("View")).toBeInTheDocument();
  });
});
