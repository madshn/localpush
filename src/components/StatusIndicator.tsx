interface StatusIndicatorProps {
  status: "active" | "pending" | "error" | "unknown";
}

export function StatusIndicator({ status }: StatusIndicatorProps) {
  const labels: Record<string, string> = {
    active: "All delivered",
    pending: "Pending",
    error: "Error",
    unknown: "Loading...",
  };

  return (
    <div className="status-indicator">
      <span className={`status-dot ${status}`} />
      <span>{labels[status]}</span>
    </div>
  );
}
