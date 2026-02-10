import { useState } from "react";
import { Bell } from "lucide-react";
import { toast } from "sonner";
import { useConnectNtfy } from "../api/hooks/useTargets";
import { logger } from "../utils/logger";

interface TargetInfo {
  id: string;
  target_type: string;
}

interface NtfyConnectProps {
  onConnected: (targetInfo: TargetInfo) => void;
}

export function NtfyConnect({ onConnected }: NtfyConnectProps) {
  const [serverUrl, setServerUrl] = useState("");
  const [topic, setTopic] = useState("");
  const [authToken, setAuthToken] = useState("");
  const connectMutation = useConnectNtfy();

  const handleUrlChange = (value: string) => {
    if (value && !value.startsWith("http://") && !value.startsWith("https://")) {
      setServerUrl("https://" + value);
    } else {
      setServerUrl(value);
    }
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    if (!serverUrl.trim()) {
      logger.warn("ntfy connection attempt with missing server URL");
      return;
    }

    try {
      const result = await connectMutation.mutateAsync({
        serverUrl: serverUrl.trim(),
        topic: topic.trim() || undefined,
        authToken: authToken.trim() || undefined,
      });
      onConnected(result);
      setServerUrl("");
      setTopic("");
      setAuthToken("");
    } catch (error) {
      logger.error("ntfy connection failed", { error });
      toast.error("ntfy connection failed");
    }
  };

  const inputClass =
    "w-full px-3 py-2 text-xs border border-border rounded-md bg-bg-primary text-text-primary placeholder:text-text-secondary/50 focus:outline-none focus:border-accent";

  return (
    <form onSubmit={handleSubmit} className="flex flex-col gap-3">
      <div>
        <label
          htmlFor="ntfy-url"
          className="block text-[11px] font-medium text-text-secondary uppercase tracking-wide mb-1.5"
        >
          Server URL
        </label>
        <input
          id="ntfy-url"
          type="url"
          className={inputClass}
          placeholder="https://ntfy.sh"
          value={serverUrl}
          onChange={(e) => handleUrlChange(e.target.value)}
          disabled={connectMutation.isPending}
        />
      </div>

      <div>
        <label
          htmlFor="ntfy-topic"
          className="block text-[11px] font-medium text-text-secondary uppercase tracking-wide mb-1.5"
        >
          Topic Name
        </label>
        <input
          id="ntfy-topic"
          type="text"
          className={inputClass}
          placeholder="localpush-alerts"
          value={topic}
          onChange={(e) => setTopic(e.target.value)}
          disabled={connectMutation.isPending}
        />
      </div>

      <div>
        <label
          htmlFor="ntfy-token"
          className="block text-[11px] font-medium text-text-secondary uppercase tracking-wide mb-1.5"
        >
          Auth Token{" "}
          <span className="normal-case tracking-normal text-text-secondary/60">
            (optional)
          </span>
        </label>
        <input
          id="ntfy-token"
          type="password"
          className={inputClass}
          value={authToken}
          onChange={(e) => setAuthToken(e.target.value)}
          disabled={connectMutation.isPending}
        />
      </div>

      {/* Security coaching box */}
      <div className="flex gap-2.5 p-3 bg-success/10 border border-success/20 rounded-md">
        <Bell size={16} className="text-success shrink-0 mt-0.5" />
        <p className="text-[11px] text-text-secondary leading-relaxed">
          ntfy sends push notifications to your devices. Use a unique topic name
          to avoid conflicts with other users on shared servers.
        </p>
      </div>

      <div className="flex justify-end">
        <button
          type="submit"
          className="text-xs font-medium px-4 py-2 rounded-md bg-accent text-white hover:bg-accent/90 transition-colors disabled:opacity-50"
          disabled={connectMutation.isPending || !serverUrl.trim()}
        >
          {connectMutation.isPending ? "Testing..." : "Test Connection"}
        </button>
      </div>

      {connectMutation.isSuccess && (
        <div className="text-xs text-success bg-success-bg border border-success/30 rounded-md p-2.5">
          Connected! Server healthy
        </div>
      )}

      {connectMutation.isError && (
        <div className="text-xs text-error bg-error-bg border border-error/30 rounded-md p-2.5">
          {connectMutation.error.message || "Connection failed"}
        </div>
      )}
    </form>
  );
}
