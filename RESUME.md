# LocalPush Resume Prompt

**Last Updated:** 2026-02-04
**Project Path:** `~/dev/localpush`
**Status:** Scaffolding ~60% complete

---

## Resume Prompt

Copy and paste this to continue work:

```
I'm resuming work on LocalPush, a macOS menu bar app for guaranteed file→webhook delivery.

**Project:** ~/dev/localpush
**Plan:** Read PLAN.md for full context, architecture, and remaining tasks.

**Current State:**
- Trait-based architecture complete (CredentialStore, FileWatcher, WebhookClient, DeliveryLedger)
- SQLite ledger with WAL mode implemented and tested
- Production Keychain and FSEvents implementations started
- Frontend shell scaffolded (React + Tauri commands)

**Remaining Work (Priority Order):**
1. Production webhook client (reqwest) - src-tauri/src/production/webhook_client.rs
2. Mock implementations for testing - src-tauri/src/mocks/
3. Sources module (Claude Code stats parser) - src-tauri/src/sources/
4. CLAUDE.md files (root, src, src-tauri)
5. Test infrastructure (vitest.config.ts, mockIPC)
6. CI/CD pipeline (.github/workflows/)
7. Git init and verification builds

**Key Principle:** Use sub-agents extensively. This project is designed for 98% AI construction with parallel work streams.

Please read PLAN.md and RESUME.md, then continue with the next priority task using appropriate sub-agents.
```

---

## Sub-Agent Strategy

LocalPush is designed for maximum agentic velocity through parallel sub-agents. All external dependencies are abstracted behind traits, enabling isolated testing.

### Recommended Agent Types

| Agent | Model | Purpose | Tools |
|-------|-------|---------|-------|
| **rust-backend** | sonnet | Implement Rust backend code | Edit, Write, Bash (cargo) |
| **react-frontend** | sonnet | Implement React frontend code | Edit, Write |
| **rust-tester** | haiku | Write and run Rust tests | Edit, Bash (cargo test) |
| **frontend-tester** | haiku | Write and run Vitest tests | Edit, Bash (npm test) |
| **ci-builder** | haiku | Create/update CI workflows | Write, Bash (gh) |
| **doc-writer** | haiku | CLAUDE.md and documentation | Write |

### Model Selection Rationale

- **sonnet** for implementation: Needs code comprehension, creative problem-solving
- **haiku** for testing/docs: I/O-bound, template-driven, can run in parallel

### Parallel Work Streams

```
Stream A: Backend                Stream B: Frontend              Stream C: Infra
─────────────────               ─────────────────               ─────────────────
rust-backend: webhook_client    react-frontend: TransparencyPreview  ci-builder: verify.yml
rust-tester: webhook tests      frontend-tester: component tests     doc-writer: CLAUDE.md
rust-backend: sources/claude    react-frontend: SettingsPanel        ci-builder: release.yml
```

### Agent Dispatch Patterns

#### Pattern 1: Sequential Implementation + Test

```typescript
// First: Implement
Task({
  subagent_type: "general-purpose",
  model: "sonnet",
  prompt: `
    Implement ReqwestWebhookClient in src-tauri/src/production/webhook_client.rs

    Requirements:
    1. Implement WebhookClient trait from traits/webhook_client.rs
    2. Use reqwest with async/await
    3. Support all WebhookAuth variants (None, Header, Bearer, Basic)
    4. 25 second timeout
    5. Return WebhookResponse with status, body, duration_ms

    Read the trait file first, then implement. Do not run tests yet.
  `
})

// Then: Test
Task({
  subagent_type: "general-purpose",
  model: "haiku",
  prompt: `
    Write tests for ReqwestWebhookClient in src-tauri/src/production/webhook_client.rs

    Test scenarios:
    1. Successful POST with JSON payload
    2. Each auth type (None, Header, Bearer, Basic)
    3. Timeout handling
    4. Network error handling
    5. Non-2xx response handling

    Run: cargo test -p localpush webhook
    Return: Test results summary
  `
})
```

#### Pattern 2: Parallel Independent Tasks

```typescript
// Launch all three in parallel (single message, multiple Task calls)
Task({
  subagent_type: "general-purpose",
  model: "haiku",
  prompt: "Create .github/workflows/verify.yml for LocalPush..."
})

Task({
  subagent_type: "general-purpose",
  model: "haiku",
  prompt: "Create root CLAUDE.md for LocalPush..."
})

Task({
  subagent_type: "general-purpose",
  model: "haiku",
  prompt: "Create src/CLAUDE.md for LocalPush frontend..."
})
```

