# LocalPush Implementation Plan

**Created:** 2026-02-04
**Status:** In Progress (scaffolding ~95% complete)

---

## Project Overview

LocalPush is a macOS menu bar app that watches local files and delivers them to webhooks with **guaranteed delivery** (WAL pattern) and **radical transparency** (users see their real data before enabling).

**Origin:** Metrick lost 7 days of Claude Code token data due to cron timing issues. LocalPush solves this with event-driven, crash-safe delivery.

---

## Current State

### Completed
- [x] Project directory created at `~/dev/localpush`
- [x] Frontend scaffolding (React + TypeScript + Vite)
- [x] Tauri configuration (tauri.conf.json, Cargo.toml)
- [x] Trait definitions (CredentialStore, FileWatcher, WebhookClient, DeliveryLedger)
- [x] SQLite ledger with WAL mode and tests
- [x] Tauri commands (get_delivery_status, get_sources, etc.)
- [x] Production implementations started (Keychain, FSEvents)

### Remaining
- [ ] npm install and cargo build verification
- [ ] Install Rust toolchain (cargo not found on this machine)

### Recently Completed (2026-02-04 - Session 2 continued)
- [x] Git initialization - Initial commit with 53 files

### Recently Completed (2026-02-04 - Session 2)
- [x] TransparencyPreview component - `src/components/TransparencyPreview.tsx`
- [x] Vitest test infrastructure - `vitest.config.ts`, `src/test/`
- [x] Sample tests passing (10 tests across 2 files)

### Recently Completed (2026-02-04)
- [x] Production webhook client (reqwest) - `production/webhook_client.rs`
- [x] Mock implementations for testing - `mocks/mod.rs`
- [x] Sources module (Claude Code stats parser) - `sources/mod.rs`, `sources/claude_stats.rs`
- [x] CLAUDE.md files (root, src, src-tauri)
- [x] CI/CD pipeline (GitHub Actions) - `.github/workflows/verify.yml`, `release.yml`

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│  LocalPush                                                       │
├─────────────────────────────────────────────────────────────────┤
│  ┌──────────────┐     ┌──────────────┐     ┌──────────────────┐ │
│  │  Menu Bar UI │ ◀──▶│  Tauri Core  │ ◀──▶│ Delivery Ledger  │ │
│  │   (React)    │     │    (Rust)    │     │ (SQLite + WAL)   │ │
│  └──────────────┘     └──────┬───────┘     └──────────────────┘ │
│                              │                                   │
│  ┌───────────────────────────┼───────────────────────┐          │
│  │                           │                       │          │
│  ▼                           ▼                       ▼          │
│  traits/               production/              mocks/          │
│  ├─ CredentialStore    ├─ Keychain             ├─ InMemory     │
│  ├─ FileWatcher        ├─ FSEvents             ├─ Manual       │
│  ├─ WebhookClient      ├─ Reqwest              ├─ Recorded     │
│  └─ DeliveryLedger     └─ SQLite               └─ InMemory     │
└─────────────────────────────────────────────────────────────────┘
```

---

## Testing Strategy (Layered)

| Layer | Type | Framework | AI Autonomy |
|-------|------|-----------|-------------|
| 1 | Rust unit tests | cargo test | 100% |
| 2 | Rust integration | mock runtime | 100% |
| 3 | Frontend tests | Vitest + mockIPC | 100% |
| 4 | Behavioral E2E | Playwright headless | 95% |
| 5 | Smoke tests | Human verification | 0% |

**Key insight:** All system dependencies abstracted behind traits → fully testable by AI.

---

## Verification Gates (Mandatory)

Every change must pass:

```bash
cargo test                    # Rust unit + integration tests
cargo clippy -- -D warnings   # Rust linting
npm run test                  # Frontend tests
npm run lint                  # ESLint
npm run typecheck             # TypeScript strict
cargo build --release         # Build verification
```

---

## Sub-Agent Strategy

This project benefits from specialized sub-agents for different work streams:

### Recommended Agent Types

| Agent | Purpose | Tools | Model |
|-------|---------|-------|-------|
| **rust-coder** | Implement Rust backend code | Edit, Write, Bash (cargo) | sonnet |
| **react-coder** | Implement React frontend code | Edit, Write | sonnet |
| **rust-tester** | Write and run Rust tests | Edit, Bash (cargo test) | haiku |
| **frontend-tester** | Write and run Vitest tests | Edit, Bash (npm test) | haiku |
| **ci-builder** | Create/update CI workflows | Write, Bash (gh) | haiku |
| **doc-writer** | CLAUDE.md and documentation | Write | haiku |

### Parallel Work Streams

```
Stream A: Backend                Stream B: Frontend              Stream C: Infra
─────────────────               ─────────────────               ─────────────────
rust-coder: webhook_client      react-coder: TransparencyPreview  ci-builder: verify.yml
rust-tester: webhook tests      frontend-tester: component tests  doc-writer: CLAUDE.md
rust-coder: sources/claude      react-coder: SettingsPanel        ci-builder: release.yml
```

### Agent Dispatch Pattern

```typescript
// Example: Implementing webhook client
Task({
  subagent_type: "general-purpose",  // or custom "rust-coder"
  prompt: `
    Implement the ReqwestWebhookClient in src-tauri/src/production/webhook_client.rs

    Requirements:
    1. Implement WebhookClient trait from traits/webhook_client.rs
    2. Use reqwest with async/await
    3. Support all WebhookAuth variants (None, Header, Bearer, Basic)
    4. Include timeout of 25 seconds
    5. Return WebhookResponse with status, body, duration_ms

    After implementation:
    - Run: cargo test -p localpush webhook
    - Ensure all tests pass

    Return: Summary of implementation and test results
  `
})
```

---

## File Structure

```
~/dev/localpush/
├── PLAN.md                       # This file
├── RESUME.md                     # Resume prompt for new sessions
├── CLAUDE.md                     ✓ Created (Root AI instructions)
├── package.json                  ✓ Created
├── tsconfig.json                 ✓ Created
├── vite.config.ts                ✓ Created
├── index.html                    ✓ Created
├── src/
│   ├── CLAUDE.md                 ✓ Created (Frontend AI instructions)
│   ├── main.tsx                  ✓ Created
│   ├── App.tsx                   ✓ Created
│   ├── styles.css                ✓ Created
│   ├── components/
│   │   ├── StatusIndicator.tsx   ✓ Created
│   │   ├── SourceList.tsx        ✓ Created
│   │   ├── DeliveryQueue.tsx     ✓ Created
│   │   └── TransparencyPreview.tsx  ✓ Created
│   └── api/hooks/
│       ├── useDeliveryStatus.ts  ✓ Created
│       ├── useSources.ts         ✓ Created
│       └── useDeliveryQueue.ts   ✓ Created
├── src-tauri/
│   ├── CLAUDE.md                 ✓ Created (Backend AI instructions)
│   ├── Cargo.toml                ✓ Created
│   ├── tauri.conf.json           ✓ Created
│   ├── build.rs                  ✓ Created
│   └── src/
│       ├── main.rs               ✓ Created
│       ├── lib.rs                ✓ Created
│       ├── state.rs              ✓ Created
│       ├── ledger.rs             ✓ Created (with tests)
│       ├── commands/mod.rs       ✓ Created
│       ├── traits/
│       │   ├── mod.rs            ✓ Created
│       │   ├── credential_store.rs ✓ Created
│       │   ├── file_watcher.rs   ✓ Created
│       │   ├── webhook_client.rs ✓ Created
│       │   └── delivery_ledger.rs ✓ Created
│       ├── production/
│       │   ├── mod.rs            ✓ Created
│       │   ├── credential_store.rs ✓ Created
│       │   ├── file_watcher.rs   ✓ Created
│       │   └── webhook_client.rs ✓ Created
│       ├── mocks/
│       │   └── mod.rs            ✓ Created
│       └── sources/
│           ├── mod.rs            ✓ Created
│           └── claude_stats.rs   ✓ Created
├── specs/                        [ ] TODO
├── tests/                        [ ] TODO
└── .github/workflows/
    ├── verify.yml                ✓ Created
    └── release.yml               ✓ Created
