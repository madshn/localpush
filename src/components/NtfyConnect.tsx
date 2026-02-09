import { useState } from 'react';
import { useConnectNtfy } from '../api/hooks/useTargets';
import { logger } from '../utils/logger';

interface NtfyConnectProps {
  onConnected: (targetInfo: any) => void;
}

export function NtfyConnect({ onConnected }: NtfyConnectProps) {
  const [serverUrl, setServerUrl] = useState('');
  const [topic, setTopic] = useState('');
  const [authToken, setAuthToken] = useState('');
  const connectMutation = useConnectNtfy();

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    if (!serverUrl.trim()) {
      logger.warn('ntfy connection attempt with missing server URL');
      return;
    }

    try {
      const result = await connectMutation.mutateAsync({
        serverUrl: serverUrl.trim(),
        topic: topic.trim() || undefined,
        authToken: authToken.trim() || undefined,
      });
      onConnected(result);
      setServerUrl('');
      setTopic('');
      setAuthToken('');
    } catch (error) {
      logger.error('ntfy connection failed', { error });
    }
  };

  return (
    <form onSubmit={handleSubmit}>
      <div className="form-field">
        <label htmlFor="ntfy-url">Server URL</label>
        <input
          id="ntfy-url"
          type="url"
          className="input"
          placeholder="https://ntfy.sh"
          value={serverUrl}
          onChange={(e) => setServerUrl(e.target.value)}
          disabled={connectMutation.isPending}
        />
      </div>

      <div className="form-field">
        <label htmlFor="ntfy-topic">Topic Name</label>
        <input
          id="ntfy-topic"
          type="text"
          className="input"
          placeholder="localpush-alerts"
          value={topic}
          onChange={(e) => setTopic(e.target.value)}
          disabled={connectMutation.isPending}
        />
      </div>

      <div className="form-field">
        <label htmlFor="ntfy-token">
          Auth Token <span style={{ fontSize: 12, color: 'var(--text-secondary)' }}>(optional)</span>
        </label>
        <input
          id="ntfy-token"
          type="password"
          className="input"
          value={authToken}
          onChange={(e) => setAuthToken(e.target.value)}
          disabled={connectMutation.isPending}
        />
      </div>

      <div className="form-actions">
        <button
          type="submit"
          className="btn"
          disabled={connectMutation.isPending || !serverUrl.trim()}
        >
          {connectMutation.isPending ? 'Testing...' : 'Test Connection'}
        </button>
      </div>

      {connectMutation.isSuccess && (
        <div className="status-message" style={{ backgroundColor: 'var(--success-bg)', color: 'var(--success-text)' }}>
          Connected! Server healthy
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
