import { useState } from "react";
import { Shield } from "lucide-react";
import { toast } from "sonner";
import { useConnectZapier } from "../api/hooks/useTargets";
import { logger } from "../utils/logger";

interface TargetInfo {
  id: string;
  target_type: string;
}

interface ZapierConnectProps {
  onConnected: (targetInfo: TargetInfo) => void;
}

export function ZapierConnect({ onConnected }: ZapierConnectProps) {
  const [name, setName] = useState("");
  const [webhookUrl, setWebhookUrl] = useState("");
  const connectMutation = useConnectZapier();

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    if (!name.trim() || !webhookUrl.trim()) {
      logger.warn("Zapier connection attempt with missing fields");
      return;
    }

    if (!webhookUrl.startsWith("https://hooks.zapier.com/")) {
      toast.error("Invalid Zapier webhook URL");
      return;
    }

    try {
      const result = await connectMutation.mutateAsync({
        name: name.trim(),
        webhookUrl: webhookUrl.trim(),
      });
      onConnected(result);
      setName("");
      setWebhookUrl("");
    } catch (error) {
      logger.error("Zapier connection failed", { error });
      toast.error("Zapier connection failed");
    }
  };

  const inputClass =
    "w-full px-3 py-2 text-xs border border-border rounded-md bg-bg-primary text-text-primary placeholder:text-text-secondary/50 focus:outline-none focus:border-accent";

  return (
    <form onSubmit={handleSubmit} className="flex flex-col gap-3">
      <div>
        <label
          htmlFor="zapier-name"
          className="block text-[11px] font-medium text-text-secondary uppercase tracking-wide mb-1.5"
        >
          Name
        </label>
        <input
          id="zapier-name"
          type="text"
          className={inputClass}
          placeholder="My Zapier Webhook"
          value={name}
          onChange={(e) => setName(e.target.value)}
          disabled={connectMutation.isPending}
        />
        <div className="text-[11px] text-text-secondary mt-1">
          A friendly name for this webhook
        </div>
      </div>

      <div>
        <label
          htmlFor="zapier-webhook-url"
          className="block text-[11px] font-medium text-text-secondary uppercase tracking-wide mb-1.5"
        >
          Webhook URL
        </label>
        <input
          id="zapier-webhook-url"
          type="url"
          className={inputClass}
          placeholder="https://hooks.zapier.com/hooks/catch/..."
          value={webhookUrl}
          onChange={(e) => setWebhookUrl(e.target.value)}
          disabled={connectMutation.isPending}
        />
        <div className="text-[11px] text-text-secondary mt-1">
          Create a Zap with &quot;Webhooks by Zapier&quot; → &quot;Catch
          Hook&quot;, then paste the URL here
        </div>
      </div>

      <div className="flex gap-2.5 p-3 bg-accent/10 border border-accent/20 rounded-md">
        <Shield size={16} className="text-accent shrink-0 mt-0.5" />
        <p className="text-[11px] text-text-secondary leading-relaxed">
          The webhook URL is self-authenticating. Only share it with trusted
          systems. LocalPush will send JSON payloads to this endpoint.
        </p>
      </div>

      <div className="flex justify-end">
        <button
          type="submit"
          className="text-xs font-medium px-4 py-2 rounded-md bg-accent text-white hover:bg-accent/90 transition-colors disabled:opacity-50"
          disabled={
            connectMutation.isPending || !name.trim() || !webhookUrl.trim()
          }
        >
          {connectMutation.isPending ? "Testing..." : "Test Connection"}
        </button>
      </div>

      {connectMutation.isSuccess && (
        <div className="text-xs text-success bg-success-bg border border-success/30 rounded-md p-2.5">
          Connected! Zapier webhook verified
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
