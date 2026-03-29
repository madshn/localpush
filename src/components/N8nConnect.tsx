import { Shield } from 'lucide-react';
import { useState } from 'react';
import { useConnectN8n, useUpdateN8n } from '../api/hooks/useTargets';
import { logger } from '../utils/logger';

interface TargetInfo {
  id: string;
  target_type: string;
}

interface N8nConnectProps {
  onConnected: (targetInfo: TargetInfo) => void;
  targetId?: string;
  initialInstanceUrl?: string;
  submitLabel?: string;
  successLabel?: string;
  onCancel?: () => void;
}

export function N8nConnect({
  onConnected,
  targetId,
  initialInstanceUrl = '',
  submitLabel,
  successLabel,
  onCancel,
}: N8nConnectProps) {
  const [instanceUrl, setInstanceUrl] = useState(initialInstanceUrl);
  const [apiKey, setApiKey] = useState('');
  const connectMutation = useConnectN8n();
  const updateMutation = useUpdateN8n();
  const isEditing = !!targetId;
  const isPending = connectMutation.isPending || updateMutation.isPending;

  const handleUrlChange = (value: string) => {
    if (value && !value.startsWith('http://') && !value.startsWith('https://')) {
      setInstanceUrl(`https://${value}`);
    } else {
      setInstanceUrl(value);
    }
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    if (!instanceUrl.trim() || !apiKey.trim()) {
      logger.warn('n8n connection attempt with missing fields');
      return;
    }

    try {
      if (isEditing) {
        if (!targetId) {
          throw new Error('Missing target id');
        }
        const result = await updateMutation.mutateAsync({
          targetId,
          instanceUrl: instanceUrl.trim(),
          apiKey: apiKey.trim(),
        });
        onConnected({
          id: result.target_info.id,
          target_type: result.target_info.target_type,
        });
      } else {
        const result = await connectMutation.mutateAsync({
          instanceUrl: instanceUrl.trim(),
          apiKey: apiKey.trim(),
        });
        onConnected(result);
      }
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
  const inputClass =
    'w-full px-3 py-2 text-xs border border-border rounded-md bg-bg-primary text-text-primary placeholder:text-text-secondary/50 focus:outline-none focus:border-accent';

  return (
    <form onSubmit={handleSubmit} className="flex flex-col gap-3">
      <div>
        <label
          htmlFor="n8n-url"
          className="block text-[11px] font-medium text-text-secondary uppercase tracking-wide mb-1.5"
        >
          Instance URL
        </label>
        <input
          id="n8n-url"
          type="url"
          className={inputClass}
          placeholder="https://your-n8n.example.com"
          value={instanceUrl}
          onChange={(e) => handleUrlChange(e.target.value)}
          disabled={isPending}
        />
        {apiKeyHelpUrl && (
          <div className="text-[11px] text-text-secondary mt-1">
            Get your API key at{' '}
            <a
              href={apiKeyHelpUrl}
              target="_blank"
              rel="noopener noreferrer"
              className="text-accent hover:underline"
            >
              {apiKeyHelpUrl}
            </a>
          </div>
        )}
      </div>

      <div>
        <label
          htmlFor="n8n-api-key"
          className="block text-[11px] font-medium text-text-secondary uppercase tracking-wide mb-1.5"
        >
          API Key
        </label>
        <input
          id="n8n-api-key"
          type="password"
          className={inputClass}
          value={apiKey}
          onChange={(e) => setApiKey(e.target.value)}
          disabled={isPending}
        />
        {isEditing && (
          <div className="text-[11px] text-text-secondary mt-1">
            Enter a fresh API key to replace the saved one for this target.
          </div>
        )}
      </div>

      {/* Security coaching box */}
      <div className="flex gap-2.5 p-3 bg-accent/10 border border-accent/20 rounded-md">
        <Shield size={16} className="text-accent shrink-0 mt-0.5" />
        <p className="text-[11px] text-text-secondary leading-relaxed">
          Use HTTPS to ensure your local data is encrypted during transit. Avoid exposing plain HTTP
          endpoints publicly.
        </p>
      </div>

      <div className="flex justify-end gap-2">
        {onCancel && (
          <button
            type="button"
            className="text-xs font-medium px-4 py-2 rounded-md bg-bg-tertiary text-text-secondary border border-border hover:border-border-hover transition-colors"
            onClick={onCancel}
            disabled={isPending}
          >
            Cancel
          </button>
        )}
        <button
          type="submit"
          className="text-xs font-medium px-4 py-2 rounded-md bg-accent text-white hover:bg-accent/90 transition-colors disabled:opacity-50"
          disabled={isPending || !instanceUrl.trim() || !apiKey.trim()}
        >
          {isPending
            ? 'Testing...'
            : submitLabel || (isEditing ? 'Save and reconnect' : 'Test Connection')}
        </button>
      </div>

      {connectMutation.isSuccess && !isEditing && (
        <div className="text-xs text-success bg-success-bg border border-success/30 rounded-md p-2.5">
          {successLabel || 'Connected!'} {connectMutation.data.details?.active_workflows || 0}{' '}
          active workflows found
        </div>
      )}

      {updateMutation.isSuccess && isEditing && (
        <div className="text-xs text-success bg-success-bg border border-success/30 rounded-md p-2.5">
          {successLabel || 'Updated!'}{' '}
          {updateMutation.data.target_info.details?.active_workflows || 0} active workflows found
        </div>
      )}

      {(connectMutation.isError || updateMutation.isError) && (
        <div className="text-xs text-error bg-error-bg border border-error/30 rounded-md p-2.5">
          {connectMutation.error?.message || updateMutation.error?.message || 'Connection failed'}
        </div>
      )}
    </form>
  );
}
