# UX Constitution: LocalPush

**Created:** 2026-02-09
**Version:** 1.0
**Status:** Baseline

---

## Overview

This document captures the UX intent for LocalPush across 6 structured passes. It serves as the single source of truth for all UI decisions and generates a Visual Assistant Prompt for tools like Google Stitch, Polymet, or v0.dev.

**Reference apps:** Dato (tray popup), Raycast (full settings window)
**Visual language:** Dark mode only, macOS-native feel

---

## Pass 1: Mental Model

> LocalPush is a **menu bar health indicator** for your local data pipeline.

Like Dato shows your next meeting at a glance, LocalPush shows whether your data is flowing. The tray icon is the primary surface: green means "all good," yellow/red means "review me."

**User thinks:** "Is my data flowing?"
**User does NOT think:** "Let me configure my webhook pipes."

**Analogies that work:**
- "Dropbox's menu bar icon, but for webhooks" — the status dot tells you everything
- "Time Machine's menu bar icon" — it just works, you check it when something feels off
- "Dato but for data delivery" — click for quick status, open settings when needed

**Analogies that fail:**
- "An API configuration tool" — too technical
- "A webhook dashboard" — implies monitoring, not status-at-a-glance
- "A file sync app" — wrong mental model, LocalPush pushes, doesn't sync

---

## Pass 2: Information Architecture

### Two UI Surfaces

LocalPush has two distinct interaction surfaces, each with a clear purpose.

#### Surface 1: Tray Popup (Dato-style)

**Purpose:** Status at-a-glance. Zero configuration. Quick check.
**Size:** Compact (~300px wide, height varies with content)
**Trigger:** Click menu bar icon

**Content hierarchy:**
1. **Health banner** — overall status (green/yellow/red) with summary text
2. **Source status list** — each enabled source with traffic light + last delivery time
3. **Quick counts** — pending, failed, delivered today
4. **Footer action** — "Open LocalPush" button to launch full window

**What is NOT in the popup:**
- No enable/disable toggles
- No configuration forms
- No detailed error messages
- No activity log

#### Surface 2: Full Window (Raycast-style)

**Purpose:** Configuration, details, investigation, onboarding.
**Size:** Standard window (~800x600, resizable)
**Trigger:** "Open LocalPush" from popup, or direct app launch

**Tab structure:**
| Tab | Purpose | Primary Objects |
|-----|---------|----------------|
| **Sources** | Enable/disable, bind, push | Source cards with enable flow, binding list, Push Now |
| **Activity** | Delivery history | Chronological log with expandable detail rows |
| **Settings** | Target connections, preferences | Target setup (n8n/ntfy), general settings |

### Core Objects

| Object | User-Facing Name | Description |
|--------|-----------------|-------------|
| Source | "Source" or by name ("Claude Stats") | A local data provider |
| Target | "Target" or by name ("My n8n") | A connected destination platform |
| Endpoint | "Webhook" or "Destination" | A specific URL within a target |
| Binding | Hidden concept — expressed as "connected to [endpoint]" | Source-to-endpoint wire |
| Ledger entry | "Delivery" | A single push attempt with status |

**Key principle:** Users never need to understand "bindings" as a concept. They enable a source, pick where data goes, and it's connected. The technical wiring is invisible.

---

## Pass 3: Affordances

### Must Be Obvious (No Explanation Needed)

| Affordance | Surface | How It Works |
|------------|---------|-------------|
| **Tray icon = app is running** | Menu bar | Icon visible means LocalPush is active |
| **Icon color = overall health** | Menu bar | Green = good, yellow = pending, red = failures, grey = idle |
| **Click icon = see status** | Menu bar | Popup appears instantly |
| **Traffic light per source** | Popup + Full window | Colored dot next to each source name |
| **"Open LocalPush" = go deeper** | Popup | Clear CTA button at popup footer |
| **Enable/Disable** | Full window | Button toggles source state |
| **Push Now** | Full window | Button on enabled sources, sends data immediately |

### Should Be Discoverable (Learn Once)

| Affordance | Surface | How It Works |
|------------|---------|-------------|
| **Transparency preview** | Full window | Shown automatically during enable flow |
| **Endpoint picker** | Full window | Dropdown of discovered webhooks during enable flow |
| **Activity detail expand** | Full window | Click row to see delivery details |
| **Target test connection** | Full window (Settings) | "Test" button next to connected targets |

### Can Be Hidden (Power User)

