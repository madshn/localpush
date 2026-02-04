# LocalPush Testing Infrastructure

## Overview

LocalPush test infrastructure is built on **Vitest 2.x** with React Testing Library for component testing and full Tauri IPC mocking. The setup enables fast, reliable unit and component tests with guaranteed delivery semantics built in.

**Status:** Fully operational, all 10 tests passing.

## Quick Start

```bash
# Run tests once
npm test

# Watch mode (auto-rerun on file changes)
npm run test:watch

# Generate coverage report
npm run test:coverage
```

## Architecture

### Files

```
localpush/
├── vitest.config.ts           # Vitest configuration (jsdom, globals, coverage)
├── src/
│   ├── components/
│   │   ├── StatusIndicator.tsx
│   │   └── StatusIndicator.test.tsx  # ✓ 5 tests
│   ├── api/
│   │   └── hooks/
│   │       ├── useDeliveryStatus.ts
│   │       └── useDeliveryStatus.test.ts  # ✓ 5 tests
│   └── test/
│       ├── setup.ts            # Global mocking, test initialization
│       ├── mocks.ts            # Reusable mock data and fixtures
│       ├── utils.tsx           # Custom render() and renderHook()
│       └── README.md           # Test documentation
```

### Setup Chain

```
Test Run
  ↓
setupFiles: src/test/setup.ts
  ├─ Import @testing-library/jest-dom matchers
  ├─ Mock @tauri-apps/api/core (invoke, listen)
  ├─ Mock @tauri-apps/plugin-* (notification, process, updater)
  └─ Export mockInvoke for test-specific behavior
  ↓
Test imports src/test/utils.tsx
  ├─ Custom render() wraps with QueryClientProvider
  ├─ Custom renderHook() wraps with QueryClientProvider
  └─ Re-exports screen, waitFor, fireEvent, within, act
  ↓
Test runs with full mocking available
```

## Test Suites

### StatusIndicator Tests (5 tests, ✓ passing)

**Location:** `src/components/StatusIndicator.test.tsx`

Tests the visual indicator component that displays delivery status:

- ✓ Renders active status with correct label
- ✓ Renders pending status with correct label
- ✓ Renders error status with correct label
- ✓ Renders unknown status with correct label
- ✓ Applies correct CSS class for each status

**Usage:**
```typescript
render(<StatusIndicator status="active" />);
expect(screen.getByText('All delivered')).toBeInTheDocument();
```

### useDeliveryStatus Hook Tests (5 tests, ✓ passing)

**Location:** `src/api/hooks/useDeliveryStatus.test.ts`

Tests the React Query hook that fetches delivery status via Tauri IPC:

- ✓ Fetches delivery status successfully
- ✓ Handles pending status correctly
- ✓ Handles error status correctly
- ✓ Handles Tauri IPC errors gracefully
- ✓ Calls invoke with correct command name

**Usage:**
```typescript
mockInvoke.mockResolvedValue(mockDeliveryStatusActive);

const { result } = renderHook(() => useDeliveryStatus());

await waitFor(() => {
  expect(result.current.isSuccess).toBe(true);
});

expect(result.current.data).toEqual(mockDeliveryStatusActive);
expect(mockInvoke).toHaveBeenCalledWith('get_delivery_status');
```

## Mocking

### Tauri IPC Mocking

All Tauri APIs are mocked in `src/test/setup.ts`:

**Available mock functions:**
- `mockInvoke` — Mocks `@tauri-apps/api/core.invoke()`
- `mockListen` — Mocks event listeners (not used in tests yet)

**Usage in tests:**

```typescript
import { mockInvoke } from '../../test/setup';

// Success case
mockInvoke.mockResolvedValue(expectedData);

// Error case
mockInvoke.mockRejectedValue(new Error('Network error'));

// Verify IPC was called correctly
expect(mockInvoke).toHaveBeenCalledWith('get_delivery_status');
```

### Mock Data

Reusable fixtures in `src/test/mocks.ts`:

```typescript
// Delivery status variants
mockDeliveryStatusActive          // All delivered
mockDeliveryStatusPending         // 3 pending, 0 failed
mockDeliveryStatusError           // 1 pending, 2 failed
mockDeliveryStatusUnknown         // Loading state

// Queue entries
mockDeliveryEntry                 // Pending delivery
mockDeliveredEntry                // Successfully delivered
mockFailedEntry                   // Failed after retries

// Source configuration
mockSource                        // Enabled source
mockSourceDisabled                // Disabled source
mockSourcePreview                 // Preview data structure
```

**Usage:**
```typescript
import { mockDeliveryStatusActive } from '../../test/mocks';

mockInvoke.mockResolvedValue(mockDeliveryStatusActive);
```

## Custom Rendering

### `render()` Function

Wraps components with React Query provider:

```typescript
import { render, screen } from '../test/utils.tsx';

render(<StatusIndicator status="active" />);
expect(screen.getByText('All delivered')).toBeInTheDocument();
```

**What it provides:**
- QueryClientProvider with test-optimized settings
- No retry logic (retry: false)
- Instant staleness (staleTime: 0)

### `renderHook()` Function

Wraps hooks with React Query provider:

```typescript
import { renderHook, waitFor } from '../../test/utils.tsx';

const { result } = renderHook(() => useDeliveryStatus());

await waitFor(() => {
  expect(result.current.isSuccess).toBe(true);
});
```

**What it provides:**
- Same QueryClientProvider wrapper as render()
- Full React Testing Library hook API

## Coverage

Current coverage report:

```
%Stmts  | %Branch | %Funcs | %Lines
--------|---------|--------|--------
 100%   |  100%   |  100%  |  100%  (tested components/hooks)
  21%   |   72%   |   23%  |   21%  (overall, excluding untested files)
```

