import { expect, afterEach, vi, beforeEach } from 'vitest';
import '@testing-library/jest-dom';

// Mock Tauri IPC
export const mockInvoke = vi.fn();
export const mockListen = vi.fn();

vi.mock('@tauri-apps/api/core', () => ({
  invoke: mockInvoke,
  listen: mockListen,
}));

// Mock Tauri Notification plugin
vi.mock('@tauri-apps/plugin-notification', () => ({
  isPermissionGranted: vi.fn(async () => true),
  requestPermission: vi.fn(async () => 'granted'),
  sendNotification: vi.fn(),
}));

// Mock Tauri Process plugin
vi.mock('@tauri-apps/plugin-process', () => ({
  exit: vi.fn(),
  relaunch: vi.fn(),
}));

// Mock Tauri Updater plugin
vi.mock('@tauri-apps/plugin-updater', () => ({
  checkUpdate: vi.fn(async () => null),
  installUpdate: vi.fn(),
}));

// Reset all mocks between tests
beforeEach(() => {
  mockInvoke.mockReset();
  mockListen.mockReset();
});

afterEach(() => {
  vi.clearAllMocks();
});
