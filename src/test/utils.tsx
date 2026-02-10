import { ReactNode } from 'react';
import {
  render as rtlRender,
  RenderOptions,
  renderHook as rtlRenderHook,
  RenderHookOptions,
} from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';

/**
 * Create a test QueryClient with sensible test defaults
 */
function createTestQueryClient() {
  return new QueryClient({
    defaultOptions: {
      queries: {
        // Disable retries and set instant stale time for tests
        retry: false,
        staleTime: 0,
      },
    },
  });
}

/**
 * Custom render function that wraps components with necessary providers
 * (React Query, etc.)
 */
export function render(
  ui: ReactNode,
  {
    initialState: _initialState,
    ...renderOptions
  }: RenderOptions & {
    initialState?: unknown;
  } = {}
) {
  const queryClient = createTestQueryClient();

  function Wrapper({ children }: { children: ReactNode }) {
    return (
      <QueryClientProvider client={queryClient}>
        {children}
      </QueryClientProvider>
    );
  }

  return rtlRender(ui, { wrapper: Wrapper, ...renderOptions });
}

/**
 * Custom renderHook that wraps hooks with React Query provider
 */
export function renderHook<TProps, TResult>(
  hook: (props: TProps) => TResult,
  options?: Omit<RenderHookOptions<TProps>, 'wrapper'>
) {
  const queryClient = createTestQueryClient();

  function Wrapper({ children }: { children: ReactNode }) {
    return (
      <QueryClientProvider client={queryClient}>
        {children}
      </QueryClientProvider>
    );
  }

  return rtlRenderHook(hook, { wrapper: Wrapper, ...options });
}

// Re-export common utilities from React Testing Library
export {
  screen,
  waitFor,
  fireEvent,
  within,
  act,
} from '@testing-library/react';
