import { useState } from 'react';
import { useConnectN8n } from '../api/hooks/useTargets';
import { logger } from '../utils/logger';

interface N8nConnectProps {
  onConnected: (targetInfo: any) => void;
}

export function N8nConnect({ onConnected }: N8nConnectProps) {
  const [instanceUrl, setInstanceUrl] = useState('');
  const [apiKey, setApiKey] = useState('');
  const connectMutation = useConnectN8n();

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    if (!instanceUrl.trim() || !apiKey.trim()) {
      logger.warn('n8n connection attempt with missing fields');
      return;
    }

    try {
      const result = await connectMutation.mutateAsync({
        instanceUrl: instanceUrl.trim(),
        apiKey: apiKey.trim(),
      });
      onConnected(result);
      setInstanceUrl('');
      setApiKey('');
    } catch (error) {
      logger.error('n8n connection failed', { error });
    }
  };

  const getApiKeyHelpUrl = () => {
    if (!instanceUrl.trim()) return null;
    try {
      const url = new URL(instanceUrl);
      return `${url.origin}/settings/api`;
    } catch {
      return null;
    }
  };

  const apiKeyHelpUrl = getApiKeyHelpUrl();

  return (
    <form onSubmit={handleSubmit}>
      <div className="form-field">
        <label htmlFor="n8n-url">Instance URL</label>
        <input
          id="n8n-url"
          type="url"
          className="input"
          placeholder="https://your-n8n.example.com"
          value={instanceUrl}
          onChange={(e) => setInstanceUrl(e.target.value)}
          disabled={connectMutation.isPending}
        />
        {apiKeyHelpUrl && (
          <div style={{ fontSize: 12, color: 'var(--text-secondary)', marginTop: 4 }}>
            Get your API key at{' '}
            <a href={apiKeyHelpUrl} target="_blank" rel="noopener noreferrer" style={{ color: 'var(--accent)' }}>
              {apiKeyHelpUrl}
            </a>
          </div>
        )}
      </div>

      <div className="form-field">
        <label htmlFor="n8n-api-key">API Key</label>
        <input
          id="n8n-api-key"
          type="password"
          className="input"
          value={apiKey}
          onChange={(e) => setApiKey(e.target.value)}
          disabled={connectMutation.isPending}
        />
      </div>

      <div className="form-actions">
        <button
          type="submit"
          className="btn"
          disabled={connectMutation.isPending || !instanceUrl.trim() || !apiKey.trim()}
        >
          {connectMutation.isPending ? 'Testing...' : 'Test Connection'}
        </button>
      </div>

      {connectMutation.isSuccess && (
        <div className="status-message" style={{ backgroundColor: 'var(--success-bg)', color: 'var(--success-text)' }}>
          Connected! {connectMutation.data.details?.active_workflows || 0} active workflows found
        </div>
      )}

      {connectMutation.isError && (
        <div className="status-message" style={{ backgroundColor: 'var(--error-bg)', color: 'var(--error-text)' }}>
          {connectMutation.error.message || 'Connection failed'}
        </div>
      )}
    </form>
  );
}
