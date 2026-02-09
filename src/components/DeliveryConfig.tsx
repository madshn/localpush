import { useState } from "react";
import { Plus, X } from "lucide-react";

interface DeliveryConfigProps {
  endpointName: string;
  endpointUrl: string;
  authenticated: boolean;
  onConfirm: (
    customHeaders: [string, string][],
    authHeaderName: string,
    authHeaderValue: string
  ) => void;
  onBack: () => void;
}

export function DeliveryConfig({
  endpointName,
  endpointUrl,
  authenticated,
  onConfirm,
  onBack,
}: DeliveryConfigProps) {
  const [authName, setAuthName] = useState("Authorization");
  const [authValue, setAuthValue] = useState("");
  const [headers, setHeaders] = useState<[string, string][]>([]);
  const [showHeaders, setShowHeaders] = useState(false);

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

  const handleConfirm = () => {
    const nonEmptyHeaders = headers.filter(([k]) => k.trim() !== "");
    onConfirm(nonEmptyHeaders, authName, authValue);
  };

  const handleSkip = () => {
    onConfirm([], "", "");
  };

  const authRequired = authenticated;
  const canContinue = !authRequired || authValue.trim() !== "";

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
      {authRequired && (
        <div className="text-xs text-error bg-error-bg border border-error rounded-md p-2.5 mb-3">
          This endpoint requires authentication.
        </div>
      )}

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
            placeholder="Secret value (e.g. Bearer token...)"
            className={inputClass}
          />
        </div>
      </div>

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

      {/* Actions */}
      <div className="flex items-center justify-between">
        <button
          className="text-xs font-medium px-3 py-1.5 rounded-md bg-bg-tertiary text-text-secondary border border-border hover:border-border-hover transition-colors"
          onClick={onBack}
        >
          Back
        </button>
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