#### Pattern 3: Research Then Implement

```typescript
// First: Research existing patterns
Task({
  subagent_type: "Explore",
  prompt: `
    Explore the existing trait implementations in src-tauri/src/production/
    Understand the patterns used for:
    - Error handling
    - Logging (tracing)
    - Constructor patterns
    Return: Summary of patterns to follow
  `
})

// Then: Implement following patterns
Task({
  subagent_type: "general-purpose",
  model: "sonnet",
  prompt: `
    Implement sources/claude_stats.rs following the patterns from production/

    [Include pattern summary from research agent]

    Parse ~/.claude/stats-cache.json and emit delivery events.
  `
})
```

### Verification Gates

Every implementation must pass verification. Run after completing a work stream:

```bash
# Rust verification
cargo test                    # Unit + integration tests
cargo clippy -- -D warnings   # Linting

# Frontend verification
npm run test                  # Vitest tests
npm run lint                  # ESLint
npm run typecheck             # TypeScript strict

# Full build
cargo build --release         # Release build verification
```

### Agent Prompt Best Practices

1. **Focused scope**: One file or one feature per agent
2. **Clear requirements**: Numbered list of what to implement
3. **Context provided**: Reference existing files to read first
4. **Explicit output**: What should the agent return?
5. **No guessing**: If unclear, agent should ask or return partial

### When NOT to Use Sub-Agents

- Quick single-file edits (do directly)
- Exploratory debugging (needs full context)
- Decisions requiring human input (ask first)
- Sequential dependencies (wait for previous result)

---

## File Structure Reference

```
~/dev/localpush/
├── PLAN.md                       # Full implementation plan
├── RESUME.md                     # This file
├── package.json                  ✓
├── tsconfig.json                 ✓
├── vite.config.ts                ✓
├── index.html                    ✓
├── src/
│   ├── CLAUDE.md                 [ ] TODO
│   ├── main.tsx                  ✓
│   ├── App.tsx                   ✓
│   ├── components/
│   │   ├── StatusIndicator.tsx   ✓
│   │   ├── SourceList.tsx        ✓
│   │   ├── DeliveryQueue.tsx     ✓
│   │   └── TransparencyPreview.tsx  [ ] TODO
│   └── api/hooks/                ✓
├── src-tauri/
│   ├── CLAUDE.md                 [ ] TODO
│   ├── Cargo.toml                ✓
│   ├── tauri.conf.json           ✓
│   └── src/
│       ├── main.rs               ✓
│       ├── lib.rs                ✓
│       ├── state.rs              ✓
│       ├── ledger.rs             ✓ (with tests)
│       ├── commands/mod.rs       ✓
│       ├── traits/               ✓ (all 4 traits)
│       ├── production/
│       │   ├── credential_store.rs ✓
│       │   ├── file_watcher.rs   ✓
│       │   └── webhook_client.rs [ ] TODO
│       ├── mocks/                [ ] TODO
│       └── sources/              [ ] TODO
└── .github/workflows/            [ ] TODO
```

---

## Key Architecture Decisions

1. **Trait-based DI**: All external dependencies behind protocols → 100% testable
2. **WAL delivery**: SQLite WAL mode ensures crash-safe delivery
3. **5-state machine**: PENDING → IN_FLIGHT → DELIVERED/FAILED/DLQ
4. **Exponential backoff**: 1s, 2s, 4s, 8s... up to 1 hour max
5. **Layered testing**: Unit → Integration → Behavioral → Smoke

---

## Quick Commands

```bash
# Development
cd ~/dev/localpush
npm run tauri dev              # Run dev server

# Rust only
cd src-tauri
cargo test                     # Run tests
cargo clippy -- -D warnings    # Lint

# Frontend only
npm run test                   # Vitest
npm run lint                   # ESLint
npm run typecheck              # TypeScript

# Full verification
npm run check                  # Runs all checks
```

---

## Next Session Checklist

1. [ ] Read PLAN.md for full context
2. [ ] Check git status (not yet initialized)
3. [ ] Pick next priority from remaining work
4. [ ] Dispatch appropriate sub-agents
5. [ ] Run verification gates after each completion
6. [ ] Update PLAN.md status as work completes
