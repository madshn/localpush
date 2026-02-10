import { describe, it, expect } from "vitest";
import { render, screen, fireEvent } from "../test/utils.tsx";
import { ActivityCard } from "./ActivityCard";
import type { ActivityEntry } from "../api/hooks/useActivityLog";

const mockEntry: ActivityEntry = {
  id: "test-1",
  source: "claude-stats",
  sourceId: "claude_code_stats",
  status: "delivered",
  timestamp: new Date("2026-02-09T10:30:00"),
  deliveredAt: new Date("2026-02-09T10:30:05"),
  retryCount: 0,
  payload: { tokens: 1500, model: "opus" },
  payloadSummary: "tokens: 1500, model: opus",
};

const failedEntry: ActivityEntry = {
  id: "test-2",
  source: "apple-notes",
  sourceId: "apple_notes",
  status: "failed",
  timestamp: new Date("2026-02-09T11:00:00"),
  retryCount: 3,
  error: "Connection timeout",
  payload: { note_id: "abc123" },
  payloadSummary: "note_id: abc123",
};

describe("ActivityCard", () => {
  it("renders source name and status", () => {
    render(<ActivityCard entry={mockEntry} />);
    expect(screen.getByText("claude-stats")).toBeInTheDocument();
    expect(screen.getByText("Delivered")).toBeInTheDocument();
  });

  it("shows payload summary in header row", () => {
    render(<ActivityCard entry={mockEntry} />);
    expect(screen.getByText("tokens: 1500, model: opus")).toBeInTheDocument();
  });

  it("shows error message for failed entries", () => {
    render(<ActivityCard entry={failedEntry} />);
    expect(screen.getByText("apple-notes")).toBeInTheDocument();
    // Error is shown in expanded view
    fireEvent.click(screen.getByText("apple-notes"));
    expect(screen.getByText(/Connection timeout/)).toBeInTheDocument();
  });

  it("expands on click to show details", () => {
    render(<ActivityCard entry={mockEntry} />);

    // Details not visible initially
    expect(screen.queryByText(/Created:/)).not.toBeInTheDocument();

    // Click to expand
    fireEvent.click(screen.getByText("claude-stats"));

    // Details now visible
    expect(screen.getByText(/Created:/)).toBeInTheDocument();
  });

  it("collapses on second click", () => {
    render(<ActivityCard entry={mockEntry} />);

    // Expand
    fireEvent.click(screen.getByText("claude-stats"));
    expect(screen.getByText(/Created:/)).toBeInTheDocument();

    // Collapse — click the summary row
    fireEvent.click(screen.getByText("Delivered"));
    expect(screen.queryByText(/Created:/)).not.toBeInTheDocument();
  });

  it("shows retry button for failed entries when expanded", () => {
    render(<ActivityCard entry={failedEntry} />);

    fireEvent.click(screen.getByText("apple-notes"));
    expect(screen.getByText("Retry")).toBeInTheDocument();
  });

  it("does not show retry button for delivered entries", () => {
    render(<ActivityCard entry={mockEntry} />);

    fireEvent.click(screen.getByText("claude-stats"));
    expect(screen.queryByText("Retry")).not.toBeInTheDocument();
  });

  it("shows replay button for all entries when expanded", () => {
    render(<ActivityCard entry={mockEntry} />);

    fireEvent.click(screen.getByText("claude-stats"));
    expect(screen.getByText("Replay")).toBeInTheDocument();
  });
});
