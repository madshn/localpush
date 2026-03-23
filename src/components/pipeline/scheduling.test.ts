import { describe, expect, it } from "vitest";

import type { Binding } from "../../api/hooks/useBindings";
import { formatNextPushLabel, getNextPushBySource } from "./scheduling";

function makeBinding(overrides: Partial<Binding>): Binding {
  return {
    source_id: "source-0",
    target_id: "target-0",
    endpoint_id: "endpoint-0",
    endpoint_url: "https://example.com",
    endpoint_name: "Example",
    created_at: "1000",
    active: true,
    headers_json: null,
    auth_credential_key: null,
    delivery_mode: "interval",
    schedule_time: "10",
    schedule_day: null,
    last_scheduled_at: null,
    ...overrides,
  };
}

describe("scheduling", () => {
  it("spreads ten interval bindings across ten minutes", () => {
    const nowMs = Date.UTC(2026, 2, 18, 21, 20, 0);
    const bindings = Array.from({ length: 10 }, (_, index) =>
      makeBinding({
        source_id: `source-${index}`,
        endpoint_id: `endpoint-${index}`,
        created_at: String(index),
      })
    );

    const nextPushBySource = getNextPushBySource(bindings, nowMs);
    const offsets = bindings
      .map((binding) => {
        const nextPush = nextPushBySource.get(binding.source_id);
        expect(nextPush).toBeDefined();
        return ((nextPush as number) - nowMs) / 60_000;
      })
      .sort((a, b) => a - b);

    expect(offsets).toEqual([0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
  });

  it("shows a due-now label for imminent pushes", () => {
    const nowMs = Date.UTC(2026, 2, 18, 21, 20, 0);
    expect(formatNextPushLabel(nowMs, nowMs)).toBe("Next push due now");
    expect(formatNextPushLabel(nowMs + 2 * 60_000, nowMs)).toBe("Next push in 2m");
  });
});
