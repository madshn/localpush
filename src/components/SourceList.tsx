import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useQueryClient } from "@tanstack/react-query";
import { useSources } from "../api/hooks/useSources";
import { TransparencyPreview } from "./TransparencyPreview";

interface SourcePreview {
  title: string;
  summary: string;
  fields: Array<{ label: string; value: string; sensitive: boolean }>;
  lastUpdated: string | null;
}

export function SourceList() {
  const { data: sources, isLoading } = useSources();
  const queryClient = useQueryClient();
  const [expandedSourceId, setExpandedSourceId] = useState<string | null>(null);
  const [preview, setPreview] = useState<SourcePreview | null>(null);
  const [previewLoading, setPreviewLoading] = useState(false);

  const handleEnable = async (sourceId: string) => {
    try {
      await invoke("enable_source", { sourceId });
      await queryClient.invalidateQueries({ queryKey: ["sources"] });
    } catch (error) {
      console.error("Failed to enable source:", error);
      alert(`Failed to enable source: ${error}`);
    }
  };

  const handleDisable = async (sourceId: string) => {
    try {
      await invoke("disable_source", { sourceId });
      await queryClient.invalidateQueries({ queryKey: ["sources"] });
    } catch (error) {
      console.error("Failed to disable source:", error);
      alert(`Failed to disable source: ${error}`);
    }
  };

  const handleEnableClick = async (sourceId: string, isEnabled: boolean) => {
    if (isEnabled) {
      handleDisable(sourceId);
    } else {
      // Show transparency preview before enabling
      setExpandedSourceId(sourceId);
      await loadPreview(sourceId);
    }
  };

  const loadPreview = async (sourceId: string) => {
    setPreviewLoading(true);
    try {
      const data = await invoke<SourcePreview>("get_source_preview", { sourceId });
      setPreview(data);
    } catch (error) {
      console.error("Failed to load preview:", error);
      alert(`Failed to load preview: ${error}`);
    } finally {
      setPreviewLoading(false);
    }
  };

  const handlePreviewEnable = async () => {
    if (expandedSourceId) {
      await handleEnable(expandedSourceId);
      setExpandedSourceId(null);
      setPreview(null);
    }
  };

  const handlePreviewRefresh = async () => {
    if (expandedSourceId) {
      await loadPreview(expandedSourceId);
    }
  };

  if (isLoading) {
    return <div>Loading sources...</div>;
  }

  return (
    <div className="card">
      <h2 className="card-title">Data Sources</h2>
      {sources?.map((source) => (
        <div key={source.id}>
          <div className="source-item">
            <div className="source-info">
              <h3>{source.name}</h3>
              <p>{source.description}</p>
              {source.lastSync && (
                <p>Last sync: {new Date(source.lastSync).toLocaleString()}</p>
              )}
            </div>
            <button
              className={source.enabled ? "btn btn-secondary" : "btn"}
              onClick={() => handleEnableClick(source.id, source.enabled)}
            >
              {source.enabled ? "Disable" : "Enable"}
            </button>
          </div>
          {expandedSourceId === source.id && preview && (
            <div style={{ marginTop: "16px" }}>
              <TransparencyPreview
                sourceId={source.id}
                preview={preview}
                onEnable={handlePreviewEnable}
                onRefresh={handlePreviewRefresh}
                isLoading={previewLoading}
              />
            </div>
          )}
        </div>
      ))}
    </div>
  );
}