| Affordance | Surface | How It Works |
|------------|---------|-------------|
| **Custom delivery headers** | Full window | Optional step in enable flow |
| **Retry failed delivery** | Full window (Activity) | Action on failed entries |
| **Unbind endpoint** | Full window (Sources) | Remove button on bound endpoint |

---

## Pass 4: Cognitive Load

### Tray Popup: Zero Cognitive Load

The popup is read-only. Users glance, understand, and close. No decisions to make.

- Colors tell the story (green/yellow/red)
- Counts give specifics ("2 pending, 0 failed")
- One action available: "Open LocalPush" for details

### Full Window: Managed Complexity

**The 5-step enable flow (kept intentionally):**

| Step | Purpose | Cognitive Ask |
|------|---------|---------------|
| 1. Click "Enable" | Intent signal | Minimal |
| 2. Transparency preview | See YOUR real data | Read and understand what's being sent |
| 3. Pick endpoint | Choose destination | Select from dropdown (not type a URL) |
| 4. Delivery config | Optional headers | Skip if not needed (most users skip) |
| 5. Security coaching | Acknowledge data leaving machine | Read and confirm |

**Why 5 steps is correct:** Radical transparency requires informed consent. Each step serves the principle that "nothing is sent without you seeing it first." Collapsing steps would compromise the core value proposition.

### Language Strategy

| Technical Term | User-Facing Language |
|---------------|---------------------|
| binding | "connected to [endpoint name]" |
| in_flight | "Sending..." |
| dlq (dead letter queue) | "Gave up after 5 retries" |
| 2xx response | "Delivered" |
| pending | "Waiting to send" |
| WAL / ledger | Never shown to user |

### Communication Tone

From the vision document:

| Instead of... | Say... |
|--------------|--------|
| "Data transmission initiated via authenticated webhook" | "Sending your stats now..." |
| "Queue depth: 3 pending items" | "3 updates waiting to send (we'll keep trying)" |
| "Delivery confirmed: HTTP 200" | "Delivered" |
| "Connection error: ECONNREFUSED" | "Can't reach [target name] — we'll retry automatically" |

---

## Pass 5: State Design

### Tray Icon States

| State | Visual | Meaning |
|-------|--------|---------|
| **All good** | Green dot or green-tinted icon | All enabled sources delivering successfully |
| **Pending** | Yellow dot or yellow-tinted icon | Deliveries queued or in progress |
| **Failed** | Red dot or red-tinted icon | One or more deliveries have failed |
| **Idle** | Grey/neutral icon | No sources enabled |
| **Starting** | Pulsing icon | App initializing |

### Tray Popup States

| State | Content |
|-------|---------|
| **Healthy** | Green header: "All sources delivering" + source list with green dots |
| **Degraded** | Yellow/red header: "[N] deliveries need attention" + source list with mixed dots |
| **Empty** | "No sources enabled yet" + "Open LocalPush to get started" |
| **Offline** | "Deliveries queued — will send when online" + queued count |

### Full Window States

| Component | Loading | Empty | Error | Success |
|-----------|---------|-------|-------|---------|
| **Source list** | Skeleton cards | "Enable your first source to start pushing data" | "Failed to load sources" + retry | Source cards with traffic lights |
| **Activity log** | "Loading activity..." | "No deliveries yet. Enable a source to start pushing data." | "Failed to load activity" + retry | Chronological entries with status icons |
| **Settings/Targets** | "Loading targets..." | "Add your first target to connect" | "Failed to load targets" + retry | Connected target list with "Test" buttons |

### Delivery Entry States

| Status | Icon | Color | Label |
|--------|------|-------|-------|
| **Pending** | Circle outline | Yellow | "Waiting to send" |
| **Sending** | Arrow right | Yellow | "Sending..." |
| **Delivered** | Checkmark | Green | "Delivered" |
| **Failed** | X mark | Red | "Failed — retrying (attempt N/5)" |
| **Dead letter** | Skull/stop | Red | "Gave up after 5 retries" |

---

## Pass 6: Flow Integrity

### Happy Path: Install to First Delivery