**Targets:** 70% for lines, functions, branches, statements (configured in `vitest.config.ts`)

**Tested files:**
- ✓ StatusIndicator.tsx (100% coverage)
- ✓ useDeliveryStatus.ts (100% coverage)

**Untested (add tests as features stabilize):**
- SourceList.tsx
- DeliveryQueue.tsx
- useDeliveryQueue.ts
- useSources.ts
- App.tsx

## Configuration

### vitest.config.ts

```typescript
export default defineConfig({
  plugins: [react()],
  test: {
    environment: 'jsdom',
    globals: true,
    setupFiles: ['./src/test/setup.ts'],
    include: ['src/**/*.{test,spec}.{ts,tsx}'],
    coverage: {
      provider: 'v8',
      reporter: ['text', 'json', 'html'],
      lines: 70,
      functions: 70,
      branches: 70,
      statements: 70,
    },
  },
});
```

**Key settings:**
- `environment: 'jsdom'` — Simulates DOM (required for component tests)
- `globals: true` — `describe`, `it`, `expect` available without imports
- `setupFiles` — Runs `src/test/setup.ts` before tests
- `coverage.provider: 'v8'` — Built-in coverage from Node.js

### tsconfig.json

No changes needed; Vitest respects existing TypeScript configuration.

## CI/CD Integration

Tests run as part of `npm run check`:

```bash
npm run check
# Runs:
#   1. ESLint linting
#   2. TypeScript type checking
#   3. Vitest (this infrastructure)
#   4. Cargo tests (Rust backend)
```

All must pass before deployment.

## Writing New Tests

### Component Test Template

```typescript
import { describe, it, expect } from 'vitest';
import { render, screen } from '../test/utils.tsx';
import { MyComponent } from './MyComponent';

describe('MyComponent', () => {
  it('renders with expected content', () => {
    render(<MyComponent />);
    expect(screen.getByText('Expected text')).toBeInTheDocument();
  });

  it('calls handler on user interaction', async () => {
    const handler = vi.fn();
    render(<MyComponent onAction={handler} />);

    await userEvent.click(screen.getByRole('button'));
    expect(handler).toHaveBeenCalled();
  });
});
```

### Hook Test Template

```typescript
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderHook, waitFor } from '../../test/utils.tsx';
import { useMyHook } from './useMyHook';
import { mockInvoke } from '../../test/setup';

describe('useMyHook', () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  it('fetches data successfully', async () => {
    const expectedData = { /* ... */ };
    mockInvoke.mockResolvedValue(expectedData);

    const { result } = renderHook(() => useMyHook());

    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true);
    });

    expect(result.current.data).toEqual(expectedData);
  });

  it('handles errors gracefully', async () => {
    mockInvoke.mockRejectedValue(new Error('API failed'));

    const { result } = renderHook(() => useMyHook());

    await waitFor(() => {
      expect(result.current.isError).toBe(true);
    });

    expect(result.current.error).toEqual(new Error('API failed'));
  });
});
```

## Common Patterns

### Testing React Query Integration

```typescript
// Reset mocks before each test
beforeEach(() => {
  mockInvoke.mockReset();
});

// Mock the IPC response
mockInvoke.mockResolvedValue(data);

// Render and wait for query to resolve
const { result } = renderHook(() => useMyQuery());
await waitFor(() => {
  expect(result.current.isSuccess).toBe(true);
});

// Assert data and behavior
expect(result.current.data).toEqual(data);
```

### Testing with User Events

```typescript
import { userEvent } from '@testing-library/react';

const user = userEvent.setup();
render(<Component />);

await user.click(screen.getByRole('button'));
expect(screen.getByText('Updated')).toBeInTheDocument();
```

### Debugging Tests

```typescript
// Print rendered DOM
screen.debug();

// Print specific element
screen.debug(screen.getByRole('button'));

// Run in watch mode for better debugging
npm run test:watch
```

## Troubleshooting

### Issue: "No QueryClient set"

**Cause:** Hook test not using custom `renderHook()` wrapper

**Fix:** Import from `src/test/utils.tsx` not `@testing-library/react`:
```typescript
// ✗ Wrong
import { renderHook } from '@testing-library/react';

// ✓ Correct
import { renderHook } from '../../test/utils.tsx';
```

### Issue: Tauri IPC not mocked

**Cause:** Mock not set before hook renders

**Fix:** Set mock in test body before renderHook:
```typescript
mockInvoke.mockResolvedValue(data);  // Set BEFORE
const { result } = renderHook(() => useMyQuery());  // Then render
```

### Issue: Tests timeout

**Cause:** Forgetting to use `waitFor()` for async operations

**Fix:** Always wait for async state changes:
```typescript
await waitFor(() => {
  expect(result.current.isSuccess).toBe(true);
});
```

## Next Steps

1. **Add tests for remaining components** as they stabilize (SourceList, DeliveryQueue)
2. **Add integration tests** for complete user flows (add source → deliver → track)
3. **Add E2E tests** with Tauri's test runner for Rust backend integration
4. **Monitor coverage** during development; aim for 70%+ on all new code

## References

- [Vitest Documentation](https://vitest.dev/)
- [React Testing Library](https://testing-library.com/react)
- [Testing Library Best Practices](https://kentcdodds.com/blog/common-mistakes-with-react-testing-library)
- [Tauri IPC API](https://tauri.app/docs/guides/command/)
- [React Query Documentation](https://tanstack.com/query/latest)

---

**Last Updated:** 2026-02-04
**Test Framework:** Vitest 2.1.9
**React Testing Library:** 14.1.0
**Coverage Provider:** v8
