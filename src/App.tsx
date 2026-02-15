import { useState } from "react";
import * as Tabs from "@radix-ui/react-tabs";
import { Workflow, Activity, Settings, ExternalLink, AlertTriangle } from "lucide-react";
import { Toaster, toast } from "sonner";


import { useDeliveryStatus } from "./api/hooks/useDeliveryStatus";
import { useDlqCount } from "./api/hooks/useErrorDiagnosis";
import { StatusIndicator } from "./components/StatusIndicator";
import { PipelineView } from "./components/PipelineView";
import { ActivityLog } from "./components/ActivityLog";
import { SettingsPanel } from "./components/SettingsPanel";
import { DashboardView } from "./components/DashboardView";
import { logger } from "./utils/logger";

const isDashboard = new URLSearchParams(window.location.search).has(
  "view",
  "dashboard"
);

async function handleOpenDashboard() {
  try {
    const { WebviewWindow } = await import("@tauri-apps/api/webviewWindow");
    const existing = await WebviewWindow.getByLabel("dashboard");
    if (existing) {
      await existing.setFocus();
      logger.info("Dashboard window focused");
      return;
    }
    const dashboard = new WebviewWindow("dashboard", {
      url: "/?view=dashboard",
      title: "LocalPush Dashboard",
      width: 1100,
      height: 700,
      minWidth: 700,
      minHeight: 500,
      center: true,
      decorations: true,
      resizable: true,
    });
    dashboard.once("tauri://created", () => {
      logger.info("Dashboard window created");
    });
    dashboard.once("tauri://error", (e) => {
      logger.error("Dashboard window error", { error: e.payload });
      toast.error("Failed to open dashboard window");
    });
  } catch (error) {
    logger.error("Failed to open dashboard", { error });
    toast.error(`Dashboard error: ${error}`);
  }
}

function App() {
  const { data: status } = useDeliveryStatus();
  const { data: dlqCount } = useDlqCount();
  const [activeTab, setActiveTab] = useState("pipeline");

  // DLQ state is polled via useDlqCount hook (5s interval).
  // Backend notifies via macOS native notification on DLQ transitions.

  if (isDashboard) {
    return (
      <>
        <DashboardView />
        <Toaster
          theme="dark"
          position="bottom-center"
          toastOptions={{
            style: {
              background: "var(--color-bg-secondary)",
              border: "1px solid var(--color-border)",
              color: "var(--color-text-primary)",
            },
          }}
        />
      </>
    );
  }

  return (
    <div className="min-h-screen flex flex-col bg-bg-primary">
      <header className="flex items-center justify-between px-4 py-3 border-b border-border bg-bg-secondary">
        <h1 className="text-base font-semibold tracking-tight">LocalPush</h1>
        <div className="flex items-center gap-2">
          <StatusIndicator status={status?.overall ?? "unknown"} />
        </div>
      </header>

      {/* DLQ failure banner */}
      {dlqCount != null && dlqCount > 0 && (
        <div
          onClick={() => setActiveTab("activity")}
          className="mx-4 mt-3 px-3 py-2 bg-error-bg border border-error/20 rounded-lg cursor-pointer hover:bg-error-bg/80 transition-colors"
        >
          <div className="flex items-center gap-2">
            <AlertTriangle size={14} className="text-error shrink-0" />
            <div className="flex-1 min-w-0">
              <p className="text-xs font-medium text-error">
                {dlqCount} {dlqCount === 1 ? "delivery" : "deliveries"} need attention
              </p>
            </div>
            <span className="text-[10px] text-error/80 font-medium">View →</span>
          </div>
        </div>
      )}

      <Tabs.Root value={activeTab} onValueChange={setActiveTab} className="flex-1 flex flex-col min-h-0">
        <Tabs.List className="flex gap-1 px-4 py-2 border-b border-border">
          <Tabs.Trigger value="pipeline" className="tab-trigger">
            <Workflow size={14} />
            Pipeline
          </Tabs.Trigger>
          <Tabs.Trigger value="activity" className="tab-trigger relative">
            <Activity size={14} />
            Activity
            {dlqCount != null && dlqCount > 0 && (
              <span className="absolute -top-1 -right-1 min-w-[16px] h-4 px-1 flex items-center justify-center bg-error text-[9px] font-semibold text-white rounded-full">
                {dlqCount}
              </span>
            )}
          </Tabs.Trigger>
          <Tabs.Trigger value="settings" className="tab-trigger">
            <Settings size={14} />
            Settings
          </Tabs.Trigger>
        </Tabs.List>

        <Tabs.Content value="pipeline" className="flex-1 overflow-y-auto p-4">
          <PipelineView />
        </Tabs.Content>
        <Tabs.Content value="activity" className="flex-1 overflow-y-auto p-4">
          <ActivityLog />
        </Tabs.Content>
        <Tabs.Content value="settings" className="flex-1 overflow-y-auto p-4">
          <SettingsPanel />
        </Tabs.Content>
      </Tabs.Root>

      {/* Dashboard CTA */}
      <button
        onClick={handleOpenDashboard}
        className="mx-4 mb-3 mt-1 flex items-center justify-center gap-2 px-4 py-2.5 rounded-lg bg-accent/10 border border-accent/20 text-accent hover:bg-accent/20 transition-colors text-xs font-medium"
      >
        <ExternalLink size={14} />
        Open Full Dashboard
      </button>

      {/* Resize grip indicator */}
      <div className="resize-grip" />

      <Toaster
        theme="dark"
        position="bottom-center"
        toastOptions={{
          style: {
            background: "var(--color-bg-secondary)",
            border: "1px solid var(--color-border)",
            color: "var(--color-text-primary)",
          },
        }}
      />
    </div>
  );
}

export default App;
