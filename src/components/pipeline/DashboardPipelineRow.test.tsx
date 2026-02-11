import { render, screen, fireEvent } from "@testing-library/react";
import { describe, it, expect, vi } from "vitest";
import { DashboardPipelineRow } from "./DashboardPipelineRow";
import type { SourceData } from "./types";
import type { Binding } from "../../api/hooks/useBindings";

vi.mock("../../api/hooks/useBindings", async () => {
  const actual = await vi.importActual("../../api/hooks/useBindings");
  return { ...actual };
});

const mockSource: SourceData = {
  id: "claude_code_stats",
  name: "Claude Stats",
  description: "Parse Claude Code stats",
  enabled: true,
  last_sync: null,
  watch_path: "/home/user/.claude/stats-cache.json",
};

const mockBinding1: Binding = {
  source_id: "claude_code_stats",
  target_id: "n8n_abc123",
  endpoint_id: "ep_1",
  endpoint_url: "https://n8n.example.com/webhook/test",
  endpoint_name: "Test Webhook",
  created_at: "2026-01-01T00:00:00Z",
  active: true,
  headers_json: null,
  auth_credential_key: null,
  delivery_mode: "on_change",
  schedule_time: null,
  schedule_day: null,
  last_scheduled_at: null,
};

const mockBinding2: Binding = {
  source_id: "claude_code_stats",
  target_id: "ntfy_xyz789",
  endpoint_id: "ep_2",
  endpoint_url: "https://ntfy.example.com/localpush",
  endpoint_name: "Ntfy Push",
  created_at: "2026-01-01T00:00:00Z",
  active: true,
  headers_json: null,
  auth_credential_key: null,
  delivery_mode: "on_change",
  schedule_time: null,
  schedule_day: null,
  last_scheduled_at: null,
};

const defaultHandlers = {
  onAddTarget: vi.fn(),
  onEditBinding: vi.fn(),
  onPushNow: vi.fn(),
  onEnableClick: vi.fn(),
};

describe("DashboardPipelineRow", () => {
  it("renders source and multiple targets for active binding", () => {
    render(
      <DashboardPipelineRow
        source={mockSource}
        category="active"
        bindings={[mockBinding1, mockBinding2]}
        trafficLightStatus="green"
        isPushing={false}
        {...defaultHandlers}
      />
    );

    expect(screen.getByText("Claude Stats")).toBeInTheDocument();
    expect(screen.getByText("Test Webhook")).toBeInTheDocument();
    expect(screen.getByText("Ntfy Push")).toBeInTheDocument();
    expect(screen.getByText("Push Now")).toBeInTheDocument();
  });

  it("renders Add Target placeholder when no bindings", () => {
    render(
      <DashboardPipelineRow
        source={{ ...mockSource, enabled: false }}
        category="available"
        bindings={[]}
        trafficLightStatus="grey"
        isPushing={false}
        {...defaultHandlers}
      />
    );

    expect(screen.getByText("Claude Stats")).toBeInTheDocument();
    expect(screen.getByText("Add Target")).toBeInTheDocument();
    expect(screen.queryByText("Push Now")).not.toBeInTheDocument();
  });

  it("fires onAddTarget when Add Target is clicked", () => {
    const handlers = { ...defaultHandlers, onAddTarget: vi.fn() };
    render(
      <DashboardPipelineRow
        source={{ ...mockSource, enabled: false }}
        category="available"
        bindings={[]}
        trafficLightStatus="grey"
        isPushing={false}
        {...handlers}
      />
    );

    fireEvent.click(screen.getByText("Add Target"));
    expect(handlers.onAddTarget).toHaveBeenCalledWith("claude_code_stats");
  });

  it("fires onPushNow when Push Now is clicked", () => {
    const handlers = { ...defaultHandlers, onPushNow: vi.fn() };
    render(
      <DashboardPipelineRow
        source={mockSource}
        category="active"
        bindings={[mockBinding1]}
        trafficLightStatus="green"
        isPushing={false}
        {...handlers}
      />
    );

    fireEvent.click(screen.getByText("Push Now"));
    expect(handlers.onPushNow).toHaveBeenCalledWith("claude_code_stats");
  });

  it("fires onEditBinding when edit is clicked", () => {
    const handlers = { ...defaultHandlers, onEditBinding: vi.fn() };
    render(
      <DashboardPipelineRow
        source={mockSource}
        category="active"
        bindings={[mockBinding1]}
        trafficLightStatus="green"
        isPushing={false}
        {...handlers}
      />
    );

    fireEvent.click(screen.getByTitle("Edit binding"));
    expect(handlers.onEditBinding).toHaveBeenCalledWith(
      "claude_code_stats",
      "ep_1"
    );
  });
});