```
INSTALL
  1. brew install --cask localpush
  2. App appears in menu bar (grey icon — idle)

FIRST INTERACTION
  3. Click tray icon → popup shows "No sources enabled yet"
  4. Click "Open LocalPush" → full window opens

CONNECT TARGET (one-time setup)
  5. Settings tab is shown (or guided to it)
  6. Click "n8n" tab → paste URL + API key → "Connect"
  7. "Connected" confirmation → target appears in connected list

ENABLE FIRST SOURCE
  8. Switch to Sources tab → see available sources
  9. Click "Enable" on Claude Stats

RADICAL TRANSPARENCY FLOW (5 steps)
  10. Transparency preview: see YOUR real data (token counts, sessions)
  11. "Looks good, connect!" → pick target endpoint (dropdown)
  12. Configure delivery (most users skip optional headers)
  13. Security coaching → acknowledge → confirm
  14. Source enabled → green traffic light

FIRST DELIVERY
  15. Push happens within 5 seconds (delivery worker picks up)
  16. Activity log shows "Delivered" entry
  17. Tray icon turns green

ONGOING
  18. Tray icon stays green as long as deliveries succeed
  19. Click tray for quick status check
  20. Open full window only when investigating issues or adding sources
```

### Error Recovery Paths

| Error | User Sees | Recovery |
|-------|-----------|----------|
| Target unreachable | Red traffic light, "Can't reach [target]" in popup | Open Settings → Test connection → fix URL/API key |
| Delivery failed (retrying) | Yellow traffic light, "Retrying..." | Automatic — user can watch in Activity tab |
| Max retries exhausted | Red traffic light, "Gave up" in Activity | Retry button in Activity tab, or fix target and retry |
| No target configured | "Enable" flow stops at endpoint picker: "No targets connected" | Link to Settings to add target |
| Source data unavailable | Preview shows "No data available yet" | User waits for source app to generate data |

### Onboarding Heuristic

**First launch should reach first delivery in under 3 minutes** assuming:
- User has an n8n instance URL and API key ready
- User has Claude Code installed (stats file exists)

---

## Visual Language

### Color System

| Token | Value | Usage |
|-------|-------|-------|
| `--bg-primary` | `#1a1a1a` | Main background |
| `--bg-secondary` | `#2a2a2a` | Cards, elevated surfaces |
| `--text-primary` | `#ffffff` | Primary text |
| `--text-secondary` | `#a0a0a0` | Secondary text, labels |
| `--accent` | `#4f9eff` | Interactive elements, active states |
| `--success` | `#4ade80` | Delivered, healthy, green states |
| `--warning` | `#fbbf24` | Pending, in-progress, yellow states |
| `--error` | `#f87171` | Failed, red states |
| `--border` | `#3a3a3a` | Dividers, card borders |

### Typography

| Element | Font | Size | Weight |
|---------|------|------|--------|
| Body | -apple-system, BlinkMacSystemFont | 14px | 400 |
| Card title | System | 14px | 600 |
| App header | System | 16px | 600 |
| Monospace values | SF Mono, Monaco, Cascadia Code | 13px | 400 |
| Labels | System | 12px | 600, uppercase |
| Badges | System | 11px | 600, uppercase |

### Spacing

| Token | Value | Usage |
|-------|-------|-------|
| `xs` | 4px | Tight gaps (nav buttons) |
| `sm` | 8px | Card internal spacing |
| `md` | 12px | Section margins |
| `lg` | 16px | Page padding, card padding |

### Border Radius

| Element | Radius |
|---------|--------|
| Cards | 8px |
| Buttons | 6px |
| Inputs | 6px |
| Traffic light dots | 50% (circle) |
| Badges | 4px |

### Component Patterns

**Cards:** Dark surface (`--bg-secondary`) with border, 8px radius, 16px padding. Used for source items, target items, settings groups.

**Buttons:** Primary (blue `--accent` fill, white text) and secondary (dark fill, border, white text). 6px radius, 13px font.

**Traffic lights:** 8-10px colored circles. Green/yellow/red/grey. Used inline with source names.

**Status messages:** Colored background (10% opacity of status color) with colored border and text. Fade-in animation.

**Form inputs:** Dark background (`--bg-primary`), border, 6px radius. Blue border on focus.

---

## Design References

### Dato (Tray Popup Reference)

**What to adopt:**
- Menu/dropdown as primary surface (not a popover)
- Compact, status-focused layout
- Native macOS feel with vibrancy/translucency
- Progressive disclosure: simple popup → full window for details
- Smart defaults that work without configuration

**What to adapt:**
- Dato shows calendar data; LocalPush shows delivery health
- Dato's popup is read-only; LocalPush's popup is read-only too (config in full window)
- Dato uses system menu pattern; LocalPush can use Tauri webview popup

