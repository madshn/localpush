import { useDeliveryStatus } from "../api/hooks/useDeliveryStatus";
import { StatusIndicator } from "./StatusIndicator";
import { PipelineView } from "./PipelineView";
import { ActivityLog } from "./ActivityLog";

export function DashboardView() {
  const { data: status } = useDeliveryStatus();

  return (
    <div className="min-h-screen flex flex-col bg-bg-primary">
      <header className="flex items-center justify-between px-6 py-3 border-b border-border bg-bg-secondary">
        <h1 className="text-base font-semibold tracking-tight">
          LocalPush Dashboard
        </h1>
        <StatusIndicator status={status?.overall ?? "unknown"} />
      </header>

      <div className="flex-1 flex overflow-hidden">
        {/* Left: Pipeline view */}
        <div className="flex-1 overflow-y-auto p-4">
          <PipelineView />
        </div>

        {/* Divider */}
        <div className="w-px bg-border" />

        {/* Right: Activity log */}
        <div className="w-96 overflow-y-auto p-4">
          <h2 className="text-sm font-semibold mb-3">Activity Log</h2>
          <ActivityLog />
        </div>
      </div>
    </div>
  );
}
