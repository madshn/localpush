interface PipelineConnectorProps {
  active: boolean;
  deliveryCount?: number;
  targetCount?: number;
}

export function PipelineConnector({
  active,
  deliveryCount,
  targetCount = 1,
}: PipelineConnectorProps) {
  const height = targetCount > 1 ? targetCount * 44 : 24;
  const midY = height / 2;

  return (
    <div className="flex items-center justify-center relative">
      <svg
        width="100%"
        height={height}
        viewBox={`0 0 100 ${height}`}
        preserveAspectRatio="none"
        className="overflow-visible"
      >
        {targetCount <= 1 ? (
          <>
            {/* Single straight line */}
            <line
              x1="0"
              y1={midY}
              x2="100"
              y2={midY}
              stroke="var(--color-border)"
              strokeWidth="2"
            />
            {active && (
              <line
                x1="0"
                y1={midY}
                x2="100"
                y2={midY}
                stroke="var(--color-accent)"
                strokeWidth="2"
                className="pulse-line"
              />
            )}
            <polygon
              points={`94,${midY - 4} 100,${midY} 94,${midY + 4}`}
              fill={active ? "var(--color-accent)" : "var(--color-border)"}
            />
          </>
        ) : (
          <>
            {/* Fan-out: single line from left to center, then branch to each target */}
            <line
              x1="0"
              y1={midY}
              x2="50"
              y2={midY}
              stroke="var(--color-border)"
              strokeWidth="2"
            />
            {active && (
              <line
                x1="0"
                y1={midY}
                x2="50"
                y2={midY}
                stroke="var(--color-accent)"
                strokeWidth="2"
                className="pulse-line"
              />
            )}
            {Array.from({ length: targetCount }).map((_, i) => {
              const targetY =
                midY + (i - (targetCount - 1) / 2) * 44;
              const lineColor = active
                ? "var(--color-accent)"
                : "var(--color-border)";
              return (
                <g key={i}>
                  <path
                    d={`M 50 ${midY} Q 65 ${midY} 75 ${targetY}`}
                    fill="none"
                    stroke="var(--color-border)"
                    strokeWidth="2"
                  />
                  {active && (
                    <path
                      d={`M 50 ${midY} Q 65 ${midY} 75 ${targetY}`}
                      fill="none"
                      stroke="var(--color-accent)"
                      strokeWidth="2"
                      className="pulse-line"
                    />
                  )}
                  <line
                    x1="75"
                    y1={targetY}
                    x2="100"
                    y2={targetY}
                    stroke="var(--color-border)"
                    strokeWidth="2"
                  />
                  {active && (
                    <line
                      x1="75"
                      y1={targetY}
                      x2="100"
                      y2={targetY}
                      stroke="var(--color-accent)"
                      strokeWidth="2"
                      className="pulse-line"
                    />
                  )}
                  <polygon
                    points={`94,${targetY - 4} 100,${targetY} 94,${targetY + 4}`}
                    fill={lineColor}
                  />
                </g>
              );
            })}
          </>
        )}
      </svg>
      {/* Delivery count badge */}
      {deliveryCount !== undefined && deliveryCount > 0 && (
        <span className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 bg-bg-primary border border-border rounded-full px-1.5 py-0 text-[9px] font-medium text-text-secondary">
          {deliveryCount.toLocaleString()}
        </span>
      )}
    </div>
  );
}
