interface PipelineConnectorProps {
  active: boolean;
  deliveryCount?: number;
}

export function PipelineConnector({
  active,
  deliveryCount,
}: PipelineConnectorProps) {
  return (
    <div className="flex items-center justify-center relative">
      <svg
        width="100%"
        height="24"
        viewBox="0 0 100 24"
        preserveAspectRatio="none"
        className="overflow-visible"
      >
        {/* Background line */}
        <line
          x1="0"
          y1="12"
          x2="100"
          y2="12"
          stroke="var(--color-border)"
          strokeWidth="2"
        />
        {/* Animated pulse line when active */}
        {active && (
          <line
            x1="0"
            y1="12"
            x2="100"
            y2="12"
            stroke="var(--color-accent)"
            strokeWidth="2"
            className="pulse-line"
          />
        )}
        {/* Arrow head */}
        <polygon
          points="94,8 100,12 94,16"
          fill={active ? "var(--color-accent)" : "var(--color-border)"}
        />
      </svg>
      {/* Delivery count badge */}
      {deliveryCount !== undefined && deliveryCount > 0 && (
        <span className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 bg-bg-primary border border-border rounded-full px-1.5 py-0 text-[9px] font-medium text-text-secondary">
          {deliveryCount}
        </span>
      )}
    </div>
  );
}
