import { CheckCircle2, Clock, AlertTriangle, Loader2 } from "lucide-react";

interface StatusIndicatorProps {
  status: "active" | "pending" | "error" | "unknown";
}

const config = {
  active: {
    icon: CheckCircle2,
    label: "All delivered",
    color: "text-success",
  },
  pending: {
    icon: Clock,
    label: "Pending",
    color: "text-warning",
  },
  error: {
    icon: AlertTriangle,
    label: "Error",
    color: "text-error",
  },
  unknown: {
    icon: Loader2,
    label: "Loading...",
    color: "text-text-secondary",
  },
} as const;

export function StatusIndicator({ status }: StatusIndicatorProps) {
  const { icon: Icon, label, color } = config[status];

  return (
    <div className="flex items-center gap-1.5 text-xs text-text-secondary">
      <Icon
        size={14}
        className={`${color} ${status === "unknown" ? "animate-spin" : ""}`}
      />
      <span>{label}</span>
    </div>
  );
}
