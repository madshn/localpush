import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { logger } from "../utils/logger";

interface Target {
  id: string;
  name: string;
  target_type: string;
}

interface Endpoint {
  id: string;
  name: string;
  url: string;
  authenticated: boolean;
  auth_type?: string;
  metadata?: Record<string, unknown>;
}

interface EndpointPickerProps {
  onSelect: (
    targetId: string,
    endpointId: string,
    endpointUrl: string,
    endpointName: string,
    authenticated: boolean,
    authType?: string
  ) => void;
  onCancel: () => void;
}

export function EndpointPicker({ onSelect, onCancel }: EndpointPickerProps) {
  const [step, setStep] = useState<"target" | "endpoint">("target");
  const [targets, setTargets] = useState<Target[]>([]);
  const [endpoints, setEndpoints] = useState<Endpoint[]>([]);
  const [selectedTarget, setSelectedTarget] = useState<Target | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [endpointFilter, setEndpointFilter] = useState("");

  useEffect(() => {
    loadTargets();
  }, []);

  const loadTargets = async () => {
    setLoading(true);
    setError(null);
    try {
      logger.debug("Loading targets for endpoint picker");
      const result = await invoke<Target[]>("list_targets");
      setTargets(result);
      logger.debug("Targets loaded", { count: result.length });
    } catch (err) {
      logger.error("Failed to load targets", { error: err });
      setError(`Failed to load targets: ${err}`);
    } finally {
      setLoading(false);
    }
  };

  const handleTargetSelect = async (target: Target) => {
    setSelectedTarget(target);
    setStep("endpoint");
    setLoading(true);
    setError(null);
    try {
      logger.debug("Loading endpoints for target", { targetId: target.id });
      const result = await invoke<Endpoint[]>("list_target_endpoints", {
        targetId: target.id,
      });
      setEndpoints(result);
      logger.debug("Endpoints loaded", { targetId: target.id, count: result.length });
    } catch (err) {
      logger.error("Failed to load endpoints", { targetId: target.id, error: err });
      setError(`Failed to load endpoints: ${err}`);
    } finally {
      setLoading(false);
    }
  };

  const handleEndpointSelect = (endpoint: Endpoint) => {
    if (!selectedTarget) return;
    logger.debug("Endpoint selected", {
      targetId: selectedTarget.id,
      endpointId: endpoint.id,
      endpointUrl: endpoint.url,
    });
    onSelect(
      selectedTarget.id,
      endpoint.id,
      endpoint.url,
      endpoint.name,
      endpoint.authenticated,
      endpoint.auth_type
    );
  };

  const handleBack = () => {
    if (step === "endpoint") {
      setStep("target");
      setSelectedTarget(null);
      setEndpoints([]);
    } else {
      onCancel();
    }
  };

  if (loading && targets.length === 0) {
    return (
      <div className="card">
        <h3 className="card-title">Select Endpoint</h3>
        <p style={{ color: "var(--text-secondary)", textAlign: "center", padding: "20px" }}>
          Loading...
        </p>
      </div>
    );
  }

  if (error) {
    return (
      <div className="card">
        <h3 className="card-title">Select Endpoint</h3>
        <div className="status-message status-error" style={{ marginBottom: "12px" }}>
          {error}
        </div>
        <div className="preview-actions">
          <button className="btn btn-secondary" onClick={onCancel}>
            Cancel
          </button>
        </div>
      </div>
    );
  }

  if (targets.length === 0) {
    return (
      <div className="card">
        <h3 className="card-title">Select Endpoint</h3>
        <div style={{ padding: "20px", textAlign: "center" }}>
          <p style={{ color: "var(--text-secondary)", marginBottom: "12px" }}>
            No targets connected yet.
          </p>
          <p style={{ fontSize: "12px", color: "var(--text-secondary)" }}>
            Go to Settings to add webhook targets.
          </p>
        </div>
        <div className="preview-actions">
          <button className="btn btn-secondary" onClick={onCancel}>
            Cancel
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="card">
      <h3 className="card-title">
        {step === "target" ? "Select Target System" : "Select Endpoint"}
      </h3>

      {step === "target" ? (
        <div style={{ display: "flex", flexDirection: "column", gap: "8px" }}>
          {targets.map((target) => (
            <div
              key={target.id}
              className="source-item"
              style={{ cursor: "pointer" }}
              onClick={() => handleTargetSelect(target)}
            >
              <div className="source-info">
                <h3>{target.name}</h3>
                <p>{target.target_type}</p>
              </div>
              <span style={{ fontSize: "18px", color: "var(--text-secondary)" }}>→</span>
            </div>
          ))}
        </div>
      ) : (
        <div style={{ display: "flex", flexDirection: "column", gap: "8px" }}>
          {loading ? (
            <p
              style={{ color: "var(--text-secondary)", textAlign: "center", padding: "20px" }}
            >
              Loading endpoints...
            </p>
          ) : endpoints.length === 0 ? (
            <p
              style={{ color: "var(--text-secondary)", textAlign: "center", padding: "20px" }}
            >
              No endpoints available for this target.
            </p>
          ) : (
            <>
              <div style={{ position: "relative" }}>
                <input
                  type="text"
                  placeholder="Filter endpoints..."
                  value={endpointFilter}
                  onChange={(e) => setEndpointFilter(e.target.value)}
                  style={{
                    width: "100%",
                    padding: "8px 28px 8px 10px",
                    fontSize: "12px",
                    border: "1px solid var(--border)",
                    borderRadius: "6px",
                    background: "var(--bg-primary)",
                    color: "var(--text-primary)",
                    boxSizing: "border-box",
                  }}
                />
                {endpointFilter && (
                  <button
                    onClick={() => setEndpointFilter("")}
                    style={{
                      position: "absolute",
                      right: "6px",
                      top: "50%",
                      transform: "translateY(-50%)",
                      background: "none",
                      border: "none",
                      cursor: "pointer",
                      color: "var(--text-secondary)",
                      fontSize: "14px",
                      padding: "2px 4px",
                    }}
                  >
                    x
                  </button>
                )}
              </div>
              {(() => {
                const filtered = endpoints.filter((ep) => {
                  if (!endpointFilter) return true;
                  const q = endpointFilter.toLowerCase();
                  return ep.name.toLowerCase().includes(q) || ep.url.toLowerCase().includes(q);
                });
                return (
                  <>
                    {endpointFilter && (
                      <p style={{ fontSize: "11px", color: "var(--text-secondary)", margin: 0 }}>
                        {filtered.length} of {endpoints.length} endpoints
                      </p>
                    )}
                    {filtered.map((endpoint) => (
              <div
                key={endpoint.id}
                className="source-item"
                style={{ cursor: "pointer" }}
                onClick={() => handleEndpointSelect(endpoint)}
              >
                <div className="source-info">
                  <h3>{endpoint.name}</h3>
                  <p
                    style={{
                      fontFamily: "'SF Mono', Monaco, 'Cascadia Code', monospace",
                      fontSize: "11px",
                    }}
                  >
                    {endpoint.url}
                  </p>
                </div>
                <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
                  {endpoint.authenticated && (
                    <span className="badge" style={{ fontSize: "10px" }}>
                      AUTH
                    </span>
                  )}
                  <span style={{ fontSize: "18px", color: "var(--text-secondary)" }}>→</span>
                </div>
              </div>
            ))}
                  </>
                );
              })()}
            </>
          )}
        </div>
      )}

      <div className="preview-actions" style={{ marginTop: "12px" }}>
        <button className="btn btn-secondary" onClick={handleBack}>
          {step === "target" ? "Cancel" : "Back"}
        </button>
      </div>
    </div>
  );
}
