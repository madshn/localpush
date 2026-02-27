import React, { useState } from "react";
import * as Tabs from "@radix-ui/react-tabs";
import { Webhook, Bell, Plus, X, Zap, Cog, Table, Globe } from "lucide-react";
import { toast } from "sonner";
import { signIn } from "@choochmeque/tauri-plugin-google-auth-api";
import { useTargets, useTestTargetConnection, useReauthGoogleSheets } from "../api/hooks/useTargets";
import { N8nConnect } from "./N8nConnect";
import { NtfyConnect } from "./NtfyConnect";
import { MakeConnect } from "./MakeConnect";
import { ZapierConnect } from "./ZapierConnect";
import { GoogleSheetsConnect } from "./GoogleSheetsConnect";
import { CustomConnect } from "./CustomConnect";
import { logger } from "../utils/logger";

const GOOGLE_CLIENT_ID = import.meta.env.VITE_GOOGLE_CLIENT_ID || "";
const GOOGLE_CLIENT_SECRET = import.meta.env.VITE_GOOGLE_CLIENT_SECRET || "";

interface TargetInfo {
  id: string;
  target_type: string;
}

export function TargetSetup() {
  const [testingTargetId, setTestingTargetId] = useState<string | null>(null);
  const [reauthTargetId, setReauthTargetId] = useState<string | null>(null);
  const [showAddForm, setShowAddForm] = useState(false);
  const { data: targets, isLoading } = useTargets();
  const testMutation = useTestTargetConnection();
  const reauthMutation = useReauthGoogleSheets();

  const handleTargetConnected = (targetInfo: TargetInfo) => {
    logger.info("Target connected successfully", {
      targetId: targetInfo.id,
      type: targetInfo.target_type,
    });
    toast.success("Target connected");
    setShowAddForm(false);
  };

  const [failedTargets, setFailedTargets] = useState<Record<string, string>>({});

  const handleTestConnection = async (targetId: string) => {
    setTestingTargetId(targetId);
    setFailedTargets((prev) => { const next = { ...prev }; delete next[targetId]; return next; });
    try {
      await testMutation.mutateAsync(targetId);
      toast.success("Connection test successful");
    } catch (error) {
      const msg = String(error);
      logger.error("Target test failed", { targetId, error });
      const isAuth = msg.includes("Token") || msg.includes("Auth") || msg.includes("401") || msg.includes("403");
      setFailedTargets((prev) => ({
        ...prev,
        [targetId]: isAuth
          ? "Authentication expired. Re-authenticate to restore delivery."
          : `Connection failed: ${msg}`,
      }));
      toast.error(isAuth ? "Authentication expired" : "Connection test failed");
    } finally {
      setTestingTargetId(null);
    }
  };

  const handleReauthenticate = async (targetId: string) => {
    if (!GOOGLE_CLIENT_ID) {
      toast.error("Google OAuth not configured");
      return;
    }

    setReauthTargetId(targetId);
    try {
      const tokens = await signIn({
        clientId: GOOGLE_CLIENT_ID,
        clientSecret: GOOGLE_CLIENT_SECRET,
        scopes: [
          "openid",
          "email",
          "https://www.googleapis.com/auth/drive.readonly",
          "https://www.googleapis.com/auth/spreadsheets",
        ],
        successHtmlResponse:
          "<h1>Re-authenticated!</h1><p>You can close this window and return to LocalPush.</p>",
      });

      if (!tokens.accessToken || !tokens.refreshToken) {
        throw new Error("Missing tokens from Google sign-in");
      }

      const userInfoResp = await fetch(
        "https://www.googleapis.com/oauth2/v3/userinfo",
        { headers: { Authorization: `Bearer ${tokens.accessToken}` } }
      );
      const userInfo = await userInfoResp.json();
      const email = userInfo.email || "unknown@gmail.com";

      await reauthMutation.mutateAsync({
        targetId,
        email,
        accessToken: tokens.accessToken,
        refreshToken: tokens.refreshToken,
        expiresAt: tokens.expiresAt || Math.floor(Date.now() / 1000) + 3600,
        clientId: GOOGLE_CLIENT_ID,
        clientSecret: GOOGLE_CLIENT_SECRET,
      });

      // Clear the error state for this target
      setFailedTargets((prev) => {
        const next = { ...prev };
        delete next[targetId];
        return next;
      });
    } catch (error) {
      logger.error("Google Sheets re-auth failed", { targetId, error });
      toast.error("Re-authentication failed");
    } finally {
      setReauthTargetId(null);
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
              <React.Fragment key={target.id}>
              <div
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
              {failedTargets[target.id] && (
                <div className="mx-3 mb-2 -mt-1 px-3 py-2 bg-error-bg border border-error/20 rounded-md">
                  <p className="text-[10px] text-error">{failedTargets[target.id]}</p>
                  {target.target_type === "google-sheets" && failedTargets[target.id].includes("Authentication") && (
                    <button
                      className="mt-1.5 text-[10px] font-medium px-2.5 py-1 rounded bg-accent text-white hover:bg-accent/90 transition-colors disabled:opacity-50"
                      onClick={() => handleReauthenticate(target.id)}
                      disabled={reauthTargetId === target.id}
                    >
                      {reauthTargetId === target.id ? "Re-authenticating..." : "Re-authenticate with Google"}
                    </button>
                  )}
                </div>
              )}
            </React.Fragment>
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
