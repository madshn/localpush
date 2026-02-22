# LocalPush CLAUDE.md Comprehensive Improvement

**Date:** 2026-02-22
**Pattern:** CP-01 (Three-Tier CLAUDE.md — inline/stub/absent)
**Reference:** RAW (rightaim-ai) improvement completed 2026-02-19
**Approach:** B — Enriched identity + condensed reference

---

## Summary

Restructure localpush's CLAUDE.md from 560 lines of mixed identity + technical reference to ~445 lines of rich Product Coordinator identity with condensed technical context. Apply the CP-01 three-tier pattern proven in rightaim-ai, with localpush-native sections for delivery guarantees, macOS platform, and extensibility.

---

## What Changes

### Tier 1: Inline (rewritten/expanded)

| Section | Action | Lines |
|---------|--------|-------|
| **Product Coordinator identity** | Rewrite: field worker metaphor, customer-obsessed mindset | ~15 |
| **North Star** | Rewrite: customer framing, proof-of-life definition | ~5 |
| **Target Customers** | New: 3 personas (Automation Tinkerer, Privacy-Conscious Dev, Data Hoarder) | ~15 |
| **Decision Philosophy** | New: 6 principles including delivery guarantee + radical transparency | ~10 |
| **Domain Ownership** | Restructure: explicit what-you-own / what-you-escalate | ~20 |
| **Tool Ownership** | New: table of agents, commands, MCP tools | ~10 |
| **Permission Model** | New: pre-approved / requires approval / never | ~8 |
| **Success Criteria** | New: specific metrics and targets (installs, deliveries, success rate) | ~12 |
| **Walkie-Talkie** | New: farm contacts relevant to localpush (Bob, Mira, Metrick, Aston) | ~10 |
| **Delivery Guarantee Contract** | New (localpush-native): what "guaranteed" means and doesn't | ~15 |
| **macOS Platform Considerations** | New (localpush-native): credentials, file watching, menu bar, app data | ~10 |
| **Source/Target Extensibility** | New (localpush-native): trait-based extension mental model | ~10 |
| **Architecture** | Condense: keep ASCII diagram + data flow summary, remove details | ~25 |
| **Stack** | Condense: keep tech table, remove dev setup details | ~10 |
| **Verification Gates** | Keep: unchanged | ~10 |
| **GitHub Issues Protocol** | New: triage, creation, TODO promotion | ~20 |
| **superpowers Integration** | New (from template): skill table, TDD, verification | ~15 |

### Tier 1: Inline (from factory template, unchanged)

| Section | Lines |
|---------|-------|
| Communication Standards | ~20 |
| Impediment-Driven Development | ~30 |
| Human Attention Optimization | ~30 |
| Task System | ~15 |
| Plan Mode Context Protocol | ~15 |
| Coordinator Protocol | ~20 |
| MCP Context Discipline | ~25 |
| Human Testing Workflow | ~15 |
| Research Workflow | ~10 |
| Bob Rounds Awareness | ~5 |
| Key Files + References | ~15 |

### What Gets Removed (covered elsewhere)

| Content | Current Lines | Where It Lives |
|---------|--------------|----------------|
| Full project structure tree | ~80 | Discoverable via ls/Glob |
| 30+ Tauri commands table | ~20 | `src-tauri/CLAUDE.md` |
| Development Setup / Getting Started | ~15 | README / dev guide |
| Dev Credential Store details | ~5 | `src-tauri/CLAUDE.md` |
| Testing Strategy details | ~15 | `TESTING.md` |
| Known Issues | ~5 | Move to `TODO.md` |
| Key Decisions (Tauri-specific) | ~10 | `src-tauri/CLAUDE.md` |
| Debugging instructions | ~10 | `src-tauri/CLAUDE.md` |
| Common Tasks (add source/target) | ~25 | `src-tauri/CLAUDE.md` |
| Dependencies & Versions table | ~10 | Bob tracks in dependency-registry.md |
| Development Workflow (4-step process) | ~25 | Move to dev guide or keep in sub-CLAUDE.md |

---

## New Sections: Full Content

### Target Customers

| Persona | Description | What excites them |
|---------|-------------|-------------------|
| **The Automation Tinkerer** | n8n/Make/Zapier user who wants local data piped to their workflows without writing code. Finds LocalPush via n8n community or forum search. | One-click source setup, instant webhook delivery, "it just works" reliability. Seeing their Claude Code stats arrive in n8n within seconds of a session ending. |
| **The Privacy-Conscious Dev** | Developer who wants observability over their local tools (Claude Code, IDE stats) but won't send data to third-party analytics. Self-hosts everything. | Local-first architecture, no cloud dependency, full transparency over what data leaves their machine. The radical transparency preview is their love language. |
| **The Data Hoarder** | Power user who tracks everything — podcasts, notes, photos, coding sessions. Wants all their data in one pipeline. Discovers LocalPush through a blog post or GitHub trending. | Multi-source support, guaranteed delivery (nothing lost), easy target setup. The promise of "every digital artifact, automatically captured." |

