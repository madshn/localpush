import { Zap, CheckCircle2 } from "lucide-react";
import { useDeliveryStatus } from "../api/hooks/useDeliveryStatus";
import { useDeliveryQueueCounts } from "../api/hooks/useDeliveryQueue";
import { SparklineChart } from "./SparklineChart";

export function SummaryStats() {
  const { data: status } = useDeliveryStatus();
  const { data: queueCounts } = useDeliveryQueueCounts();

  const deliveredCount = queueCounts?.delivered ?? 0;
  const pendingCount = status?.pendingCount ?? 0;

  const healthLabel =
    (status?.failedCount ?? 0) > 0
      ? "Degraded"
      : pendingCount > 0
        ? "Pending"
        : "Operational";

  const healthColor =
    (status?.failedCount ?? 0) > 0
      ? "text-error"
      : pendingCount > 0
        ? "text-warning"
        : "text-success";

  // Synthetic sparkline data (delivery counts aren't tracked historically yet)
  const sparkData = [3, 5, 4, 7, 6, 8, deliveredCount || 5];

  return (
    <div className="grid grid-cols-2 gap-3 mb-4">
      {/* Total Deliveries */}
      <div className="bg-bg-secondary border border-border rounded-lg p-3">
        <div className="flex items-center gap-1.5 mb-2">
          <span className="text-[11px] font-medium text-text-secondary uppercase tracking-wide">
            Total Deliveries
          </span>
          <Zap size={12} className="text-accent" />
        </div>
        <div className="flex items-end justify-between">
          <span className="text-2xl font-semibold">{deliveredCount}</span>
          <SparklineChart data={sparkData} />
        </div>
      </div>

      {/* System Status */}
      <div className="bg-bg-secondary border border-border rounded-lg p-3">
        <div className="flex items-center gap-1.5 mb-2">
          <span className="text-[11px] font-medium text-text-secondary uppercase tracking-wide">
            System Status
          </span>
          <CheckCircle2 size={12} className={healthColor} />
        </div>
        <div className={`text-lg font-semibold ${healthColor}`}>
          {healthLabel}
        </div>
        {pendingCount > 0 && (
          <div className="text-[11px] text-text-secondary mt-0.5">
            {pendingCount} pending
          </div>
        )}
      </div>
    </div>
  );
}
