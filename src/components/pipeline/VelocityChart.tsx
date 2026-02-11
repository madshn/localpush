import { useState } from "react";

type Period = "7d" | "30d" | "90d";

const barHeights: Record<Period, number[]> = {
  "7d": [40, 65, 55, 80, 70, 90, 60, 45, 75, 50, 85, 70],
  "30d": [30, 50, 45, 70, 60, 80, 55, 40, 65, 50, 75, 60],
  "90d": [25, 40, 35, 55, 50, 70, 45, 35, 55, 40, 65, 50],
};

export function VelocityChart() {
  const [period, setPeriod] = useState<Period>("7d");
  const bars = barHeights[period];

  return (
    <div className="bg-bg-secondary border border-border rounded-lg p-3">
      <div className="flex items-center justify-between mb-3">
        <span className="text-[11px] font-medium text-text-secondary uppercase tracking-wide">
          Delivery Velocity
        </span>
        <div className="flex gap-0.5 bg-bg-primary rounded-md p-0.5">
          {(["7d", "30d", "90d"] as Period[]).map((p) => (
            <button
              key={p}
              onClick={() => setPeriod(p)}
              className={`px-2 py-0.5 text-[10px] font-medium rounded transition-colors ${
                period === p
                  ? "bg-accent text-white"
                  : "text-text-secondary hover:text-text-primary"
              }`}
            >
              {p.toUpperCase()}
            </button>
          ))}
        </div>
      </div>

      <div className="flex items-end gap-1 h-16">
        {bars.map((h, i) => (
          <div
            key={i}
            className="flex-1 bg-accent/20 rounded-sm transition-all duration-300"
            style={{ height: `${h}%` }}
          >
            <div
              className="w-full bg-accent rounded-sm transition-all duration-300"
              style={{ height: "100%", opacity: 0.6 + (h / 100) * 0.4 }}
            />
          </div>
        ))}
      </div>
    </div>
  );
}
