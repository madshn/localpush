import { Shield } from "lucide-react";
import { useSourceProperties, useSetSourceProperty } from "../api/hooks/useSourceConfig";

interface SourceSettingsProps {
  sourceId: string;
  sourceName: string;
  onClose: () => void;
}

/**
 * Per-source property configuration panel.
 *
 * Displays toggles for each configurable property (library_stats, recent_photos, etc.)
 * with privacy warnings for sensitive properties (photo_location, first_prompt_preview).
 *
 * Changes save immediately via optimistic updates.
 */
export function SourceSettings({ sourceId, sourceName, onClose }: SourceSettingsProps) {
  const { data: properties, isLoading } = useSourceProperties(sourceId);
  const setProperty = useSetSourceProperty();

  if (isLoading) {
    return (
      <div className="source-settings loading">
        <div className="source-settings-header">
          <h3>{sourceName} Settings</h3>
          <button onClick={onClose} className="close-btn">
            ×
          </button>
        </div>
        <p>Loading properties...</p>
      </div>
    );
  }

  if (!properties || properties.length === 0) {
    return (
      <div className="source-settings empty">
        <div className="source-settings-header">
          <h3>{sourceName} Settings</h3>
          <button onClick={onClose} className="close-btn">
            ×
          </button>
        </div>
        <p>No configurable properties for this source.</p>
      </div>
    );
  }

  const handleToggle = (propertyKey: string, currentEnabled: boolean) => {
    setProperty.mutate({
      sourceId,
      property: propertyKey,
      enabled: !currentEnabled,
    });
  };

  return (
    <div className="source-settings">
      <div className="source-settings-header">
        <h3>{sourceName} Settings</h3>
        <button onClick={onClose} className="close-btn" aria-label="Close settings">
          ×
        </button>
      </div>

      <div className="source-settings-body">
        {properties.map((prop) => (
          <div key={prop.key} className="property-item">
            <div className="property-main">
              <label htmlFor={`prop-${sourceId}-${prop.key}`} className="property-label">
                {prop.privacy_sensitive && (
                  <Shield size={16} className="privacy-icon" aria-label="Privacy sensitive" />
                )}
                <span className="property-name">{prop.label}</span>
              </label>
              <input
                type="checkbox"
                id={`prop-${sourceId}-${prop.key}`}
                checked={prop.enabled}
                onChange={() => handleToggle(prop.key, prop.enabled)}
                className="property-toggle"
              />
            </div>
            <p className="property-description">{prop.description}</p>
            {prop.privacy_sensitive && prop.enabled && (
              <div className="privacy-warning">
                <Shield size={14} />
                <span>This property may contain sensitive or identifiable information.</span>
              </div>
            )}
          </div>
        ))}
      </div>

      {setProperty.isPending && (
        <div className="source-settings-status">Saving...</div>
      )}
      {setProperty.isError && (
        <div className="source-settings-error">
          Failed to save: {String(setProperty.error)}
        </div>
      )}
    </div>
  );
}
