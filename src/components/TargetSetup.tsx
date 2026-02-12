import { useState } from "react";
import * as Tabs from "@radix-ui/react-tabs";
import { Webhook, Bell, Plus, X, Zap, Cog, Table, Globe } from "lucide-react";
import { toast } from "sonner";
import { useTargets, useTestTargetConnection } from "../api/hooks/useTargets";
import { N8nConnect } from "./N8nConnect";
import { NtfyConnect } from "./NtfyConnect";
import { MakeConnect } from "./MakeConnect";
import { ZapierConnect } from "./ZapierConnect";
import { GoogleSheetsConnect } from "./GoogleSheetsConnect";
import { CustomConnect } from "./CustomConnect";
import { logger } from "../utils/logger";

interface TargetInfo {
  id: string;
  target_type: string;
}

export function TargetSetup() {
  const [testingTargetId, setTestingTargetId] = useState<string | null>(null);
  const [showAddForm, setShowAddForm] = useState(false);
  const { data: targets, isLoading } = useTargets();
  const testMutation = useTestTargetConnection();

  const handleTargetConnected = (targetInfo: TargetInfo) => {
    logger.info("Target connected successfully", {
      targetId: targetInfo.id,
      type: targetInfo.target_type,
    });
    toast.success("Target connected");
    setShowAddForm(false);
  };

  const handleTestConnection = async (targetId: string) => {
    setTestingTargetId(targetId);
    try {
      await testMutation.mutateAsync(targetId);
      toast.success("Connection test successful");
    } catch (error) {
      logger.error("Target test failed", { targetId, error });
      toast.error("Connection test failed");
    } finally {
      setTestingTargetId(null);
    }
  };

  return (
    <div className="flex flex-col gap-3">
      {/* Connected Targets */}
      {!isLoading && targets && targets.length > 0 && (
        <div className="bg-bg-secondary border border-border rounded-lg p-4">
          <div className="flex items-center justify-between mb-3">
            <h2 className="text-sm font-semibold">Connected Targets</h2>
            <span className="text-[10px] font-medium text-accent">
              {targets.length} Active
            </span>
          </div>
          <div className="flex flex-col gap-2">
            {targets.map((target) => (
              <div
                key={target.id}
                className="flex items-center gap-3 p-3 bg-bg-primary rounded-md border-l-2 border-l-accent"
              >
                {target.target_type === "n8n" ? (
                  <Webhook size={16} className="text-accent shrink-0" />
                ) : target.target_type === "ntfy" ? (
                  <Bell size={16} className="text-success shrink-0" />
                ) : target.target_type === "make" ? (
                  <Cog size={16} className="text-purple-500 shrink-0" />
                ) : target.target_type === "google-sheets" ? (
                  <Table size={16} className="text-green-600 shrink-0" />
                ) : target.target_type === "custom" ? (
                  <Globe size={16} className="text-blue-500 shrink-0" />
                ) : (
                  <Zap size={16} className="text-orange-500 shrink-0" />
                )}
                <div className="flex-1 min-w-0">
                  <div className="text-xs font-medium truncate">
                    {target.name}
                  </div>
                  <span
                    className={`text-[10px] font-medium px-1.5 py-0.5 rounded ${
                      target.target_type === "n8n"
                        ? "bg-accent-muted text-accent"
                        : "bg-success-bg text-success"
                    }`}
                  >
                    {target.target_type}
                  </span>
                </div>
                <button
                  className="text-xs font-medium px-2.5 py-1 rounded bg-bg-tertiary text-text-secondary border border-border hover:border-border-hover transition-colors disabled:opacity-50"
                  onClick={() => handleTestConnection(target.id)}
                  disabled={testingTargetId === target.id}
                >
                  {testingTargetId === target.id ? "Testing..." : "Test"}
                </button>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Add New Target */}
      {showAddForm ? (
        <div className="bg-bg-secondary border border-border rounded-lg p-4">
          <div className="flex items-center justify-between mb-3">
            <h2 className="text-sm font-semibold">Add New Target</h2>
            <button
              onClick={() => setShowAddForm(false)}
              className="text-text-secondary hover:text-text-primary transition-colors"
            >
              <X size={14} />
            </button>
          </div>

          <Tabs.Root defaultValue="n8n">
            <Tabs.List className="flex gap-1 mb-4 bg-bg-primary rounded-lg p-1">
              <Tabs.Trigger value="n8n" className="tab-trigger">
                <Webhook size={12} className="shrink-0" />
                n8n
              </Tabs.Trigger>
              <Tabs.Trigger value="ntfy" className="tab-trigger">
                <Bell size={12} className="shrink-0" />
                ntfy.app
              </Tabs.Trigger>
              <Tabs.Trigger value="make" className="tab-trigger">
                <Cog size={12} className="shrink-0" />
                Make
              </Tabs.Trigger>
              <Tabs.Trigger value="zapier" className="tab-trigger">
                <Zap size={12} className="shrink-0" />
                Zapier
              </Tabs.Trigger>
              <Tabs.Trigger value="google-sheets" className="tab-trigger">
                <Table size={12} className="shrink-0" />
                Sheets
              </Tabs.Trigger>
              <Tabs.Trigger value="custom" className="tab-trigger">
                <Globe size={12} className="shrink-0" />
                Custom
              </Tabs.Trigger>
            </Tabs.List>

            <Tabs.Content value="n8n">
              <N8nConnect onConnected={handleTargetConnected} />
            </Tabs.Content>
            <Tabs.Content value="ntfy">
              <NtfyConnect onConnected={handleTargetConnected} />
            </Tabs.Content>
            <Tabs.Content value="make">
              <MakeConnect onConnected={handleTargetConnected} />
            </Tabs.Content>
            <Tabs.Content value="zapier">
              <ZapierConnect onConnected={handleTargetConnected} />
            </Tabs.Content>
            <Tabs.Content value="google-sheets">
              <GoogleSheetsConnect onConnected={handleTargetConnected} />
            </Tabs.Content>
            <Tabs.Content value="custom">
              <CustomConnect onConnected={handleTargetConnected} />
            </Tabs.Content>
          </Tabs.Root>
        </div>
      ) : (
        <button
          onClick={() => setShowAddForm(true)}
          className="flex items-center justify-center gap-2 w-full py-2.5 text-xs font-medium rounded-lg border border-dashed border-border text-text-secondary hover:text-accent hover:border-accent transition-colors"
        >
          <Plus size={14} />
          Add New Target
        </button>
      )}
    </div>
  );
}
