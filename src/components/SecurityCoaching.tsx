import { Shield, Lock, Unlock, AlertTriangle } from "lucide-react";

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
    <div className="bg-bg-secondary border border-border rounded-lg p-4">
      <h3 className="text-sm font-semibold mb-3">Security Assessment</h3>

      <div className="flex flex-col gap-3 mb-3">
        {/* Transport Security */}
        <div>
          <div className="flex items-center gap-2 mb-1.5">
            {transportSecure ? (
              <Lock size={14} className="text-success" />
            ) : (
              <Unlock size={14} className="text-error" />
            )}
            <span className="text-xs font-semibold">Transport Security</span>
          </div>
          <p className="text-xs text-text-secondary ml-[22px]">
            {transportSecure ? (
              <>HTTPS encryption detected. Data will be encrypted in transit.</>
            ) : (
              <>
                <span className="text-error">Warning:</span> HTTP connection
                detected. Data will be sent unencrypted.
              </>
            )}
          </p>
        </div>

        {/* Authentication */}
        <div>
          <div className="flex items-center gap-2 mb-1.5">
            {authSecure ? (
              <Shield size={14} className="text-success" />
            ) : (
              <AlertTriangle size={14} className="text-warning" />
            )}
            <span className="text-xs font-semibold">Authentication</span>
          </div>
          <p className="text-xs text-text-secondary ml-[22px]">
            {authSecure ? (
              <>
                Authenticated endpoint ({authType || "unknown method"}). Only
                authorized recipients can access data.
              </>
            ) : (
              <>
                <span className="text-warning">Advisory:</span> No
                authentication configured. Anyone with the URL can receive data.
              </>
            )}
          </p>
        </div>

        {/* Security coaching box (Stitch prototype 7 style) */}
        <div className="flex gap-2.5 p-3 bg-accent/10 border border-accent/20 rounded-md">
          <Shield size={16} className="text-accent shrink-0 mt-0.5" />
          <p className="text-xs text-text-secondary leading-relaxed">
            {transportSecure && authSecure ? (
              <>This endpoint uses industry-standard security practices.</>
            ) : !transportSecure ? (
              <>
                <strong className="text-error">Not recommended:</strong> Sending
                sensitive data over HTTP exposes it to interception. Use HTTPS to
                ensure your local data is encrypted during transit.
              </>
            ) : (
              <>
                <strong className="text-warning">Caution:</strong> Ensure this
                endpoint URL is kept private if it handles sensitive data.
              </>
            )}
          </p>
        </div>
      </div>

      <div className="flex gap-2 justify-end">
        <button
          className="text-xs font-medium px-3 py-1.5 rounded-md bg-bg-tertiary text-text-secondary border border-border hover:border-border-hover transition-colors"
          onClick={onBack}
        >
          Back
        </button>
        <button
          className="text-xs font-medium px-3 py-1.5 rounded-md bg-accent text-white hover:bg-accent/90 transition-colors"
          onClick={onConfirm}
        >
          Confirm & Enable
        </button>
      </div>
    </div>
  );
}
