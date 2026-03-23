import { Shield, X } from "lucide-react";
import {
  useSetSourceProperty,
  useSetSourceWindowDays,
  useSourceProperties,
  useSourceWindowSetting,
} from "../api/hooks/useSourceConfig";

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
  const { data: windowSetting } = useSourceWindowSetting(sourceId);
  const setProperty = useSetSourceProperty();
  const setWindowDays = useSetSourceWindowDays();

  const handleToggle = (propertyKey: string, currentEnabled: boolean) => {
    setProperty.mutate({
      sourceId,
      property: propertyKey,
      enabled: !currentEnabled,
    });
  };

  const handleWindowChange = (days: number) => {
    setWindowDays.mutate({
      sourceId,
      days,
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

  if ((!properties || properties.length === 0) && !windowSetting) {
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

      {windowSetting && (
        <div className="mb-3 pb-3 border-b border-border/60">
          <label
            htmlFor={`window-${sourceId}`}
            className="block text-text-primary text-[11px] font-medium mb-1"
          >
            {windowSetting.label}
          </label>
          <select
            id={`window-${sourceId}`}
            value={windowSetting.days}
            onChange={(event) => handleWindowChange(Number(event.target.value))}
            className="w-full rounded-md border border-border bg-bg-secondary px-2 py-1 text-xs text-text-primary"
          >
            {windowSetting.recommended_days.map((days) => (
              <option key={days} value={days}>
                {days} days
              </option>
            ))}
          </select>
          <p className="text-[10px] text-text-secondary/70 mt-1">{windowSetting.description}</p>
          <p className="text-[10px] text-text-secondary/60 mt-0.5">
            Default {windowSetting.default_days}d, range {windowSetting.min_days}-{windowSetting.max_days}d
          </p>
        </div>
      )}

      <div className="flex flex-col gap-2">
        {(properties ?? []).map((prop) => (
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

      {(setProperty.isPending || setWindowDays.isPending) && (
        <div className="mt-2 text-[10px] text-text-secondary">Saving...</div>
      )}
      {setProperty.isError && (
        <div className="mt-2 text-[10px] text-error">
          Failed to save: {String(setProperty.error)}
        </div>
      )}
      {setWindowDays.isError && (
        <div className="mt-2 text-[10px] text-error">
          Failed to save: {String(setWindowDays.error)}
        </div>
      )}
    </div>
  );
}
