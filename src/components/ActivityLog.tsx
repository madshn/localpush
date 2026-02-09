import { useState } from 'react';
import { useActivityLog, type ActivityEntry } from '../api/hooks/useActivityLog';

export function ActivityLog() {
  const { data: entries, isLoading } = useActivityLog();
  const [expandedId, setExpandedId] = useState<string | null>(null);

  const toggleExpanded = (id: string) => {
    setExpandedId(expandedId === id ? null : id);
  };

  const formatTime = (date: Date): string => {
    return date.toLocaleTimeString('en-US', {
      hour12: false,
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit'
    });
  };

  const formatFullTimestamp = (date: Date): string => {
    return date.toLocaleString('en-US', {
      year: 'numeric',
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
      hour12: false
    });
  };

  const getStatusDisplay = (entry: ActivityEntry): { icon: string; text: string; color: string } => {
    switch (entry.status) {
      case 'delivered':
        return {
          icon: '✓',
          text: entry.statusCode || 'Delivered',
          color: 'var(--success)'
        };
      case 'pending':
        return {
          icon: '○',
          text: 'Pending',
          color: 'var(--warning)'
        };
      case 'in_flight':
        return {
          icon: '→',
          text: 'Sending...',
          color: 'var(--warning)'
        };
      case 'failed':
        return {
          icon: '✕',
          text: entry.error || 'Failed',
          color: 'var(--error)'
        };
      case 'dlq':
        return {
          icon: '☠',
          text: entry.error ? `Dead letter: ${entry.error}` : 'Dead letter',
          color: 'var(--error)'
        };
      default:
        return {
          icon: '?',
          text: 'Unknown',
          color: 'var(--text-secondary)'
        };
    }
  };

  if (isLoading) {
    return (
      <div className="card" style={{ padding: '20px', textAlign: 'center' }}>
        <p style={{ color: 'var(--text-secondary)' }}>Loading activity...</p>
      </div>
    );
  }

  if (!entries || entries.length === 0) {
    return (
      <div className="card" style={{ padding: '20px', textAlign: 'center' }}>
        <p style={{ color: 'var(--text-secondary)' }}>
          No deliveries yet. Enable a source to start pushing data.
        </p>
      </div>
    );
  }

  return (
    <div className="card" style={{ padding: '12px' }}>
      <h3 className="card-title" style={{ marginBottom: '12px' }}>Activity Log</h3>
      <div style={{
        display: 'flex',
        flexDirection: 'column',
        gap: '4px',
        maxHeight: '400px',
        overflowY: 'auto'
      }}>
        {entries.map(entry => {
          const status = getStatusDisplay(entry);
          const isExpanded = expandedId === entry.id;

          return (
            <div key={entry.id} style={{ marginBottom: '4px' }}>
              <div
                onClick={() => toggleExpanded(entry.id)}
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: '8px',
                  padding: '6px 8px',
                  backgroundColor: 'var(--bg-secondary)',
                  borderRadius: '4px',
                  cursor: 'pointer',
                  fontSize: '13px',
                  transition: 'background-color 0.15s',
                }}
                onMouseEnter={(e) => {
                  e.currentTarget.style.backgroundColor = 'var(--bg-primary)';
                }}
                onMouseLeave={(e) => {
                  e.currentTarget.style.backgroundColor = 'var(--bg-secondary)';
                }}
              >
                <span style={{
                  fontFamily: '"SF Mono", Monaco, "Cascadia Code", monospace',
                  color: 'var(--text-secondary)',
                  fontSize: '12px',
                  minWidth: '65px'
                }}>
                  {formatTime(entry.timestamp)}
                </span>

                <span style={{
                  color: 'var(--text-primary)',
                  minWidth: '100px',
                  fontSize: '13px'
                }}>
                  {entry.source}
                </span>

                <span style={{
                  color: status.color,
                  fontSize: '14px',
                  fontWeight: 'bold'
                }}>
                  {status.icon}
                </span>

                <span style={{
                  color: status.color,
                  fontSize: '12px',
                  flex: 1,
                  overflow: 'hidden',
                  textOverflow: 'ellipsis',
                  whiteSpace: 'nowrap'
                }}>
                  {status.text}
                  {(entry.status === 'failed' || entry.status === 'dlq') && entry.retryCount > 0 && (
                    <span style={{ color: 'var(--text-secondary)', marginLeft: '4px' }}>
                      (retry {entry.retryCount}/5)
                    </span>
                  )}
                </span>

                <span style={{
                  color: 'var(--text-secondary)',
                  fontSize: '12px'
                }}>
                  {isExpanded ? '▼' : '▶'}
                </span>
              </div>

              {isExpanded && (
                <div style={{
                  marginTop: '4px',
                  padding: '8px 12px',
                  backgroundColor: 'var(--bg-primary)',
                  borderRadius: '4px',
                  fontSize: '12px',
                  fontFamily: '"SF Mono", Monaco, "Cascadia Code", monospace',
                  color: 'var(--text-secondary)',
                  lineHeight: '1.6'
                }}>
                  <div><strong>ID:</strong> {entry.id}</div>
                  <div><strong>Source:</strong> {entry.source}</div>
                  <div><strong>Status:</strong> {entry.status}</div>
                  <div><strong>Created:</strong> {formatFullTimestamp(entry.timestamp)}</div>
                  {entry.deliveredAt && (
                    <div><strong>Delivered:</strong> {formatFullTimestamp(entry.deliveredAt)}</div>
                  )}
                  <div><strong>Retry count:</strong> {entry.retryCount}</div>
                  {entry.error && (
                    <div style={{ color: 'var(--error)', marginTop: '4px' }}>
                      <strong>Error:</strong> {entry.error}
                    </div>
                  )}
                </div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}
