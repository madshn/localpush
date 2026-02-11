# Plan: Targets + Dashboard (Session 1)

## Execution Context

| Field | Value |
|-------|-------|
| **Working Directory** | `/Users/madsnissen/dev/localpush/.worktrees/targets-dashboard` |
| **Git Branch** | `feature/targets-dashboard` |
| **Worktree Mode** | true |

---

## Scope

Three features: Make.com connector (A), Zapier connector (B), Dashboard kanban view (C).

---

## Feature A: Make.com Connector

Full API-integrated target with webhook discovery.

### Step A1: Backend — `targets/make.rs`

**New file:** `src-tauri/src/targets/make.rs`

Implement `Target` trait:

```rust
pub struct MakeTarget {
    id: String,
    zone_url: String,    // e.g. "https://eu1.make.com"
    api_key: String,
    team_id: String,
    client: Client,
}
```

- `new(id, zone_url, api_key)` — trim trailing slash, store
- `test_connection()` — `GET {zone_url}/api/v2/teams` with `Authorization: Token {api_key}`. Extract first team_id. Return TargetInfo.
- `list_endpoints()` — `GET {zone_url}/api/v2/hooks?teamId={team_id}&typeName=gateway-webhook&assigned=true`. Map each hook to TargetEndpoint with `url` field (the `hook.*.make.com` URL). Include hook name, enabled status, linked scenario ID in metadata.

Auth header: `Authorization: Token {api_key}`

Delivery POSTs go directly to the hook URL (no auth header needed — URL is self-authenticating).

Register in `targets/mod.rs`.

### Step A2: Backend — Command + State

**File:** `src-tauri/src/commands/mod.rs`

Add `connect_make_target(zone_url: String, api_key: String)`:
- Create MakeTarget, call test_connection()
- Store in TargetManager
- Persist config: `target.{id}.type = "make"`, `target.{id}.url = zone_url`
- Store API key in credential store: `make:{id}`

**File:** `src-tauri/src/state.rs`

Add startup restoration for Make targets (same pattern as n8n):
- Read `target.*.type == "make"` from config
- Retrieve API key from credential store
- Reconstruct MakeTarget, add to TargetManager

**File:** `src-tauri/src/main.rs`

Register `connect_make_target` command.

### Step A3: Frontend — MakeConnect.tsx

**New file:** `src/components/MakeConnect.tsx`

Form with:
- Zone URL input (placeholder: "https://eu1.make.com") with helper text explaining zones
- API Token input (password field)
- "Connect" button → invokes `connect_make_target`
- Success/error states

Follow `N8nConnect.tsx` pattern exactly.

**File:** `src/components/TargetSetup.tsx`

Add "Make" tab alongside n8n and ntfy.

### Step A4: Tests

Unit tests in `make.rs`:
- Mock hook list response → verify endpoint extraction
- Mock auth failure → verify TargetError::AuthFailed
- Verify URL construction (hook URL vs API URL)

---

## Feature B: Zapier Connector

Simple paste-URL target (no API discovery).

### Step B1: Backend — `targets/zapier.rs`

**New file:** `src-tauri/src/targets/zapier.rs`

```rust
pub struct ZapierTarget {
    id: String,
    webhook_url: String,  // https://hooks.zapier.com/hooks/catch/...
    name: String,
    client: Client,
}
```

- `new(id, name, webhook_url)` — validate URL starts with `https://hooks.zapier.com/`
- `test_connection()` — POST a test payload `{"test": true, "source": "localpush"}` to webhook_url. 200 = success.
- `list_endpoints()` — return single endpoint with the webhook URL

No API key needed. URL is the auth.

Register in `targets/mod.rs`.

### Step B2: Backend — Command + State

**File:** `src-tauri/src/commands/mod.rs`

Add `connect_zapier_target(name: String, webhook_url: String)`:
- Validate URL domain
- Create ZapierTarget, test connection
- Store in TargetManager
- Persist: `target.{id}.type = "zapier"`, `target.{id}.url = webhook_url`, `target.{id}.name = name`
- No credential store needed (URL is the credential)

**File:** `src-tauri/src/state.rs`

Add startup restoration for Zapier targets.

