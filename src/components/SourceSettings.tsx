import { Shield, X } from "lucide-react";
import { useSourceProperties, useSetSourceProperty } from "../api/hooks/useSourceConfig";

interface SourceSettingsProps {
  sourceId: string;
  sourceName: string;
  onClose: () => void;
}

/**
 * Per-source property configuration panel.
 *
 * Displays toggles for each configurable property with privacy warnings
 * for sensitive properties. Changes save immediately via optimistic updates.
 */
export function SourceSettings({ sourceId, sourceName, onClose }: SourceSettingsProps) {
  const { data: properties, isLoading } = useSourceProperties(sourceId);
  const setProperty = useSetSourceProperty();

  const handleToggle = (propertyKey: string, currentEnabled: boolean) => {
    setProperty.mutate({
      sourceId,
      property: propertyKey,
      enabled: !currentEnabled,
    });
  };

  if (isLoading) {
    return (
      <div className="mt-2 p-3 bg-bg-primary border border-border rounded-md text-xs">
        <div className="flex items-center justify-between mb-2">
          <span className="font-medium text-text-primary">{sourceName} Properties</span>
          <button onClick={onClose} className="p-0.5 text-text-secondary hover:text-text-primary transition-colors">
            <X size={12} />
          </button>
        </div>
        <p className="text-text-secondary">Loading properties...</p>
      </div>
    );
  }

  if (!properties || properties.length === 0) {
    return (
      <div className="mt-2 p-3 bg-bg-primary border border-border rounded-md text-xs">
        <div className="flex items-center justify-between mb-2">
          <span className="font-medium text-text-primary">{sourceName} Properties</span>
          <button onClick={onClose} className="p-0.5 text-text-secondary hover:text-text-primary transition-colors">
            <X size={12} />
          </button>
        </div>
        <p className="text-text-secondary">No configurable properties for this source.</p>
      </div>
    );
  }

  return (
    <div className="mt-2 p-3 bg-bg-primary border border-border rounded-md text-xs">
      <div className="flex items-center justify-between mb-2">
        <span className="font-medium text-text-primary">Data Properties</span>
        <button onClick={onClose} className="p-0.5 text-text-secondary hover:text-text-primary transition-colors">
          <X size={12} />
        </button>
      </div>

      <div className="flex flex-col gap-2">
        {properties.map((prop) => (
          <div key={prop.key}>
            <label
              htmlFor={`prop-${sourceId}-${prop.key}`}
              className="flex items-center justify-between cursor-pointer"
            >
              <div className="flex items-center gap-1.5 flex-1 min-w-0">
                {prop.privacy_sensitive && (
                  <Shield size={12} className="text-warning shrink-0" />
                )}
                <span className="text-text-primary truncate">{prop.label}</span>
              </div>
              <input
                type="checkbox"
                id={`prop-${sourceId}-${prop.key}`}
                checked={prop.enabled}
                onChange={() => handleToggle(prop.key, prop.enabled)}
                className="w-3.5 h-3.5 rounded cursor-pointer accent-accent shrink-0 ml-2"
              />
            </label>
            <p className="text-[10px] text-text-secondary/70 mt-0.5 ml-0">{prop.description}</p>
            {prop.privacy_sensitive && prop.enabled && (
              <div className="flex items-center gap-1 mt-1 text-[10px] text-warning">
                <Shield size={10} />
                <span>May contain sensitive information</span>
              </div>
            )}
          </div>
        ))}
      </div>

      {setProperty.isPending && (
        <div className="mt-2 text-[10px] text-text-secondary">Saving...</div>
      )}
      {setProperty.isError && (
        <div className="mt-2 text-[10px] text-error">
          Failed to save: {String(setProperty.error)}
        </div>
      )}
    </div>
  );
}
