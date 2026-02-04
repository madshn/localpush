# LocalPush Frontend

React 18 + TypeScript menu bar UI for LocalPush. Communicates with Rust backend via Tauri IPC.

**Role:** Display app state, capture user input, invoke backend commands, reflect delivery progress in real-time.

---

## Stack

| Tool | Version | Purpose |
|------|---------|---------|
| React | 18.3 | UI framework |
| TypeScript | 5.6 | Type safety |
| Vite | 6.0 | Build + dev server |
| TanStack Query | 5.0 | Data fetching + caching |
| Zustand | 5.0 | Local state (UI flags, modals) |
| Tauri IPC | 2.0 | Backend command bridge |
| Vitest | 2.0 | Unit tests |

---

## Architecture

```
App.tsx (Router)
├── StatusIndicator    # Overall app health (green/yellow/red)
├── SourceList         # Add/configure sources (uses useSources hook)
├── DeliveryQueue      # In-flight deliveries (uses useDeliveryQueue hook)
├── TransparencyPreview (modal)  # Show user's real data before enabling
└── SettingsPanel      # Auth config, webhook URL, etc.

api/hooks/
├── useDeliveryStatus  # Query: get current delivery stats
├── useSources         # Query: list configured sources
└── useDeliveryQueue   # Query: in-flight deliveries
```

---

## Key Patterns

### Hooks = Tauri IPC Wrappers

Every backend command gets a React Query hook in `api/hooks/`:

```typescript
// src/api/hooks/useSources.ts
export function useSources() {
  return useQuery({
    queryKey: ["sources"],
    queryFn: async () => {
      return await invoke("get_sources", {});
    },
    refetchInterval: 5000,  // Poll backend every 5s
  });
}
```

**Use in components:**
```tsx
function SourceList() {
  const { data: sources, isLoading } = useSources();

  if (isLoading) return <div>Loading...</div>;
  return sources.map(s => <SourceItem key={s.id} source={s} />);
}
```

### Local State = Zustand Stores

Zustand for UI-only state (modals, form inputs, view mode):

```typescript
// src/store.ts
export const useUIStore = create((set) => ({
  isSettingsOpen: false,
  toggleSettings: () => set((s) => ({ isSettingsOpen: !s.isSettingsOpen })),
}));
```

**DO NOT** use Zustand for server state (use TanStack Query instead).

### Error Handling

```typescript
const { data, isError, error } = useDeliveryStatus();

if (isError) {
  return <div className="error">{error.message}</div>;
}
```

Backend errors come as `String` in Tauri's `Result` type. Display directly to user or log.

---

## Component Guidelines

### Structure

```tsx
// src/components/MyComponent.tsx

import { useQuery } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";

interface Props {
  sourceId: string;
}

export function MyComponent({ sourceId }: Props) {
  // 1. Fetch data
  const { data, isLoading, error } = useQuery({
    queryKey: ["deliveries", sourceId],
    queryFn: async () => invoke("get_deliveries", { sourceId }),
  });

  // 2. Render states
  if (isLoading) return <div className="loading">Loading...</div>;
  if (error) return <div className="error">Error: {error.message}</div>;

  // 3. Render success
  return (
    <div className="component">
      {data?.items.map(item => (
        <div key={item.id}>{item.name}</div>
      ))}
    </div>
  );
}
```

### Props

- **Always typed** — `interface Props { ... }`
- **Destructured in function signature**
- **Pass IDs down, not full objects** — Fetch fresh data in leaf components

### Event Handlers

```tsx
const handleClick = async () => {
  try {
    await invoke("update_webhook_url", { url: newUrl });
    // Optionally invalidate cache to refetch
    queryClient.invalidateQueries({ queryKey: ["deliveries"] });
  } catch (err) {
    setError(String(err));
  }
};
```

---

## Styling

Use `styles.css` (kept minimal). Classes follow component names:

```css
.app { display: flex; flex-direction: column; }
.app-header { padding: 8px; border-bottom: 1px solid #e0e0e0; }
.status-indicator { width: 12px; height: 12px; border-radius: 50%; }
.status-indicator.ok { background: #4caf50; }
.status-indicator.warning { background: #ff9800; }
.status-indicator.error { background: #f44336; }
```

**Avoid inline styles.** Use CSS classes or Tailwind if added later.

---

## Testing

### Setup

```bash
npm test              # Run Vitest
npm test:watch       # Watch mode
npm test:coverage    # Coverage report
```

