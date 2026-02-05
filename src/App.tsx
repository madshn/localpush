import { useState } from "react";
import { useDeliveryStatus } from "./api/hooks/useDeliveryStatus";
import { StatusIndicator } from "./components/StatusIndicator";
import { SourceList } from "./components/SourceList";
import { DeliveryQueue } from "./components/DeliveryQueue";
import { SettingsPanel } from "./components/SettingsPanel";

function App() {
  const [view, setView] = useState<"status" | "sources" | "settings">("status");
  const { data: status } = useDeliveryStatus();

  return (
    <div className="app">
      <header className="app-header">
        <h1>LocalPush</h1>
        <StatusIndicator status={status?.overall ?? "unknown"} />
      </header>

      <nav className="app-nav">
        <button
          className={view === "status" ? "active" : ""}
          onClick={() => setView("status")}
        >
          Status
        </button>
        <button
          className={view === "sources" ? "active" : ""}
          onClick={() => setView("sources")}
        >
          Sources
        </button>
        <button
          className={view === "settings" ? "active" : ""}
          onClick={() => setView("settings")}
        >
          Settings
        </button>
      </nav>

      <main className="app-main">
        {view === "status" && <DeliveryQueue />}
        {view === "sources" && <SourceList />}
        {view === "settings" && <SettingsPanel />}
      </main>
    </div>
  );
}

export default App;
