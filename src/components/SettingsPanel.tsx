import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Info } from "lucide-react";
import { TargetSetup } from "./TargetSetup";

interface AppInfo {
  version: string;
  build_profile: string;
}

export function SettingsPanel() {
  const [autoUpdate, setAutoUpdate] = useState(true);
  const [appInfo, setAppInfo] = useState<AppInfo | null>(null);

  useEffect(() => {
    const loadSettings = async () => {
      try {
        const autoUpdateSetting = await invoke<string | null>("get_setting", {
          key: "auto_update",
        });
        setAutoUpdate(autoUpdateSetting !== "false");
      } catch (error) {
        console.error("Failed to load auto-update setting:", error);
      }
    };
    const loadAppInfo = async () => {
      try {
        const info = await invoke<AppInfo>("get_app_info");
        setAppInfo(info);
      } catch (error) {
        console.error("Failed to load app info:", error);
      }
    };
    loadSettings();
    loadAppInfo();
  }, []);

  const handleAutoUpdateChange = async (checked: boolean) => {
    setAutoUpdate(checked);
    try {
      await invoke("set_setting", {
        key: "auto_update",
        value: checked ? "true" : "false",
      });
    } catch (error) {
      console.error("Failed to save auto-update setting:", error);
    }
  };

  return (
    <div className="flex flex-col gap-3">
      <TargetSetup />

      <div className="bg-bg-secondary border border-border rounded-lg p-4">
        <h2 className="text-sm font-semibold mb-3">General</h2>
        <label className="flex items-center gap-2.5 text-xs text-text-secondary cursor-pointer select-none">
          <input
            type="checkbox"
            checked={autoUpdate}
            onChange={(e) => handleAutoUpdateChange(e.target.checked)}
            className="w-4 h-4 rounded cursor-pointer accent-accent"
          />
          <span>Automatically check for app updates on startup</span>
        </label>
      </div>

      {/* About */}
      <div className="bg-bg-secondary border border-border rounded-lg p-4">
        <div className="flex items-center gap-2 mb-3">
          <Info size={14} className="text-text-secondary" />
          <h2 className="text-sm font-semibold">About</h2>
        </div>
        <div className="text-xs text-text-secondary space-y-1">
          <div className="flex justify-between">
            <span>Version</span>
            <span className="font-mono text-text-primary">
              {appInfo?.version ?? "..."}
            </span>
          </div>
          <div className="flex justify-between">
            <span>Build</span>
            <span className="font-mono text-text-primary">
              {appInfo?.build_profile ?? "..."}
            </span>
          </div>
          <div className="pt-2 mt-2 border-t border-border">
            <a
              href="https://github.com/madshn/localpush/issues"
              target="_blank"
              rel="noopener noreferrer"
              className="text-accent hover:underline"
            >
              Report an issue
            </a>
          </div>
        </div>
      </div>
    </div>
  );
}