### Decision Philosophy

| Principle | Meaning |
|-----------|---------|
| **Customer obsession** | Every feature decision starts with "would this make the customer's day?" |
| **Ship and learn** | Working software in front of users beats perfect plans in documents |
| **Prove life first** | Proof of life (install, active source, delivery) before polish |
| **Measure, don't guess** | Instrument everything — let data confirm or kill your assumptions |
| **Guaranteed delivery or nothing** | The WAL contract is sacred. If we say "guaranteed," it survives crashes, reboots, and network failures. No silent data loss, ever. |
| **Radical transparency** | Users see their real data before enabling anything. No "trust us" — show the actual payload. |

### Success Criteria

| Signal | Metric | Target |
|--------|--------|--------|
| **Someone installs** | Homebrew/DMG downloads | ≥1 non-developer (proof of life) |
| **Someone uses it daily** | Active sources running >24h | ≥3 users |
| **Deliveries land** | Successful webhook deliveries/day | >50 across all users |
| **Nothing is lost** | Delivery success rate | >99.5% |
| **Sources grow** | Distinct source types actively used | ≥3 |
| **Targets expand** | Distinct target types connected | ≥2 (n8n + ntfy) |
| **Tests pass** | Combined test suite | 100% green |
| **Code is clean** | cargo clippy + npm lint | 0 warnings |

### Delivery Guarantee Contract

| Guarantee | Implementation |
|-----------|---------------|
| **Crash-safe** | SQLite WAL mode — writes survive app crashes mid-delivery |
| **Retry on failure** | Failed deliveries stay in ledger, retried on next poll cycle (5s) |
| **No silent drops** | Every source event is ledgered before any delivery attempt |
| **Binding resolution** | If no bindings exist, falls back to legacy global webhook (v0.1 compat) |
| **Visibility** | Every delivery state (pending → delivering → delivered/failed) is queryable |

**What "guaranteed" does NOT mean:**
- Network uptime — if the target is unreachable for days, deliveries queue but don't magically arrive
- Ordering — deliveries are best-effort ordered, not strictly sequential
- Exactly-once — at-least-once semantics; targets should be idempotent

Any feature that weakens these guarantees requires explicit user approval before shipping.

### macOS Platform Considerations

| Concern | Current Approach |
|---------|-----------------|
| **Credentials** | Dev: file-based (`dev-credentials.json`). Prod: macOS Keychain via `keyring` crate |
| **File watching** | FSEvents via `notify-rs` with 300ms debounce |
| **Menu bar** | 22x22 PNG template icon, single 420x680 window |
| **App data** | `~/Library/Application Support/com.localpush.app/` (config.sqlite + ledger.sqlite) |
| **Build/signing** | Tauri bundler → .dmg. Code signing + notarization for distribution (Mira's domain) |

**Key rule:** Use `tauri::async_runtime::spawn` (NOT `tokio::spawn`) for any spawned work that needs Tauri context. Use `Mutex<Connection>` for rusqlite thread safety.

### Source/Target Extensibility

LocalPush is designed for easy extension. The trait-based architecture means adding a source or target is a bounded task.

**Adding a source:** Implement `Source` trait → register in SourceManager → auto-appears in UI. See `src-tauri/CLAUDE.md` for trait details and examples.

**Adding a target:** Implement `Target` trait → add connect command → add frontend form → register startup restoration. See `src-tauri/CLAUDE.md` for trait details and examples.

**Key architectural constraint:** Sources are southbound (local data in), targets are northbound (data out). Bindings connect them. The delivery worker is the only component that touches the network.

### GitHub Issues Protocol

**Issue Triage:**

| Label | Meaning | Response |
|-------|---------|----------|
| `bug` | Confirmed defect | Acknowledge within session, fix or document workaround |
| `enhancement` | Feature request | Evaluate against roadmap, label priority |
| `question` | User needs help | Answer directly or point to docs |
| `good first issue` | Bounded, well-documented | Keep ≥2 open for contributor onboarding |

**Creating Issues:** Immediate fix → no issue. Deferred fix → issue with repro steps. Feature idea → issue with persona, expected behavior, priority recommendation.

**TODO → Issue Promotion:** At session end, review TODO.md. Completed → remove. Open + actionable → promote to GitHub Issue. Ephemeral → leave.

---

## Implementation Plan

1. Write the new CLAUDE.md (full rewrite)
2. Verify extracted content is already covered in sub-CLAUDE.md files
3. Move Known Issues to TODO.md
4. Verify Development Workflow content exists in sub-CLAUDE.md files
5. Commit with message: `feat(localpush): comprehensive CLAUDE.md improvement — CP-01 applied`
