# Test Infrastructure

LocalPush uses **Vitest** for unit and component tests with React Testing Library for component testing and mocked Tauri IPC.

## Structure

```
src/test/
├── setup.ts       # Global test setup, Tauri mocking
├── mocks.ts       # Reusable mock data and fixtures
├── utils.ts       # Custom render function with React Query
└── README.md      # This file
```

## Setup Files

### `setup.ts`
- Initializes Vitest globals
- Mocks all Tauri APIs (@tauri-apps/api/core, plugins)
- Resets mocks between tests
- Exports `mockInvoke` for test-specific IPC mocking

### `mocks.ts`
Provides reusable mock data:
- `mockDeliveryStatus*` — DeliveryStatus variants (active, pending, error, unknown)
- `mockDeliveryEntry` — Pending delivery queue item
- `mockDeliveredEntry` — Successfully delivered item
- `mockFailedEntry` — Failed delivery after retries
- `mockSource` — Sample source configuration
- `mockSourceDisabled` — Disabled source configuration
- `mockSourcePreview` — Sample source preview data

### `utils.ts`
Custom render function that wraps components with:
- React Query QueryClientProvider (with test-optimized defaults)
- Returns RTL's full API for convenience

## Running Tests

```bash
# Run tests once
npm test

# Watch mode (re-run on file changes)
npm run test:watch

# Generate coverage report
npm run test:coverage
```

## Writing Tests

### Component Tests

Use the custom `render` from `src/test/utils`:

```typescript
import { describe, it, expect } from 'vitest';
import { render, screen } from '../test/utils';
import { StatusIndicator } from './StatusIndicator';

describe('StatusIndicator', () => {
  it('renders active status', () => {
    render(<StatusIndicator status="active" />);
    expect(screen.getByText('All delivered')).toBeInTheDocument();
  });
});
```

### Hook Tests with Tauri IPC

Mock `invoke` in your test, then use `renderHook`:

```typescript
import { renderHook, waitFor } from '@testing-library/react';
import { useDeliveryStatus } from './useDeliveryStatus';
import { mockInvoke } from '../../test/setup';
import { mockDeliveryStatusActive } from '../../test/mocks';

describe('useDeliveryStatus', () => {
  it('fetches status successfully', async () => {
    mockInvoke.mockResolvedValue(mockDeliveryStatusActive);

    const { result } = renderHook(() => useDeliveryStatus());

    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true);
    });

    expect(result.current.data).toEqual(mockDeliveryStatusActive);
    expect(mockInvoke).toHaveBeenCalledWith('get_delivery_status');
  });
});
```

### Testing Tauri IPC Errors

```typescript
it('handles IPC errors gracefully', async () => {
  const error = new Error('IPC communication failed');
  mockInvoke.mockRejectedValue(error);

  const { result } = renderHook(() => useDeliveryStatus());

  await waitFor(() => {
    expect(result.current.isError).toBe(true);
  });

  expect(result.current.error).toEqual(error);
});
```

## Key Patterns

### Mocking IPC Responses
```typescript
import { mockInvoke } from '../../test/setup';

mockInvoke.mockResolvedValue(expectedData); // Success
mockInvoke.mockRejectedValue(new Error('...')); // Error
```

### Waiting for Async Operations
```typescript
import { waitFor } from '@testing-library/react';

await waitFor(() => {
  expect(result.current.isSuccess).toBe(true);
});
```

### React Query Test Defaults
The custom `render` function sets:
- `retry: false` — No retry logic during tests
- `staleTime: 0` — Always consider data stale (triggers refetch)

This prevents flaky tests and ensures predictable behavior.

## Coverage Thresholds

Current targets: **70%** for lines, functions, branches, statements.

Update `vitest.config.ts` coverage settings as needed.

## Best Practices

1. **Reset mocks between tests** — `beforeEach(() => mockInvoke.mockReset())` is already in setup.ts, but test-specific resets are fine too
2. **Use waitFor for async** — Don't rely on timeouts; wait for the specific condition
3. **Test user behavior, not implementation** — Use `screen.getByText` not `container.querySelector`
4. **Mock Tauri early** — The setup.ts handles most, but test-specific mocks override it
5. **Keep mock data in mocks.ts** — Reusable fixtures prevent duplication

## Debugging Tests

Run in watch mode with extra logging:

```bash
npm run test:watch
```

In tests, use `screen.debug()` to see rendered DOM:

```typescript
it('renders something', () => {
  render(<Component />);
  screen.debug(); // Prints DOM to console
  // ... assertions
});
```

## Integration with CI/CD

Tests run as part of the `npm run check` command, which also includes:
- ESLint linting
- TypeScript type checking
- Cargo tests (Rust backend)

Ensure all pass before committing.
