import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Search, X, ChevronRight, Loader2 } from "lucide-react";
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
      const result = await invoke<Target[]>("list_targets");
      setTargets(result);
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
      const result = await invoke<Endpoint[]>("list_target_endpoints", {
        targetId: target.id,
      });
      setEndpoints(result);
    } catch (err) {
      logger.error("Failed to load endpoints", { targetId: target.id, error: err });
      setError(`Failed to load endpoints: ${err}`);
    } finally {
      setLoading(false);
    }
  };

  const handleEndpointSelect = (endpoint: Endpoint) => {
    if (!selectedTarget) return;
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
      <div className="bg-bg-secondary border border-border rounded-lg p-4">
        <h3 className="text-sm font-semibold mb-3">Select Endpoint</h3>
        <p className="text-xs text-text-secondary text-center py-5">Loading...</p>
      </div>
    );
  }

  if (error) {
    return (
      <div className="bg-bg-secondary border border-border rounded-lg p-4">
        <h3 className="text-sm font-semibold mb-3">Select Endpoint</h3>
        <div className="text-xs text-error bg-error-bg border border-error rounded-md p-3 mb-3">
          {error}
        </div>
        <div className="flex justify-end">
          <button
            className="text-xs font-medium px-3 py-1.5 rounded-md bg-bg-tertiary text-text-secondary border border-border hover:border-border-hover transition-colors"
            onClick={onCancel}
          >
            Cancel
          </button>
        </div>
      </div>
    );
  }

  if (targets.length === 0) {
    return (
      <div className="bg-bg-secondary border border-border rounded-lg p-4">
        <h3 className="text-sm font-semibold mb-3">Select Endpoint</h3>
        <div className="py-5 text-center">
          <p className="text-xs text-text-secondary mb-2">
            No targets connected yet.
          </p>
          <p className="text-[11px] text-text-secondary">
            Go to Settings to add webhook targets.
          </p>
        </div>
        <div className="flex justify-end">
          <button
            className="text-xs font-medium px-3 py-1.5 rounded-md bg-bg-tertiary text-text-secondary border border-border hover:border-border-hover transition-colors"
            onClick={onCancel}
          >
            Cancel
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="bg-bg-secondary border border-border rounded-lg p-4">
      <h3 className="text-sm font-semibold mb-3">
        {step === "target" ? "Select Target System" : "Select Endpoint"}
      </h3>

      {step === "target" ? (
        <div className="flex flex-col gap-2">
          {targets.map((target) => (
            <div
              key={target.id}
              className="flex items-center justify-between p-3 bg-bg-primary rounded-md cursor-pointer hover:bg-bg-tertiary transition-colors"
              onClick={() => handleTargetSelect(target)}
            >
              <div>
                <div className="text-xs font-medium">{target.name}</div>
                <div className="text-[11px] text-text-secondary">
                  {target.target_type}
                </div>
              </div>
              <ChevronRight size={14} className="text-text-secondary" />
            </div>
          ))}
        </div>
      ) : (
        <div className="flex flex-col gap-2">
          {loading ? (
            <div className="flex flex-col items-center gap-2 py-6">
              <Loader2 size={20} className="text-accent animate-spin" />
              <p className="text-xs text-text-secondary">Loading endpoints...</p>
            </div>
          ) : endpoints.length === 0 ? (
            <p className="text-xs text-text-secondary text-center py-5">
              No endpoints available for this target.
            </p>
          ) : (
            <>
              <div className="relative">
                <Search
                  size={14}
                  className="absolute left-2.5 top-1/2 -translate-y-1/2 text-text-secondary"
                />
                <input
                  type="text"
                  placeholder="Filter endpoints..."
                  value={endpointFilter}
                  onChange={(e) => setEndpointFilter(e.target.value)}
                  className="w-full pl-8 pr-8 py-2 text-xs border border-border rounded-md bg-bg-primary text-text-primary placeholder:text-text-secondary/50 focus:outline-none focus:border-accent"
                />
                {endpointFilter && (
                  <button
                    onClick={() => setEndpointFilter("")}
                    className="absolute right-2 top-1/2 -translate-y-1/2 text-text-secondary hover:text-text-primary"
                  >
                    <X size={14} />
                  </button>
                )}
              </div>
              {(() => {
                const filtered = endpoints.filter((ep) => {
                  if (!endpointFilter) return true;
                  const q = endpointFilter.toLowerCase();
                  return (
                    ep.name.toLowerCase().includes(q) ||
                    ep.url.toLowerCase().includes(q)
                  );
                });
                return (
                  <>
                    {endpointFilter && (
                      <p className="text-[11px] text-text-secondary">
                        {filtered.length} of {endpoints.length} endpoints
                      </p>
                    )}
                    {filtered.map((endpoint) => (
                      <div
                        key={endpoint.id}
                        className="flex items-center justify-between p-3 bg-bg-primary rounded-md cursor-pointer hover:bg-bg-tertiary transition-colors"
                        onClick={() => handleEndpointSelect(endpoint)}
                      >
                        <div className="min-w-0 flex-1">
                          <div className="text-xs font-medium">{endpoint.name}</div>
                          <div className="text-[10px] font-mono text-text-secondary truncate">
                            {endpoint.url}
                          </div>
                        </div>
                        <div className="flex items-center gap-2 ml-2">
                          {endpoint.authenticated && (
                            <span className="text-[9px] font-medium px-1.5 py-0.5 rounded bg-accent-muted text-accent">
                              AUTH
                            </span>
                          )}
                          <ChevronRight size={14} className="text-text-secondary" />
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

      <div className="flex justify-end mt-3">
        <button
          className="text-xs font-medium px-3 py-1.5 rounded-md bg-bg-tertiary text-text-secondary border border-border hover:border-border-hover transition-colors"
          onClick={handleBack}
        >
          {step === "target" ? "Cancel" : "Back"}
        </button>
      </div>
    </div>
  );
}
