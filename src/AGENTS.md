# LocalPush Frontend (Codex Guide)

Frontend guidance for `src/` (React + TypeScript + Tauri IPC). This mirrors the portable parts of `src/CLAUDE.md`.

## Role

The frontend should:

- Display app state clearly
- Capture user input safely
- Invoke backend commands via Tauri IPC
- Reflect delivery progress and failures in near real time

## Stack (Current)

- React 18
- TypeScript
- Vite
- TanStack Query
- Zustand (UI-only state)
- Tauri IPC
- Vitest

## Architecture (High-Level)

Key app areas:

- Pipeline/Home view (source cards, preview, binding/endpoint setup)
- Activity log (delivery history)
- Settings/targets (connect/manage targets)

Common pattern:

- `api/hooks/*` wrap backend IPC commands
- UI components consume hooks
- Server state lives in React Query
- UI-only state lives in Zustand

## Core Patterns

### 1) Hooks Wrap Tauri Commands

Prefer a dedicated React Query hook per backend command/query in `api/hooks/`.

- Stable `queryKey`
- Typed return values where available
- Polling only where the UX benefits (status, queue, activity)

### 2) Zustand for UI State Only

Use Zustand for:

- Modal open/close state
- Local view toggles
- Temporary form/UI state

Do not use Zustand for server state that should be cached/refetched/invalidation-driven.

### 3) Error Handling

- Backend command errors often arrive as serialized strings from Tauri `Result<T, String>`.
- Surface user-actionable errors in the UI.
- Avoid swallowing errors in event handlers; either display or log them.

## Component Conventions

- Always type component props (`interface Props`).
- Destructure props in function signature.
- Prefer passing IDs and loading fresh data in leaf components when data can go stale.
- Render loading / error / success states explicitly.

## Mutation / Events

When mutating backend state from UI:

- `await` the IPC call
- Handle errors explicitly
- Invalidate or update relevant React Query caches

## Styling

- Prefer CSS classes (`styles.css` / component classes).
- Avoid inline styles unless there is a clear reason.
- Keep styles readable and component-scoped by naming convention.

## Testing

Run:

```bash
npm test
npm test:watch
npm test:coverage
```

Patterns:

- Mock Tauri IPC (`invoke`) in tests.
- Wrap components with a test `QueryClientProvider`.
- Test loading/error/success states, not only happy-path rendering.

## Coordination with Backend

- IPC contracts are part of the frontend/backend boundary. If a command payload/response changes, update both sides in the same change set.
- Prefer small, explicit command payloads and predictable response shapes.
