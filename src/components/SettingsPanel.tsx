import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

type AuthType = "none" | "header" | "bearer" | "basic";

interface WebhookAuth {
  type: AuthType;
  name?: string;
  value?: string;
  token?: string;
  username?: string;
  password?: string;
}

interface WebhookConfig {
  url: string;
  auth: WebhookAuth;
}

export function SettingsPanel() {
  const [url, setUrl] = useState("");
  const [authType, setAuthType] = useState<AuthType>("none");
  const [headerName, setHeaderName] = useState("");
  const [headerValue, setHeaderValue] = useState("");
  const [bearerToken, setBearerToken] = useState("");
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [testStatus, setTestStatus] = useState<"idle" | "testing" | "success" | "error">("idle");
  const [testMessage, setTestMessage] = useState("");
  const [saveStatus, setSaveStatus] = useState<"idle" | "saving" | "success" | "error">("idle");
  const [autoUpdate, setAutoUpdate] = useState(true);

  useEffect(() => {
    // Load existing config
    const loadConfig = async () => {
      try {
        const config = await invoke<{ url: string; auth: WebhookAuth | null }>("get_webhook_config");
        if (config.url) {
          setUrl(config.url);
        }
        if (config.auth) {
          setAuthType(config.auth.type);
          if (config.auth.type === "header") {
            setHeaderName(config.auth.name || "");
            setHeaderValue(config.auth.value || "");
          } else if (config.auth.type === "bearer") {
            setBearerToken(config.auth.token || "");
          } else if (config.auth.type === "basic") {
            setUsername(config.auth.username || "");
            setPassword(config.auth.password || "");
          }
        }
      } catch (error) {
        console.error("Failed to load config:", error);
      }

      try {
        const autoUpdateSetting = await invoke<string | null>("get_setting", { key: "auto_update" });
        setAutoUpdate(autoUpdateSetting !== "false");
      } catch (error) {
        console.error("Failed to load auto-update setting:", error);
      }
    };
    loadConfig();
  }, []);

  const buildAuthConfig = (): WebhookAuth => {
    switch (authType) {
      case "none":
        return { type: "none" };
      case "header":
        return { type: "header", name: headerName, value: headerValue };
      case "bearer":
        return { type: "bearer", token: bearerToken };
      case "basic":
        return { type: "basic", username, password };
    }
  };

  const handleTest = async () => {
    setTestStatus("testing");
    setTestMessage("");
    try {
      const config: WebhookConfig = {
        url,
        auth: buildAuthConfig(),
      };
      const result = await invoke<string>("test_webhook", { config });
      setTestStatus("success");
      setTestMessage(result);
    } catch (error) {
      setTestStatus("error");
      setTestMessage(`Test failed: ${error}`);
    }
  };

  const handleSave = async () => {
    setSaveStatus("saving");
    try {
      const config: WebhookConfig = {
        url,
        auth: buildAuthConfig(),
      };
      await invoke("add_webhook_target", { config });
      await invoke("set_setting", { key: "auto_update", value: autoUpdate ? "true" : "false" });
      setSaveStatus("success");
      setTimeout(() => setSaveStatus("idle"), 2000);
    } catch (error) {
      setSaveStatus("error");
      alert(`Failed to save configuration: ${error}`);
      setTimeout(() => setSaveStatus("idle"), 2000);
    }
  };

  return (
    <div className="card">
      <h2 className="card-title">Webhook Configuration</h2>

      <div className="settings-form">
        <div className="form-field">
          <label htmlFor="webhook-url">Webhook URL</label>
          <input
            id="webhook-url"
            type="text"
            value={url}
            onChange={(e) => setUrl(e.target.value)}
            placeholder="https://your-webhook-endpoint.com/events"
            className="input"
          />
        </div>

        <div className="form-field">
          <label htmlFor="auth-type">Authentication Type</label>
          <select
            id="auth-type"
            value={authType}
            onChange={(e) => setAuthType(e.target.value as AuthType)}
            className="input"
          >
            <option value="none">None</option>
            <option value="header">Custom Header</option>
            <option value="bearer">Bearer Token</option>
            <option value="basic">Basic Auth</option>
          </select>
        </div>

        {authType === "header" && (
          <>
            <div className="form-field">
              <label htmlFor="header-name">Header Name</label>
              <input
                id="header-name"
                type="text"
                value={headerName}
                onChange={(e) => setHeaderName(e.target.value)}
                placeholder="X-API-Key"
                className="input"
              />
            </div>
            <div className="form-field">
              <label htmlFor="header-value">Header Value</label>
              <input
                id="header-value"
                type="password"
                value={headerValue}
                onChange={(e) => setHeaderValue(e.target.value)}
                placeholder="your-api-key"
                className="input"
              />
            </div>
          </>
        )}

        {authType === "bearer" && (
          <div className="form-field">
            <label htmlFor="bearer-token">Bearer Token</label>
            <input
              id="bearer-token"
              type="password"
              value={bearerToken}
              onChange={(e) => setBearerToken(e.target.value)}
              placeholder="your-bearer-token"
              className="input"
            />
          </div>
        )}

        {authType === "basic" && (
          <>
            <div className="form-field">
              <label htmlFor="username">Username</label>
              <input
                id="username"
                type="text"
                value={username}
                onChange={(e) => setUsername(e.target.value)}
                placeholder="username"
                className="input"
              />
            </div>
            <div className="form-field">
              <label htmlFor="password">Password</label>
              <input
                id="password"
                type="password"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                placeholder="password"
                className="input"
              />
            </div>
          </>
        )}

        <div className="form-field">
          <label className="checkbox-label">
            <input
              type="checkbox"
              checked={autoUpdate}
              onChange={(e) => setAutoUpdate(e.target.checked)}
            />
            <span>Automatically check for app updates on startup</span>
          </label>
        </div>

        {testStatus !== "idle" && (
          <div className={`status-message status-${testStatus}`}>
            {testMessage || (testStatus === "testing" ? "Testing connection..." : "")}
          </div>
        )}

        <div className="form-actions">
          <button
            className="btn btn-secondary"
            onClick={handleTest}
            disabled={!url || testStatus === "testing"}
          >
            {testStatus === "testing" ? "Testing..." : "Test Webhook"}
          </button>
          <button
            className="btn"
            onClick={handleSave}
            disabled={!url || saveStatus === "saving"}
          >
            {saveStatus === "saving" ? "Saving..." : "Save Configuration"}
          </button>
        </div>
      </div>
    </div>
  );
}
