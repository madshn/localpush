import { useState } from "react";

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

  return (
    <div className="card">
      <h3 className="card-title">Configure Delivery</h3>

      {/* Endpoint info */}
      <div
        style={{
          padding: "8px 10px",
          background: "var(--bg-primary)",
          borderRadius: "6px",
          marginBottom: "12px",
        }}
      >
        <div style={{ fontSize: "12px", fontWeight: 500 }}>{endpointName}</div>
        <div
          style={{
            fontSize: "10px",
            color: "var(--text-secondary)",
            fontFamily: "'SF Mono', Monaco, 'Cascadia Code', monospace",
          }}
        >
          {endpointUrl}
        </div>
      </div>

      {/* Auth section */}
      {authRequired && (
        <div
          className="status-message status-error"
          style={{ marginBottom: "12px", fontSize: "12px" }}
        >
          This endpoint requires authentication.
        </div>
      )}

      <div style={{ marginBottom: "12px" }}>
        <label style={{ fontSize: "12px", fontWeight: 500, display: "block", marginBottom: "4px" }}>
          {authRequired ? "Auth Header (required)" : "Auth Header (optional)"}
        </label>
        <div style={{ display: "flex", gap: "6px", marginBottom: "6px" }}>
          <input
            type="text"
            value={authName}
            onChange={(e) => setAuthName(e.target.value)}
            placeholder="Header name"
            style={{
              flex: "0 0 140px",
              padding: "6px 8px",
              fontSize: "12px",
              border: "1px solid var(--border)",
              borderRadius: "4px",
              background: "var(--bg-primary)",
              color: "var(--text-primary)",
            }}
          />
          <input
            type="password"
            value={authValue}
            onChange={(e) => setAuthValue(e.target.value)}
            placeholder="Secret value (e.g. Bearer token...)"
            style={{
              flex: 1,
              padding: "6px 8px",
              fontSize: "12px",
              border: "1px solid var(--border)",
              borderRadius: "4px",
              background: "var(--bg-primary)",
              color: "var(--text-primary)",
            }}
          />
        </div>
      </div>

      {/* Custom headers */}
      <div style={{ marginBottom: "12px" }}>
        <button
          onClick={() => {
            setShowHeaders(!showHeaders);
            if (!showHeaders && headers.length === 0) addHeader();
          }}
          style={{
            background: "none",
            border: "none",
            cursor: "pointer",
            color: "var(--accent)",
            fontSize: "12px",
            padding: 0,
            textDecoration: "underline",
          }}
        >
          {showHeaders ? "Hide custom headers" : "Add custom headers"}
        </button>

        {showHeaders && (
          <div style={{ marginTop: "8px", display: "flex", flexDirection: "column", gap: "6px" }}>
            {headers.map((header, i) => (
              <div key={i} style={{ display: "flex", gap: "6px", alignItems: "center" }}>
                <input
                  type="text"
                  value={header[0]}
                  onChange={(e) => updateHeader(i, 0, e.target.value)}
                  placeholder="Header name"
                  style={{
                    flex: "0 0 140px",
                    padding: "6px 8px",
                    fontSize: "12px",
                    border: "1px solid var(--border)",
                    borderRadius: "4px",
                    background: "var(--bg-primary)",
                    color: "var(--text-primary)",
                  }}
                />
                <input
                  type="text"
                  value={header[1]}
                  onChange={(e) => updateHeader(i, 1, e.target.value)}
                  placeholder="Value"
                  style={{
                    flex: 1,
                    padding: "6px 8px",
                    fontSize: "12px",
                    border: "1px solid var(--border)",
                    borderRadius: "4px",
                    background: "var(--bg-primary)",
                    color: "var(--text-primary)",
                  }}
                />
                <button
                  onClick={() => removeHeader(i)}
                  className="btn btn-secondary"
                  style={{ fontSize: "11px", padding: "4px 8px" }}
                >
                  x
                </button>
              </div>
            ))}
            <button
              onClick={addHeader}
              style={{
                background: "none",
                border: "none",
                cursor: "pointer",
                color: "var(--accent)",
                fontSize: "11px",
                padding: 0,
                textAlign: "left",
              }}
            >
              + Add another header
            </button>
          </div>
        )}
      </div>

      {/* Actions */}
      <div className="preview-actions">
        <button className="btn btn-secondary" onClick={onBack}>
          Back
        </button>
        <div style={{ display: "flex", gap: "8px" }}>
          {!authRequired && (
            <button className="btn btn-secondary" onClick={handleSkip}>
              Skip
            </button>
          )}
          <button className="btn" onClick={handleConfirm} disabled={!canContinue}>
            Continue
          </button>
        </div>
      </div>
    </div>
  );
}
