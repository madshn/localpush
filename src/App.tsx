import * as Tabs from "@radix-ui/react-tabs";
import { Workflow, Activity, Settings } from "lucide-react";
import { Toaster } from "sonner";
import { useDeliveryStatus } from "./api/hooks/useDeliveryStatus";
import { StatusIndicator } from "./components/StatusIndicator";
import { PipelineView } from "./components/PipelineView";
import { ActivityLog } from "./components/ActivityLog";
import { SettingsPanel } from "./components/SettingsPanel";

function App() {
  const { data: status } = useDeliveryStatus();

  return (
    <div className="min-h-screen flex flex-col bg-bg-primary">
      <header className="flex items-center justify-between px-4 py-3 border-b border-border bg-bg-secondary">
        <h1 className="text-base font-semibold tracking-tight">LocalPush</h1>
        <StatusIndicator status={status?.overall ?? "unknown"} />
      </header>

      <Tabs.Root defaultValue="pipeline" className="flex-1 flex flex-col">
        <Tabs.List className="flex gap-1 px-4 py-2 border-b border-border">
          <Tabs.Trigger value="pipeline" className="tab-trigger">
            <Workflow size={14} />
            Pipeline
          </Tabs.Trigger>
          <Tabs.Trigger value="activity" className="tab-trigger">
            <Activity size={14} />
            Activity
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
