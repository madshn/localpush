import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { TargetSetup } from './TargetSetup';

export function SettingsPanel() {
  const [autoUpdate, setAutoUpdate] = useState(true);

  useEffect(() => {
    const loadSettings = async () => {
      try {
        const autoUpdateSetting = await invoke<string | null>('get_setting', { key: 'auto_update' });
        setAutoUpdate(autoUpdateSetting !== 'false');
      } catch (error) {
        console.error('Failed to load auto-update setting:', error);
      }
    };
    loadSettings();
  }, []);

  const handleAutoUpdateChange = async (checked: boolean) => {
    setAutoUpdate(checked);
    try {
      await invoke('set_setting', { key: 'auto_update', value: checked ? 'true' : 'false' });
    } catch (error) {
      console.error('Failed to save auto-update setting:', error);
    }
  };

  return (
    <div>
      <TargetSetup />

      <div className="card" style={{ marginTop: 12 }}>
        <h2 className="card-title">General</h2>
        <div className="form-field">
          <label className="checkbox-label">
            <input
              type="checkbox"
              checked={autoUpdate}
              onChange={(e) => handleAutoUpdateChange(e.target.checked)}
            />
            <span>Automatically check for app updates on startup</span>
          </label>
        </div>
      </div>
    </div>
  );
}