**File:** `src-tauri/src/main.rs`

Register `connect_zapier_target` command.

### Step B3: Frontend — ZapierConnect.tsx

**New file:** `src/components/ZapierConnect.tsx`

Form with:
- Name input (e.g. "My Zap Webhook")
- Webhook URL input with validation (must be hooks.zapier.com)
- Inline instructions: "Create a Zap with 'Webhooks by Zapier' > 'Catch Hook', then paste the URL here"
- "Connect" button
- Success/error states

**File:** `src/components/TargetSetup.tsx`

Add "Zapier" tab.

### Step B4: Tests

Unit tests in `zapier.rs`:
- Valid URL accepted
- Invalid domain rejected
- Test connection with mock response

---

## Feature C: Dashboard Kanban View

Replace current PipelineView with the stitch_dash-v2 3-column flow layout.

### Reference

The prototype at `stitch-prototypes/stitch_dash-v2/` shows:
- Header: LocalPush logo + "Operational" badge + settings gear
- Summary stats: Total Deliveries (1,842 +12%) + Active Flows (4 of 10)
- Active Pipelines: 3-column grid per row: [Source] ---(count)--- [Target]
  - Animated dashed line connecting source to target
  - Delivery count badge in center circle
  - Unbound sources show dashed "Add Target" card
- Historical Velocity chart (7D/30D/90D bar chart)
- Activity Log preview (2-3 recent entries + "View all activity" link)

The prototype HTML is at `stitch-prototypes/stitch_dash-v2/code.html` — use it as direct reference for layout and CSS.

### Step C1: PipelineView Redesign

**File:** `src/components/PipelineView.tsx`

Replace the current vertical card list with:

1. **Summary stats row** — 2-card grid: Total Deliveries + Active Flows
2. **Active Pipelines section** — For each source with bindings:
   - 3-column grid: `[SourceCard] --- [CountBadge] --- [TargetCard]`
   - Animated dashed SVG line connecting source to target
   - Count badge shows total deliveries for that binding
   - Sources without bindings show `[SourceCard] --- [0] --- [+ Add Target]`
3. **Historical Velocity** — Bar chart placeholder (can be static/decorative for now, real data later)
4. **Activity Log preview** — Last 2-3 deliveries with "View all activity" link that switches to Activity tab

Keep the existing flow state management (enable flow, endpoint picker, etc.) — just change the layout.

### Step C2: PipelineCard → PipelineRow

Refactor `PipelineCard.tsx` or create new `PipelineRow.tsx`:

Each row is a `grid grid-cols-3` with:
- Left: Source card (icon, name, type badge)
- Center: SVG connector line + delivery count circle
- Right: Target card (icon, name, type) or "Add Target" dashed card

The animated dashed line uses CSS:
```css
.pulse-line {
    stroke-dasharray: 4;
    animation: dash 1s linear infinite;
}
@keyframes dash {
    to { stroke-dashoffset: -8; }
}
```

### Step C3: Activity Log Preview

Add inline activity log at bottom of Pipeline tab:
- Reuse existing `useActivityLog` hook
- Show last 3 entries (icon + source name + status + time)
- "View all activity" button switches to Activity tab

### Step C4: Preserve Existing Flows

The enable/bind/push flow modals must still work. When user clicks a source card or "Add Target", the existing flow state triggers. The visual change is the layout, not the interaction logic.

---

## Verification

```bash
# Backend
cd src-tauri && cargo test && cargo clippy -- -D warnings

# Frontend
npm run typecheck && npm run lint && npm test
```

---

## Implementation Order

| # | Step | Description |
|---|------|-------------|
| 0 | Verify context | cd to worktree, verify branch |
| 1 | A1-A2 | Make.com backend (target + command + state) |
| 2 | B1-B2 | Zapier backend (target + command + state) |
| 3 | A4 + B4 | Backend tests |
| 4 | `cargo test && cargo clippy` | Backend gate |
| 5 | A3 + B3 | Frontend connect forms |
| 6 | C1-C4 | Dashboard kanban redesign |
| 7 | `npm run typecheck && npm run lint && npm test` | Frontend gate |
| 8 | Commit + push + PR | Ready for review |