```

---

## MVP Feature Scope

| Feature | Status | Priority |
|---------|--------|----------|
| SQLite ledger with WAL | ✓ Done | P0 |
| Trait-based architecture | ✓ Done | P0 |
| Menu bar UI shell | ✓ Done | P0 |
| Claude Code stats parser | TODO | P0 |
| Webhook delivery (reqwest) | TODO | P0 |
| Transparency preview | TODO | P0 |
| n8n Header Auth | TODO | P0 |
| Homebrew Cask | TODO | P1 |
| Auto-update | TODO | P1 |
| macOS notarization | TODO | P1 |

---

## Research Completed

8 parallel research agents completed comprehensive analysis:

1. **Tauri templates** → Use ahkohd/tauri-macos-menubar-app-example patterns
2. **File watcher** → notify-rs with debouncer-full, 300ms debounce
3. **WAL delivery** → SQLite WAL mode, 5-state machine, exponential backoff
4. **Competitive** → Clear market gap for local→webhook with guarantees
5. **Credentials** → keyring crate v3 for macOS Keychain
6. **Homebrew** → Custom tap, architecture-specific DMGs
7. **n8n webhook** → Header Auth, standard payload schema
8. **Auto-update** → GitHub Releases + tauri-plugin-updater

Full research available in Bob's context.

---

## Next Steps (Priority Order)

1. **Complete production implementations**
   - webhook_client.rs (reqwest)
   - mocks/mod.rs (test doubles)

2. **Create sources module**
   - sources/mod.rs
   - sources/claude_stats.rs (parse ~/.claude/stats-cache.json)

3. **Write CLAUDE.md files**
   - Root CLAUDE.md (entry point)
   - src/CLAUDE.md (frontend patterns)
   - src-tauri/CLAUDE.md (backend patterns)

4. **Set up test infrastructure**
   - vitest.config.ts
   - Frontend test setup with mockIPC
   - Rust test utilities

5. **Create CI/CD pipeline**
   - .github/workflows/verify.yml
   - .github/workflows/release.yml

6. **Initialize git**
   - git init
   - .gitignore
   - Initial commit

7. **Verify builds**
   - npm install
   - cargo build
   - npm run tauri dev

---

## Success Criteria

### MVP Complete When:
- [ ] Claude Code stats syncing to n8n webhook
- [ ] Data survives intentional crash test
- [ ] User can preview their real data before enabling
- [ ] All verification gates pass
- [ ] Installable via Homebrew Cask

### Quality Thresholds:
- Type coverage: 100% (TypeScript strict + Rust)
- Test coverage: >80%
- Lint errors: 0
- Build: Passes on CI
