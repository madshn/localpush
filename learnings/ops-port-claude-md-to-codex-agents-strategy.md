# Learning: Port `CLAUDE.md` to Codex `AGENTS.md` with a selective mirror

## Why this exists

This repo had strong project instructions in `CLAUDE.md` (root + subdirectories) but no Codex-native `AGENTS.md` mirrors. A direct copy would have imported Claude-specific persona/process rules and created instruction conflicts.

The successful approach was a **selective mirror**:

- Keep project truths (architecture, invariants, constraints, verification)
- Rewrite tool/workflow guidance to fit Codex
- Drop persona and tool-specific ritual content

## Port strategy (repeatable)

### 1) Inventory instruction files first

Find all `CLAUDE.md` / `AGENT.md` / `AGENTS.md` files and map scope:

- repo root (product context + invariants)
- frontend subdir (`src/`)
- backend subdir (`src-tauri/`)

This prevents creating only a root mirror while missing important local implementation guidance.

### 2) Classify content into 3 buckets

#### A. Portable (copy/adapt)

Keep content that improves engineering correctness regardless of model/tool:

- Product purpose and priorities
- Architecture and data flow
- Reliability invariants (e.g., WAL/ledger guarantees)
- Platform constraints / gotchas
- Extension patterns
- Verification commands
- Frontend/backend implementation conventions

#### B. Rewrite (tool-specific but still useful)

Rewrite content that is useful but tied to a different agent runtime:

- Permissions/process expectations
- "When to escalate" guidance
- Completion criteria phrasing
- Communication formatting rules (if desired)

In this repo, global Codex runtime instructions already cover most of this, so the local mirror kept only repo-specific verification and boundary notes.

#### C. Drop (do not mirror)

Remove content that is specific to Claude identity or ecosystem plugins:

- Persona roleplay ("Product Coordinator", "farm" experts, etc.)
- Claude-only commands/tools/plugins
- Mandatory rituals that conflict with Codex defaults

## What was created here

- `/Users/madsnissen/dev/localpush/AGENTS.md` (repo-level codex guide)
- `/Users/madsnissen/dev/localpush/src/AGENTS.md` (frontend patterns)
- `/Users/madsnissen/dev/localpush/src-tauri/AGENTS.md` (backend patterns)

This matches the existing `CLAUDE.md` hierarchy and keeps guidance local to the code being edited.

## Why hierarchy mirroring matters

Agents often read the nearest instruction file first. If only the root mirror exists, frontend/backend-specific conventions can be missed during localized edits.

Mirroring the hierarchy improves:

- correctness (local patterns are applied)
- speed (less repo spelunking)
- consistency (same conventions used by different agents/tools)

## Drift management (before automation exists)

Until sync/parsing automation is in place:

- Treat `CLAUDE.md` and `AGENTS.md` as sibling outputs derived from shared project knowledge
- When changing important invariants/architecture/verification, update both mirrors in the same PR
- Prefer moving large shared sections to tool-neutral docs later (for example `docs/architecture.md`, `docs/invariants.md`) and link from both files

## Suggested future automation design (high level)

When you automate this later, avoid "copy everything" transforms. Build a parser that:

1. Splits sections by heading
2. Classifies sections using rules/tags:
   - `portable`
   - `rewrite`
   - `drop`
3. Emits target templates (`CLAUDE.md`, `AGENTS.md`) per directory scope
4. Preserves manual notes in protected blocks (optional)

This keeps the system stable even when source docs become more opinionated.

## Practical checklist for repeating this in another repo

1. Find all instruction files and scopes.
2. Read enough to identify invariants and local coding patterns.
3. Create `AGENTS.md` mirrors matching the same directory hierarchy.
4. Port portable content first.
5. Remove persona/plugin-specific content.
6. Keep verification commands and platform gotchas.
7. Add a learning note (or docs note) describing the port rules and drift plan.