### Pattern: Mock Tauri Commands

```typescript
// src/__mocks__/tauri.ts
import { vi } from "vitest";

export const mockInvoke = vi.fn();

export const mockQueryClient = () => {
  return {
    invalidateQueries: vi.fn(),
    setQueryData: vi.fn(),
  };
};
```

### Example Test

```typescript
import { render, screen } from "@testing-library/react";
import { QueryClientProvider, QueryClient } from "@tanstack/react-query";
import { SourceList } from "./SourceList";
import { mockInvoke } from "../__mocks__/tauri";

describe("SourceList", () => {
  it("displays sources", async () => {
    mockInvoke.mockResolvedValue([
      { id: "1", name: "Claude Stats" },
    ]);

    const client = new QueryClient();
    render(
      <QueryClientProvider client={client}>
        <SourceList />
      </QueryClientProvider>
    );

    expect(await screen.findByText("Claude Stats")).toBeInTheDocument();
  });
});
```

---

## Key Files

| File | Purpose |
|------|---------|
| `App.tsx` | Main UI router, nav tabs |
| `api/hooks/useDeliveryStatus.ts` | Query delivery overall status |
| `api/hooks/useSources.ts` | Query configured sources |
| `api/hooks/useDeliveryQueue.ts` | Query in-flight deliveries |
| `components/StatusIndicator.tsx` | Color-coded health indicator |
| `components/SourceList.tsx` | List + add sources |
| `components/DeliveryQueue.tsx` | Show pending deliveries |
| `components/TransparencyPreview.tsx` | Preview real data modal |
| `components/SettingsPanel.tsx` | Configure auth, webhook URL |
| `store.ts` | Zustand UI state store |
| `styles.css` | Minimal CSS |

---

## Common Tasks

### Add a New Hook (Backend Query)

1. Create `src/api/hooks/useNewFeature.ts`
2. Use `useQuery` to wrap Tauri `invoke("command_name", { args })`
3. Export from `src/api/hooks/index.ts`
4. Use in components

Example:
```typescript
export function useWebhookStatus() {
  return useQuery({
    queryKey: ["webhookStatus"],
    queryFn: async () => invoke("get_webhook_status", {}),
    refetchInterval: 3000,
  });
}
```

### Add a New Component

1. Create `src/components/MyComponent.tsx`
2. Use hooks from `api/hooks/`
3. Import in `App.tsx` and render in appropriate view
4. Add test file `src/components/MyComponent.test.tsx`

### Handle Command Errors

All Tauri commands return `Result<T>`. On error:

```typescript
try {
  await invoke("save_webhook_url", { url });
} catch (err) {
  // err is a string (Tauri error)
  setErrorMessage(String(err));
  // Show in UI or toast notification
}
```

---

## Development Mode

```bash
npm run tauri dev
```

Opens dev server on `http://localhost:1420`, auto-reloads on file changes.

### Tips

- **React DevTools** — Available in dev mode (F12)
- **Inspect Backend Logs** — Check stdout for `RUST_LOG` output
- **Reload on Backend Change** — Auto-reload works; restart `npm run tauri dev` if changes don't appear

---

## TypeScript Strict Mode

All code must compile with `tsc --strict`:

```bash
npm run typecheck
```

**Rules:**
- No implicit `any`
- All event types explicitly typed
- Return types explicit on all functions
- No `// @ts-ignore`

If adding a library, ensure it has `@types/*` package or inline `.d.ts`.

---

## Performance

### Query Caching

TanStack Query caches by default. Adjust `staleTime` and `gcTime`:

```typescript
useQuery({
  queryKey: ["deliveries"],
  queryFn: () => invoke("get_deliveries", {}),
  staleTime: 10 * 1000,   // 10s before "stale"
  gcTime: 5 * 60 * 1000,  // 5m before garbage collected
});
```

### Avoid Re-renders

```tsx
// GOOD: Memoize if props are stable
const MyComponent = React.memo(({ id }: { id: string }) => {
  // ...
});

// BAD: Component re-renders on every parent render
const MyComponent = ({ id }: { id: string }) => {
  // ...
};
```

---

## References

- **Root Instructions:** `../CLAUDE.md`
- **Backend Instructions:** `../src-tauri/CLAUDE.md`
- **Main Plan:** `../PLAN.md`
- **TanStack Query:** https://tanstack.com/query/latest
- **Zustand:** https://github.com/pmndrs/zustand
- **Tauri IPC:** https://tauri.app/en/develop/calling-rust/
