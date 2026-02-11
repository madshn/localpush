import { useState } from "react";
import { Shield } from "lucide-react";
import { toast } from "sonner";
import { useConnectMake } from "../api/hooks/useTargets";
import { logger } from "../utils/logger";

interface TargetInfo {
  id: string;
  target_type: string;
}

interface MakeConnectProps {
  onConnected: (targetInfo: TargetInfo) => void;
}

export function MakeConnect({ onConnected }: MakeConnectProps) {
  const [zoneUrl, setZoneUrl] = useState("");
  const [apiToken, setApiToken] = useState("");
  const connectMutation = useConnectMake();

  const handleUrlChange = (value: string) => {
    if (value && !value.startsWith("http://") && !value.startsWith("https://")) {
      setZoneUrl("https://" + value);
    } else {
      setZoneUrl(value);
    }
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    if (!zoneUrl.trim() || !apiToken.trim()) {
      logger.warn("Make.com connection attempt with missing fields");
      return;
    }

    try {
      const result = await connectMutation.mutateAsync({
        zoneUrl: zoneUrl.trim(),
        apiKey: apiToken.trim(),
      });
      onConnected(result);
      setZoneUrl("");
      setApiToken("");
    } catch (error) {
      logger.error("Make.com connection failed", { error });
      toast.error("Make.com connection failed");
    }
  };

  const inputClass =
    "w-full px-3 py-2 text-xs border border-border rounded-md bg-bg-primary text-text-primary placeholder:text-text-secondary/50 focus:outline-none focus:border-accent";

  return (
    <form onSubmit={handleSubmit} className="flex flex-col gap-3">
      <div>
        <label
          htmlFor="make-zone-url"
          className="block text-[11px] font-medium text-text-secondary uppercase tracking-wide mb-1.5"
        >
          Zone URL
        </label>
        <input
          id="make-zone-url"
          type="url"
          className={inputClass}
          placeholder="https://eu1.make.com"
          value={zoneUrl}
          onChange={(e) => handleUrlChange(e.target.value)}
          disabled={connectMutation.isPending}
        />
        <div className="text-[11px] text-text-secondary mt-1">
          Your Make.com zone (e.g., eu1.make.com, us1.make.com). Check your
          account region in Make settings.
        </div>
      </div>

      <div>
        <label
          htmlFor="make-api-token"
          className="block text-[11px] font-medium text-text-secondary uppercase tracking-wide mb-1.5"
        >
          API Token
        </label>
        <input
          id="make-api-token"
          type="password"
          className={inputClass}
          value={apiToken}
          onChange={(e) => setApiToken(e.target.value)}
          disabled={connectMutation.isPending}
        />
        <div className="text-[11px] text-text-secondary mt-1">
          Generate in Make.com under Organization &gt; Tokens
        </div>
      </div>

      <div className="flex gap-2.5 p-3 bg-accent/10 border border-accent/20 rounded-md">
        <Shield size={16} className="text-accent shrink-0 mt-0.5" />
        <p className="text-[11px] text-text-secondary leading-relaxed">
          LocalPush will discover webhook endpoints from your Make scenarios.
          Deliveries go directly to hook URLs (no additional auth needed).
        </p>
      </div>

      <div className="flex justify-end">
        <button
          type="submit"
          className="text-xs font-medium px-4 py-2 rounded-md bg-accent text-white hover:bg-accent/90 transition-colors disabled:opacity-50"
          disabled={
            connectMutation.isPending || !zoneUrl.trim() || !apiToken.trim()
          }
        >
          {connectMutation.isPending ? "Testing..." : "Test Connection"}
        </button>
      </div>

      {connectMutation.isSuccess && (
        <div className="text-xs text-success bg-success-bg border border-success/30 rounded-md p-2.5">
          Connected! Make.com instance verified
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