### Raycast (Full Window Reference)

**What to adopt:**
- Tab-based navigation for settings organization
- Two-column master-detail for extension/command configuration
- Dark mode with automatic high-contrast optimization
- Text-centric interface with minimal visual chrome
- Progressive disclosure: General → Extensions → Advanced

**What to adapt:**
- Raycast is keyboard-first; LocalPush is mouse-first (menu bar app)
- Raycast settings are dense; LocalPush needs friendlier onboarding
- Raycast has hundreds of extensions; LocalPush has 5-30 sources

---

## Visual Assistant Prompt

Copy the block below into Google Stitch, Polymet, or v0.dev to generate baseline UI designs.

```
Design a macOS menu bar utility app with TWO UI surfaces: a compact tray popup and a full settings window.

Product: LocalPush — watches local Mac data (Claude Code stats, Apple Podcasts, etc.) and pushes it to webhook targets (n8n, ntfy) with guaranteed delivery.

Audience: Developers and tech-savvy Mac users who use automation tools. Not enterprise — personal productivity.

---

SURFACE 1: TRAY POPUP (Dato-style)
Mental Model: A health indicator you glance at. Like checking if Dropbox is syncing.
Size: ~300px wide, variable height.
Trigger: Click menu bar icon.

Content:
- Health banner at top (green/yellow/red background tint + summary text like "All sources delivering" or "2 deliveries need attention")
- Source status list: each source name with a colored traffic light dot (green/yellow/red/grey) and last delivery time
- Quick stats row: "3 delivered today, 0 pending, 0 failed"
- Footer: "Open LocalPush" button (opens full window)

States:
- Healthy: green header, all green dots
- Degraded: yellow/red header, mixed dots
- Empty (first launch): "No sources enabled yet" + "Open LocalPush to get started"
- Offline: "Deliveries queued — will send when online"

Must be obvious:
- Overall health at a glance from colors alone
- Which specific sources are healthy vs having issues
- Clear path to open the full window

Keep simple:
- Read-only. No toggles, forms, or configuration.
- Zero cognitive load. Colors and counts tell the full story.

---

SURFACE 2: FULL WINDOW (Raycast-style)
Mental Model: A settings and management window you open when configuring or investigating.
Size: ~800x600, standard macOS window.
Trigger: "Open LocalPush" from popup, or app icon.

Tabs: Sources | Activity | Settings

SOURCES TAB:
- Card per source (name, description, traffic light dot, last sync time)
- "Enable" / "Disable" button per card
- Enabled sources show bound endpoints and "Push Now" button
- Enable flow: transparency preview (real data) → endpoint picker → delivery config → security coaching → confirm

ACTIVITY TAB:
- Chronological delivery log
- Each row: timestamp, source name, status icon (checkmark/arrow/X), status label
- Click row to expand: delivery ID, full timestamp, retry count, error details
- Empty state: "No deliveries yet. Enable a source to start pushing data."

SETTINGS TAB:
- Connected targets section: list of targets with type badge (n8n/ntfy) and "Test" button
- Add target section: tabs for n8n / ntfy, connection forms (URL + API key)
- General settings: auto-update toggle

States:
- Loading: skeleton/placeholder cards
- Error: inline message with retry
- Empty: onboarding prompt with clear next step
- Success: green traffic lights, delivery confirmations

---

VISUAL LANGUAGE:
- Dark mode only
- Background: #1a1a1a (primary), #2a2a2a (cards/elevated)
- Text: #ffffff (primary), #a0a0a0 (secondary)
- Accent: #4f9eff (blue, interactive elements)
- Success: #4ade80 (green), Warning: #fbbf24 (yellow), Error: #f87171 (red)
- Border: #3a3a3a
- Font: -apple-system (system), SF Mono for data values
- Border radius: 8px cards, 6px buttons/inputs
- Spacing: 16px page padding, 12px section gaps, 8px internal
- macOS native feel — not web-app-looking

TONE:
- Friendly helper, not enterprise dashboard
- "Sending your stats now..." not "Data transmission initiated"
- "3 updates waiting to send" not "Queue depth: 3"

FLOW: Install → grey tray icon → click → "No sources yet, open settings" → add target → enable source → see real data preview → pick endpoint → confirm → green tray icon → data flowing
```

---

## Changelog

| Version | Date | Change |
|---------|------|--------|
| 1.0 | 2026-02-09 | Initial constitution: 6 passes, visual language, Stitch prompt |
