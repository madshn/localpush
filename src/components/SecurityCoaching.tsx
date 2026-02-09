interface SecurityCoachingProps {
  endpointUrl: string;
  authenticated: boolean;
  authType?: string;
  onConfirm: () => void;
  onBack: () => void;
}

export function SecurityCoaching({
  endpointUrl,
  authenticated,
  authType,
  onConfirm,
  onBack,
}: SecurityCoachingProps) {
  const isHttps = endpointUrl.toLowerCase().startsWith("https://");
  const transportSecure = isHttps;
  const authSecure = authenticated;

  return (
    <div className="card">
      <h3 className="card-title">Security Assessment</h3>

      <div style={{ display: "flex", flexDirection: "column", gap: "16px", marginBottom: "16px" }}>
        {/* Transport Security */}
        <div>
          <div style={{ display: "flex", alignItems: "center", gap: "8px", marginBottom: "8px" }}>
            <div
              style={{
                width: "12px",
                height: "12px",
                borderRadius: "50%",
                background: transportSecure ? "var(--success)" : "var(--error)",
              }}
            />
            <span style={{ fontWeight: 600, fontSize: "13px" }}>Transport Security</span>
          </div>
          <p style={{ fontSize: "12px", color: "var(--text-secondary)", marginLeft: "20px" }}>
            {transportSecure ? (
              <>HTTPS encryption detected. Data will be encrypted in transit.</>
            ) : (
              <>
                <span style={{ color: "var(--error)" }}>Warning:</span> HTTP connection detected.
                Data will be sent unencrypted.
              </>
            )}
          </p>
        </div>

        {/* Authentication */}
        <div>
          <div style={{ display: "flex", alignItems: "center", gap: "8px", marginBottom: "8px" }}>
            <div
              style={{
                width: "12px",
                height: "12px",
                borderRadius: "50%",
                background: authSecure ? "var(--success)" : "var(--warning)",
              }}
            />
            <span style={{ fontWeight: 600, fontSize: "13px" }}>Authentication</span>
          </div>
          <p style={{ fontSize: "12px", color: "var(--text-secondary)", marginLeft: "20px" }}>
            {authSecure ? (
              <>
                Authenticated endpoint ({authType || "unknown method"}). Only authorized recipients
                can access data.
              </>
            ) : (
              <>
                <span style={{ color: "var(--warning)" }}>Advisory:</span> No authentication
                configured. Anyone with the URL can receive data.
              </>
            )}
          </p>
        </div>

        {/* Overall Guidance */}
        <div
          style={{
            padding: "12px",
            background: "var(--bg-primary)",
            borderRadius: "6px",
            border: `1px solid var(--border)`,
          }}
        >
          <p style={{ fontSize: "12px", color: "var(--text-secondary)", lineHeight: "1.5" }}>
            {transportSecure && authSecure ? (
              <>This endpoint uses industry-standard security practices.</>
            ) : !transportSecure ? (
              <>
                <strong style={{ color: "var(--error)" }}>Not recommended:</strong> Sending
                sensitive data over HTTP exposes it to interception.
              </>
            ) : (
              <>
                <strong style={{ color: "var(--warning)" }}>Caution:</strong> Ensure this endpoint
                URL is kept private if it contains sensitive data.
              </>
            )}
          </p>
        </div>
      </div>

      <div className="preview-actions">
        <button className="btn btn-secondary" onClick={onBack}>
          Back
        </button>
        <button className="btn" onClick={onConfirm}>
          Confirm & Enable
        </button>
      </div>
    </div>
  );
}
