import { DeliveryStatus } from '../api/hooks/useDeliveryStatus';

/**
 * Mock DeliveryStatus - all delivered
 */
export const mockDeliveryStatusActive: DeliveryStatus = {
  overall: 'active',
  pendingCount: 0,
  failedCount: 0,
  lastDelivery: new Date(Date.now() - 60000).toISOString(), // 1 minute ago
};

/**
 * Mock DeliveryStatus - pending deliveries
 */
export const mockDeliveryStatusPending: DeliveryStatus = {
  overall: 'pending',
  pendingCount: 3,
  failedCount: 0,
  lastDelivery: new Date(Date.now() - 300000).toISOString(), // 5 minutes ago
};

/**
 * Mock DeliveryStatus - delivery errors
 */
export const mockDeliveryStatusError: DeliveryStatus = {
  overall: 'error',
  pendingCount: 1,
  failedCount: 2,
  lastDelivery: new Date(Date.now() - 3600000).toISOString(), // 1 hour ago
};

/**
 * Mock DeliveryStatus - unknown/loading
 */
export const mockDeliveryStatusUnknown: DeliveryStatus = {
  overall: 'unknown',
  pendingCount: 0,
  failedCount: 0,
  lastDelivery: null,
};

/**
 * Sample source preview structure
 */
export const mockSourcePreview = {
  title: 'Claude Code Statistics',
  summary: '1,234 tokens today (+15%)',
  fields: [
    { label: 'Messages', value: '42', sensitive: false },
    { label: 'API Calls', value: '156', sensitive: false },
    { label: 'API Key', value: 'sk_***', sensitive: true },
  ],
  lastUpdated: new Date().toISOString(),
};

/**
 * Sample delivery queue entry
 */
export const mockDeliveryEntry = {
  id: '550e8400-e29b-41d4-a716-446655440000',
  sourceId: 'source-001',
  webhookUrl: 'https://example.com/webhook',
  payload: {
    event: 'file_change',
    file: '/path/to/file.ts',
    timestamp: new Date().toISOString(),
  },
  status: 'pending' as const,
  retryCount: 0,
  maxRetries: 3,
  createdAt: new Date().toISOString(),
  nextRetry: new Date(Date.now() + 30000).toISOString(),
};

/**
 * Sample delivered entry (success)
 */
export const mockDeliveredEntry = {
  id: '550e8400-e29b-41d4-a716-446655440001',
  sourceId: 'source-001',
  webhookUrl: 'https://example.com/webhook',
  payload: {
    event: 'file_change',
    file: '/path/to/file.ts',
    timestamp: new Date().toISOString(),
  },
  status: 'delivered' as const,
  retryCount: 0,
  maxRetries: 3,
  createdAt: new Date(Date.now() - 60000).toISOString(),
  deliveredAt: new Date().toISOString(),
};

/**
 * Sample failed entry (after max retries)
 */
export const mockFailedEntry = {
  id: '550e8400-e29b-41d4-a716-446655440002',
  sourceId: 'source-002',
  webhookUrl: 'https://example.com/webhook-failing',
  payload: {
    event: 'file_change',
    file: '/path/to/file.ts',
    timestamp: new Date().toISOString(),
  },
  status: 'failed' as const,
  retryCount: 3,
  maxRetries: 3,
  error: 'Connection timeout',
  createdAt: new Date(Date.now() - 3600000).toISOString(),
  failedAt: new Date(Date.now() - 60000).toISOString(),
};

/**
 * Sample source configuration
 */
export const mockSource = {
  id: 'source-001',
  name: 'Project Repo',
  description: 'Main project repository files',
  path: '/Users/dev/project',
  webhookUrl: 'https://example.com/webhook',
  enabled: true,
  patterns: [
    '**/*.ts',
    '**/*.tsx',
    '!node_modules/**',
  ],
  createdAt: new Date().toISOString(),
  updatedAt: new Date().toISOString(),
};

/**
 * Sample disabled source
 */
export const mockSourceDisabled = {
  ...mockSource,
  id: 'source-002',
  name: 'Backup Repo',
  enabled: false,
};
