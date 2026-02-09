import { useState } from 'react';
import { useTargets, useTestTargetConnection } from '../api/hooks/useTargets';
import { N8nConnect } from './N8nConnect';
import { NtfyConnect } from './NtfyConnect';
import { logger } from '../utils/logger';

type TargetType = 'n8n' | 'ntfy';

export function TargetSetup() {
  const [selectedType, setSelectedType] = useState<TargetType>('n8n');
  const [testingTargetId, setTestingTargetId] = useState<string | null>(null);
  const { data: targets, isLoading } = useTargets();
  const testMutation = useTestTargetConnection();

  const handleTargetConnected = (targetInfo: any) => {
    logger.info('Target connected successfully', { targetId: targetInfo.id, type: targetInfo.target_type });
  };

  const handleTestConnection = async (targetId: string) => {
    setTestingTargetId(targetId);
    try {
      await testMutation.mutateAsync(targetId);
    } catch (error) {
      logger.error('Target test failed', { targetId, error });
    } finally {
      setTestingTargetId(null);
    }
  };

  return (
    <div>
      {/* Connected Targets */}
      {!isLoading && targets && targets.length > 0 && (
        <div className="card" style={{ marginBottom: 12 }}>
          <h2 className="card-title">Connected Targets</h2>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
            {targets.map((target) => (
              <div
                key={target.id}
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'space-between',
                  padding: 8,
                  backgroundColor: 'var(--bg-secondary)',
                  borderRadius: 4,
                }}
              >
                <div style={{ flex: 1 }}>
                  <div style={{ fontWeight: 500, marginBottom: 2 }}>{target.name}</div>
                  <div style={{ display: 'flex', alignItems: 'center', gap: 8, fontSize: 12, color: 'var(--text-secondary)' }}>
                    <span
                      style={{
                        padding: '2px 6px',
                        backgroundColor: target.target_type === 'n8n' ? '#4f9eff33' : '#10b98133',
                        color: target.target_type === 'n8n' ? '#4f9eff' : '#10b981',
                        borderRadius: 3,
                        fontSize: 11,
                        fontWeight: 500,
                      }}
                    >
                      {target.target_type}
                    </span>
                  </div>
                </div>
                <button
                  className="btn-secondary"
                  onClick={() => handleTestConnection(target.id)}
                  disabled={testingTargetId === target.id}
                  style={{ fontSize: 12, padding: '4px 8px' }}
                >
                  {testingTargetId === target.id ? 'Testing...' : 'Test'}
                </button>
              </div>
            ))}
          </div>

          {testMutation.isSuccess && (
            <div className="status-message" style={{ backgroundColor: 'var(--success-bg)', color: 'var(--success-text)', marginTop: 8 }}>
              Connection test successful
            </div>
          )}

          {testMutation.isError && (
            <div className="status-message" style={{ backgroundColor: 'var(--error-bg)', color: 'var(--error-text)', marginTop: 8 }}>
              {testMutation.error.message || 'Connection test failed'}
            </div>
          )}
        </div>
      )}

      {/* Add Target */}
      <div className="card">
        <h2 className="card-title">Add Target</h2>

        {/* Target Type Tabs */}
        <div style={{ display: 'flex', gap: 8, marginBottom: 16, borderBottom: '1px solid var(--bg-secondary)' }}>
          <button
            onClick={() => setSelectedType('n8n')}
            style={{
              padding: '8px 16px',
              background: 'none',
              border: 'none',
              color: selectedType === 'n8n' ? 'var(--accent)' : 'var(--text-secondary)',
              borderBottom: selectedType === 'n8n' ? '2px solid var(--accent)' : '2px solid transparent',
              cursor: 'pointer',
              fontSize: 14,
              fontWeight: 500,
            }}
          >
            n8n
          </button>
          <button
            onClick={() => setSelectedType('ntfy')}
            style={{
              padding: '8px 16px',
              background: 'none',
              border: 'none',
              color: selectedType === 'ntfy' ? 'var(--accent)' : 'var(--text-secondary)',
              borderBottom: selectedType === 'ntfy' ? '2px solid var(--accent)' : '2px solid transparent',
              cursor: 'pointer',
              fontSize: 14,
              fontWeight: 500,
            }}
          >
            ntfy
          </button>
        </div>

        {/* Connection Forms */}
        {selectedType === 'n8n' && <N8nConnect onConnected={handleTargetConnected} />}
        {selectedType === 'ntfy' && <NtfyConnect onConnected={handleTargetConnected} />}
      </div>
    </div>
  );
}
