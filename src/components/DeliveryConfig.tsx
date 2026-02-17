import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Plus, X, Copy, Check } from "lucide-react";
import { logger } from "../utils/logger";

type DeliveryMode = "on_change" | "interval" | "daily" | "weekly";

interface DeliveryConfigProps {
  sourceId: string;
  endpointName: string;
  endpointUrl: string;
  authenticated: boolean;
  authType?: string;
  existingAuthConfigured?: boolean;
  initialHeaders?: [string, string][];
  initialAuthName?: string;
  initialAuthValue?: string;
  initialDeliveryMode?: DeliveryMode;
  initialScheduleTime?: string;
  initialScheduleDay?: string;
  onConfirm: (
    customHeaders: [string, string][],
    authHeaderName: string,
    authHeaderValue: string,
    deliveryMode: DeliveryMode,
    scheduleTime: string | undefined,
    scheduleDay: string | undefined
  ) => void;
  onBack: () => void;
  onUnbind?: () => void;
}

export function DeliveryConfig({
  sourceId,
  endpointName,
  endpointUrl,
  authenticated,
  authType,
  existingAuthConfigured = false,
  initialHeaders,
  initialAuthName,
  initialAuthValue,
  initialDeliveryMode,
  initialScheduleTime,
  initialScheduleDay,
  onConfirm,
  onBack,
  onUnbind,
}: DeliveryConfigProps) {
  const [authName, setAuthName] = useState(initialAuthName || "Authorization");
  const [authValue, setAuthValue] = useState(initialAuthValue || "");
  const [headers, setHeaders] = useState<[string, string][]>(
    initialHeaders || []
  );
  const [showHeaders, setShowHeaders] = useState(
    (initialHeaders && initialHeaders.length > 0) || false
  );
  const [payloadCopied, setPayloadCopied] = useState(false);
  const [payloadLoading, setPayloadLoading] = useState(false);
  const [deliveryMode, setDeliveryMode] = useState<DeliveryMode>(initialDeliveryMode || "on_change");
  const [scheduleTime, setScheduleTime] = useState(initialScheduleTime || "00:01");
  const [scheduleDay, setScheduleDay] = useState(initialScheduleDay || "monday");
  const [intervalMinutes, setIntervalMinutes] = useState(
    initialDeliveryMode === "interval" && initialScheduleTime
      ? parseInt(initialScheduleTime, 10) || 15
      : 15
  );

  const addHeader = () => {
    setHeaders([...headers, ["", ""]]);
  };

  const updateHeader = (index: number, field: 0 | 1, value: string) => {
    const updated = [...headers];
    updated[index] = [...updated[index]] as [string, string];
    updated[index][field] = value;
    setHeaders(updated);
  };

  const removeHeader = (index: number) => {
    setHeaders(headers.filter((_, i) => i !== index));
  };

  const resolveScheduleTime = (): string | undefined => {
    if (deliveryMode === "on_change") return undefined;
    if (deliveryMode === "interval") return String(intervalMinutes);
    return scheduleTime;
  };

  const handleConfirm = () => {
    const nonEmptyHeaders = headers.filter(([k]) => k.trim() !== "");
    onConfirm(
      nonEmptyHeaders,
      authName,
      authValue,
      deliveryMode,
      resolveScheduleTime(),
      deliveryMode === "weekly" ? scheduleDay : undefined
    );
  };

  const handleSkip = () => {
    onConfirm([], "", "", deliveryMode,
      resolveScheduleTime(),
      deliveryMode === "weekly" ? scheduleDay : undefined
    );
  };

  const copyToClipboard = (text: string): boolean => {
    // Tauri webviews don't support navigator.clipboard — use execCommand fallback
    const textarea = document.createElement("textarea");
    textarea.value = text;
    textarea.style.position = "fixed";
    textarea.style.opacity = "0";
    document.body.appendChild(textarea);
    textarea.select();
    const ok = document.execCommand("copy");
    document.body.removeChild(textarea);
    return ok;
  };

  const handleCopyPayload = async () => {
    setPayloadLoading(true);
    try {
      const payload = await invoke<unknown>("get_source_sample_payload", {
        sourceId,
      });
      const json = JSON.stringify(payload, null, 2);
      const ok = copyToClipboard(json);
      if (ok) {
        setPayloadCopied(true);
        setTimeout(() => setPayloadCopied(false), 2000);
        logger.info("Sample payload copied to clipboard", {
          sourceId,
          length: json.length,
        });
      } else {
        logger.warn("execCommand copy returned false", { sourceId });
      }
    } catch (error) {
      logger.error("Failed to copy sample payload", { sourceId, error });
    } finally {
      setPayloadLoading(false);
    }
  };

  const oauthManaged = authType === "oauth2";
  const authRequired = authenticated && !oauthManaged;
  const canContinue = !authRequired || authValue.trim() !== "" || existingAuthConfigured;

  const inputClass =
    "w-full px-2 py-1.5 text-xs border border-border rounded bg-bg-primary text-text-primary placeholder:text-text-secondary/50 focus:outline-none focus:border-accent";

  return (
    <div className="bg-bg-secondary border border-border rounded-lg p-4">
      <h3 className="text-sm font-semibold mb-3">Configure Delivery</h3>

      {/* Endpoint info */}
      <div className="px-3 py-2 bg-bg-primary rounded-md mb-3">
        <div className="text-xs font-medium">{endpointName}</div>
        <div className="text-[10px] text-text-secondary font-mono">
          {endpointUrl}
        </div>
      </div>

      {/* Auth section */}
      {oauthManaged && (
        <div className="text-xs text-success bg-success-bg border border-success/30 rounded-md p-2.5 mb-3">
          Authenticated via OAuth2. No additional credentials needed.
        </div>
      )}
      {!oauthManaged && authRequired && !existingAuthConfigured && (
        <div className="text-xs text-error bg-error-bg border border-error rounded-md p-2.5 mb-3">
          This endpoint requires authentication.
        </div>
      )}
      {!oauthManaged && authRequired && existingAuthConfigured && (
        <div className="text-xs text-success bg-success-bg border border-success rounded-md p-2.5 mb-3">
          Auth token configured. Leave blank to keep existing token.
        </div>
      )}

      {!oauthManaged && (
        <div className="mb-3">
          <label className="block text-xs font-medium mb-1.5">
            {authRequired ? "Auth Header (required)" : "Auth Header (optional)"}
          </label>
          <div className="flex gap-1.5 mb-1.5">
            <input
              type="text"
              value={authName}
              onChange={(e) => setAuthName(e.target.value)}
              placeholder="Header name"
              className={`${inputClass} !w-[140px] shrink-0`}
            />
            <input
              type="password"
              value={authValue}
              onChange={(e) => setAuthValue(e.target.value)}
              placeholder={existingAuthConfigured ? "Leave blank to keep existing token" : "Secret value (e.g. Bearer token...)"}
              className={inputClass}
            />
          </div>
        </div>
      )}

      {/* Custom headers */}
      <div className="mb-3">
        <button
          onClick={() => {
            setShowHeaders(!showHeaders);
            if (!showHeaders && headers.length === 0) addHeader();
          }}
          className="text-xs text-accent hover:underline"
        >
          {showHeaders ? "Hide custom headers" : "Add custom headers"}
        </button>

        {showHeaders && (
          <div className="mt-2 flex flex-col gap-1.5">
            {headers.map((header, i) => (
              <div key={i} className="flex gap-1.5 items-center">
                <input
                  type="text"
                  value={header[0]}
                  onChange={(e) => updateHeader(i, 0, e.target.value)}
                  placeholder="Header name"
                  className={`${inputClass} !w-[140px] shrink-0`}
                />
                <input
                  type="text"
                  value={header[1]}
                  onChange={(e) => updateHeader(i, 1, e.target.value)}
                  placeholder="Value"
                  className={inputClass}
                />
                <button
                  onClick={() => removeHeader(i)}
                  className="shrink-0 p-1 text-text-secondary hover:text-error transition-colors"
                >
                  <X size={14} />
                </button>
              </div>
            ))}
            <button
              onClick={addHeader}
              className="flex items-center gap-1 text-[11px] text-accent hover:underline"
            >
              <Plus size={12} /> Add another header
            </button>
          </div>
        )}
      </div>

      {/* Delivery mode */}
      <div className="mb-3">
        <label className="block text-xs font-medium mb-1.5">
          Delivery Mode
        </label>
        <div className="flex flex-col gap-1.5">
          <label className="flex items-start gap-2 cursor-pointer">
            <input
              type="radio"
              name="deliveryMode"
              value="on_change"
              checked={deliveryMode === "on_change"}
              onChange={() => setDeliveryMode("on_change")}
              className="mt-0.5 accent-accent"
            />
            <div>
              <div className="text-xs font-medium">Real-time</div>
              <div className="text-[10px] text-text-secondary">
                Push immediately when file updates
              </div>
            </div>
          </label>
          <label className="flex items-start gap-2 cursor-pointer">
            <input
              type="radio"
              name="deliveryMode"
              value="interval"
              checked={deliveryMode === "interval"}
              onChange={() => setDeliveryMode("interval")}
              className="mt-0.5 accent-accent"
            />
            <div className="flex-1">
              <div className="text-xs font-medium">Every x minutes</div>
              <div className="text-[10px] text-text-secondary">
                Push at a regular interval
              </div>
              {deliveryMode === "interval" && (
                <div className="flex items-center gap-1.5 mt-1">
                  <span className="text-[10px] text-text-secondary">Every</span>
                  <select
                    value={intervalMinutes}
                    onChange={(e) => setIntervalMinutes(parseInt(e.target.value, 10))}
                    className={`${inputClass} !w-[70px]`}
                  >
                    <option value={5}>5</option>
                    <option value={10}>10</option>
                    <option value={15}>15</option>
                    <option value={30}>30</option>
                    <option value={60}>60</option>
                  </select>
                  <span className="text-[10px] text-text-secondary">minutes</span>
                </div>
              )}
            </div>
          </label>
          <label className="flex items-start gap-2 cursor-pointer">
            <input
              type="radio"
              name="deliveryMode"
              value="daily"
              checked={deliveryMode === "daily"}
              onChange={() => setDeliveryMode("daily")}
              className="mt-0.5 accent-accent"
            />
            <div className="flex-1">
              <div className="text-xs font-medium">Daily digest</div>
              <div className="text-[10px] text-text-secondary">
                Push once per day at a scheduled time
              </div>
              {deliveryMode === "daily" && (
                <input
                  type="time"
                  value={scheduleTime}
                  onChange={(e) => setScheduleTime(e.target.value)}
                  className={`${inputClass} !w-[120px] mt-1`}
                />
              )}
            </div>
          </label>
          <label className="flex items-start gap-2 cursor-pointer">
            <input
              type="radio"
              name="deliveryMode"
              value="weekly"
              checked={deliveryMode === "weekly"}
              onChange={() => setDeliveryMode("weekly")}
              className="mt-0.5 accent-accent"
            />
            <div className="flex-1">
              <div className="text-xs font-medium">Weekly digest</div>
              <div className="text-[10px] text-text-secondary">
                Push once per week on a scheduled day and time
              </div>
              {deliveryMode === "weekly" && (
                <div className="flex gap-1.5 mt-1">
                  <select
                    value={scheduleDay}
                    onChange={(e) => setScheduleDay(e.target.value)}
                    className={`${inputClass} !w-[130px]`}
                  >
                    <option value="monday">Monday</option>
                    <option value="tuesday">Tuesday</option>
                    <option value="wednesday">Wednesday</option>
                    <option value="thursday">Thursday</option>
                    <option value="friday">Friday</option>
                    <option value="saturday">Saturday</option>
                    <option value="sunday">Sunday</option>
                  </select>
                  <input
                    type="time"
                    value={scheduleTime}
                    onChange={(e) => setScheduleTime(e.target.value)}
                    className={`${inputClass} !w-[120px]`}
                  />
                </div>
              )}
            </div>
          </label>
        </div>
      </div>

      {/* Sample payload */}
      <div className="mb-3 p-2.5 bg-bg-primary border border-border rounded-md">
        <div className="flex items-center justify-between">
          <span className="text-[11px] text-text-secondary">
            Test with your recipient before enabling?
          </span>
          <button
            onClick={handleCopyPayload}
            disabled={payloadLoading}
            className="inline-flex items-center gap-1 text-[11px] font-medium px-2 py-1 rounded bg-bg-tertiary text-text-secondary border border-border hover:border-border-hover transition-colors disabled:opacity-50"
          >
            {payloadCopied ? (
              <>
                <Check size={12} className="text-success" />
                Copied
              </>
            ) : (
              <>
                <Copy size={12} />
                {payloadLoading ? "Loading..." : "Copy Sample Payload"}
              </>
            )}
          </button>
        </div>
      </div>

      {/* Actions */}
      <div className="flex items-center justify-between">
        <div className="flex gap-2">
          <button
            className="text-xs font-medium px-3 py-1.5 rounded-md bg-bg-tertiary text-text-secondary border border-border hover:border-border-hover transition-colors"
            onClick={onBack}
          >
            Back
          </button>
          {onUnbind && (
            <button
              className="text-xs font-medium px-3 py-1.5 rounded-md text-error border border-error/30 hover:bg-error-bg transition-colors"
              onClick={onUnbind}
            >
              Remove
            </button>
          )}
        </div>
        <div className="flex gap-2">
          {!authRequired && (
            <button
              className="text-xs font-medium px-3 py-1.5 rounded-md bg-bg-tertiary text-text-secondary border border-border hover:border-border-hover transition-colors"
              onClick={handleSkip}
            >
              Skip
            </button>
          )}
          <button
            className="text-xs font-medium px-3 py-1.5 rounded-md bg-accent text-white hover:bg-accent/90 transition-colors disabled:opacity-50"
            onClick={handleConfirm}
            disabled={!canContinue}
          >
            Continue
          </button>
        </div>
      </div>
    </div>
  );
}
