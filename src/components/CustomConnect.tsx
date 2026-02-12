import { useState } from "react";
import { Shield, Info } from "lucide-react";
import { toast } from "sonner";
import { useConnectCustom } from "../api/hooks/useTargets";
import { logger } from "../utils/logger";

interface TargetInfo {
  id: string;
  target_type: string;
}

interface CustomConnectProps {
  onConnected: (targetInfo: TargetInfo) => void;
}

export function CustomConnect({ onConnected }: CustomConnectProps) {
  const [name, setName] = useState("");
  const [webhookUrl, setWebhookUrl] = useState("");
  const [authType, setAuthType] = useState<string>("none");
  const [authToken, setAuthToken] = useState("");
  const [authHeaderName, setAuthHeaderName] = useState("");
  const [authHeaderValue, setAuthHeaderValue] = useState("");
  const [authUsername, setAuthUsername] = useState("");
  const [authPassword, setAuthPassword] = useState("");

  const connectMutation = useConnectCustom();

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    if (!name.trim() || !webhookUrl.trim()) {
      logger.warn("Custom webhook connection attempt with missing fields");
      toast.error("Name and webhook URL are required");
      return;
    }

    // Validate URL format
    if (!webhookUrl.startsWith("https://") &&
        !webhookUrl.startsWith("http://localhost") &&
        !webhookUrl.startsWith("http://127.0.0.1")) {
      toast.error("Webhook URL must use HTTPS (HTTP allowed only for localhost)");
      return;
    }

    // Validate auth-specific fields
    if (authType === "bearer" && !authToken.trim()) {
      toast.error("Bearer token is required");
      return;
    }
    if (authType === "header" && (!authHeaderName.trim() || !authHeaderValue.trim())) {
      toast.error("Header name and value are required");
      return;
    }
    if (authType === "basic" && (!authUsername.trim() || !authPassword.trim())) {
      toast.error("Username and password are required");
      return;
    }

    try {
      const result = await connectMutation.mutateAsync({
        name: name.trim(),
        webhookUrl: webhookUrl.trim(),
        authType,
        authToken: authToken || undefined,
        authHeaderName: authHeaderName || undefined,
        authHeaderValue: authHeaderValue || undefined,
        authUsername: authUsername || undefined,
        authPassword: authPassword || undefined,
      });
      toast.success("Custom webhook connected successfully");
      onConnected(result);

      // Reset form
      setName("");
      setWebhookUrl("");
      setAuthType("none");
      setAuthToken("");
      setAuthHeaderName("");
      setAuthHeaderValue("");
      setAuthUsername("");
      setAuthPassword("");
    } catch (error) {
      logger.error("Custom webhook connection failed", { error });
      const errorMessage = error instanceof Error ? error.message : "Connection failed";
      toast.error(`Connection failed: ${errorMessage}`);
    }
  };

  const inputClass =
    "w-full px-3 py-2 text-xs border border-border rounded-md bg-bg-primary text-text-primary placeholder:text-text-secondary/50 focus:outline-none focus:border-accent";
  const selectClass = inputClass;

  return (
    <form onSubmit={handleSubmit} className="flex flex-col gap-3">
      <div>
        <label
          htmlFor="custom-name"
          className="block text-[11px] font-medium text-text-secondary uppercase tracking-wide mb-1.5"
        >
          Name
        </label>
        <input
          id="custom-name"
          type="text"
          className={inputClass}
          placeholder="My API"
          value={name}
          onChange={(e) => setName(e.target.value)}
          disabled={connectMutation.isPending}
        />
        <div className="text-[11px] text-text-secondary mt-1">
          A friendly name for this webhook (e.g., "My Home Assistant", "Budget API")
        </div>
      </div>

      <div>
        <label
          htmlFor="custom-webhook-url"
          className="block text-[11px] font-medium text-text-secondary uppercase tracking-wide mb-1.5"
        >
          Webhook URL
        </label>
        <input
          id="custom-webhook-url"
          type="url"
          className={inputClass}
          placeholder="https://api.example.com/webhook"
          value={webhookUrl}
          onChange={(e) => setWebhookUrl(e.target.value)}
          disabled={connectMutation.isPending}
        />
        <div className="text-[11px] text-text-secondary mt-1">
          LocalPush will POST JSON payloads to this URL
        </div>
      </div>

      <div>
        <label
          htmlFor="custom-auth-type"
          className="block text-[11px] font-medium text-text-secondary uppercase tracking-wide mb-1.5"
        >
          Authentication
        </label>
        <select
          id="custom-auth-type"
          className={selectClass}
          value={authType}
          onChange={(e) => setAuthType(e.target.value)}
          disabled={connectMutation.isPending}
        >
          <option value="none">None</option>
          <option value="bearer">Bearer Token</option>
          <option value="header">Custom Header</option>
          <option value="basic">Basic Auth</option>
        </select>
      </div>

      {authType === "bearer" && (
        <div>
          <label
            htmlFor="custom-auth-token"
            className="block text-[11px] font-medium text-text-secondary uppercase tracking-wide mb-1.5"
          >
            Bearer Token
          </label>
          <input
            id="custom-auth-token"
            type="password"
            className={inputClass}
            placeholder="Your bearer token"
            value={authToken}
            onChange={(e) => setAuthToken(e.target.value)}
            disabled={connectMutation.isPending}
          />
          <div className="text-[11px] text-text-secondary mt-1">
            Will be sent as: <code className="text-accent">Authorization: Bearer YOUR_TOKEN</code>
          </div>
        </div>
      )}

      {authType === "header" && (
        <>
          <div>
            <label
              htmlFor="custom-auth-header-name"
              className="block text-[11px] font-medium text-text-secondary uppercase tracking-wide mb-1.5"
            >
              Header Name
            </label>
            <input
              id="custom-auth-header-name"
              type="text"
              className={inputClass}
              placeholder="X-API-Key"
              value={authHeaderName}
              onChange={(e) => setAuthHeaderName(e.target.value)}
              disabled={connectMutation.isPending}
            />
          </div>
          <div>
            <label
              htmlFor="custom-auth-header-value"
              className="block text-[11px] font-medium text-text-secondary uppercase tracking-wide mb-1.5"
            >
              Header Value
            </label>
            <input
              id="custom-auth-header-value"
              type="password"
              className={inputClass}
              placeholder="Your API key"
              value={authHeaderValue}
              onChange={(e) => setAuthHeaderValue(e.target.value)}
              disabled={connectMutation.isPending}
            />
            <div className="text-[11px] text-text-secondary mt-1">
              Will be sent as a custom header
            </div>
          </div>
        </>
      )}

      {authType === "basic" && (
        <>
          <div>
            <label
              htmlFor="custom-auth-username"
              className="block text-[11px] font-medium text-text-secondary uppercase tracking-wide mb-1.5"
            >
              Username
            </label>
            <input
              id="custom-auth-username"
              type="text"
              className={inputClass}
              placeholder="Username"
              value={authUsername}
              onChange={(e) => setAuthUsername(e.target.value)}
              disabled={connectMutation.isPending}
            />
          </div>
          <div>
            <label
              htmlFor="custom-auth-password"
              className="block text-[11px] font-medium text-text-secondary uppercase tracking-wide mb-1.5"
            >
              Password
            </label>
            <input
              id="custom-auth-password"
              type="password"
              className={inputClass}
              placeholder="Password"
              value={authPassword}
              onChange={(e) => setAuthPassword(e.target.value)}
              disabled={connectMutation.isPending}
            />
            <div className="text-[11px] text-text-secondary mt-1">
              Will be sent as: <code className="text-accent">Authorization: Basic BASE64</code>
            </div>
          </div>
        </>
      )}

      <div className="flex gap-2.5 p-3 bg-accent/10 border border-accent/20 rounded-md">
        <Shield size={16} className="text-accent shrink-0 mt-0.5" />
        <p className="text-[11px] text-text-secondary leading-relaxed">
          Connect any REST endpoint. LocalPush will POST JSON payloads with your configured authentication.
        </p>
      </div>

      {authType !== "none" && (
        <div className="flex gap-2.5 p-3 bg-warning-bg border border-warning/20 rounded-md">
          <Info size={16} className="text-warning shrink-0 mt-0.5" />
          <p className="text-[11px] text-text-secondary leading-relaxed">
            Credentials are stored securely in your system keychain (macOS Keychain in production, file-based in dev).
          </p>
        </div>
      )}

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
          Connected! Custom webhook verified
        </div>
      )}

      {connectMutation.isError && (
        <div className="text-xs text-error bg-error-bg border border-error/30 rounded-md p-2.5">
          {connectMutation.error?.message || "Connection failed"}
        </div>
      )}
    </form>
  );
}
