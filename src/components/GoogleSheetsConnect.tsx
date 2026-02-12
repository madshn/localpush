import { useState } from "react";
import { Shield } from "lucide-react";
import { toast } from "sonner";
import { signIn } from "@choochmeque/tauri-plugin-google-auth-api";
import { useConnectGoogleSheets } from "../api/hooks/useTargets";
import { logger } from "../utils/logger";

const GOOGLE_CLIENT_ID = import.meta.env.VITE_GOOGLE_CLIENT_ID || "";
const GOOGLE_CLIENT_SECRET = import.meta.env.VITE_GOOGLE_CLIENT_SECRET || "";

interface TargetInfo {
  id: string;
  target_type: string;
}

interface GoogleSheetsConnectProps {
  onConnected: (targetInfo: TargetInfo) => void;
}

export function GoogleSheetsConnect({ onConnected }: GoogleSheetsConnectProps) {
  const [isSigningIn, setIsSigningIn] = useState(false);
  const connectMutation = useConnectGoogleSheets();

  const handleSignIn = async () => {
    if (!GOOGLE_CLIENT_ID) {
      toast.error(
        "Google OAuth not configured. Set client ID in GoogleSheetsConnect.tsx"
      );
      return;
    }

    setIsSigningIn(true);
    try {
      // Launch OAuth2 flow via tauri-plugin-google-auth
      const tokens = await signIn({
        clientId: GOOGLE_CLIENT_ID,
        clientSecret: GOOGLE_CLIENT_SECRET,
        scopes: [
          "openid",
          "email",
          "https://www.googleapis.com/auth/drive.readonly",
          "https://www.googleapis.com/auth/spreadsheets",
        ],
        successHtmlResponse:
          "<h1>Connected!</h1><p>You can close this window and return to LocalPush.</p>",
      });

      if (!tokens.accessToken || !tokens.refreshToken) {
        throw new Error("Missing tokens from Google sign-in");
      }

      // Fetch user email from Google userinfo
      const userInfoResp = await fetch(
        "https://www.googleapis.com/oauth2/v3/userinfo",
        { headers: { Authorization: `Bearer ${tokens.accessToken}` } }
      );
      const userInfo = await userInfoResp.json();
      const email = userInfo.email || "unknown@gmail.com";

      // Register with backend
      const result = await connectMutation.mutateAsync({
        email,
        accessToken: tokens.accessToken,
        refreshToken: tokens.refreshToken,
        expiresAt: tokens.expiresAt || Math.floor(Date.now() / 1000) + 3600,
        clientId: GOOGLE_CLIENT_ID,
        clientSecret: GOOGLE_CLIENT_SECRET,
      });

      onConnected(result);
      logger.info("Google Sheets target connected", { email });
    } catch (error) {
      logger.error("Google Sheets sign-in failed", { error });
      toast.error("Google sign-in failed");
    } finally {
      setIsSigningIn(false);
    }
  };

  return (
    <div className="flex flex-col gap-3">
      <p className="text-xs text-text-secondary">
        Connect your Google account to push data directly to Google Sheets
        spreadsheets. Each source gets its own worksheet tab.
      </p>

      {/* Security coaching */}
      <div className="flex gap-2.5 p-3 bg-accent/10 border border-accent/20 rounded-md">
        <Shield size={16} className="text-accent shrink-0 mt-0.5" />
        <div className="text-[11px] text-text-secondary leading-relaxed">
          <p className="font-medium text-text-primary mb-1">
            Minimal permissions requested:
          </p>
          <ul className="list-disc list-inside space-y-0.5">
            <li>
              <strong>Drive (read-only)</strong> — list your spreadsheets
            </li>
            <li>
              <strong>Sheets (read/write)</strong> — append rows to bound
              spreadsheets
            </li>
          </ul>
          <p className="mt-1.5">
            LocalPush never reads existing spreadsheet data. It only appends new
            rows.
          </p>
        </div>
      </div>

      <div className="flex justify-end">
        <button
          type="button"
          onClick={handleSignIn}
          className="text-xs font-medium px-4 py-2 rounded-md bg-accent text-white hover:bg-accent/90 transition-colors disabled:opacity-50"
          disabled={isSigningIn || connectMutation.isPending}
        >
          {isSigningIn || connectMutation.isPending
            ? "Connecting..."
            : "Sign in with Google"}
        </button>
      </div>

      {connectMutation.isSuccess && (
        <div className="text-xs text-success bg-success-bg border border-success/30 rounded-md p-2.5">
          Connected! Your spreadsheets are now available as delivery endpoints.
        </div>
      )}

      {connectMutation.isError && (
        <div className="text-xs text-error bg-error-bg border border-error/30 rounded-md p-2.5">
          {connectMutation.error.message || "Connection failed"}
        </div>
      )}
    </div>
  );
}
