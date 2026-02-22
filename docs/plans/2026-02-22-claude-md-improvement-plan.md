# LocalPush CLAUDE.md Comprehensive Improvement — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Restructure localpush's CLAUDE.md from 560 lines of mixed identity/reference to ~445 lines of rich PC identity with condensed technical context, applying the CP-01 three-tier pattern.

**Architecture:** Full rewrite of CLAUDE.md using the factory template as skeleton, filling in localpush-specific content from the design doc. Extract heavy reference material that's already covered in sub-CLAUDE.md files. Create TODO.md for known issues.

**Tech Stack:** Markdown only — no code changes.

**Design doc:** `docs/plans/2026-02-22-claude-md-comprehensive-improvement-design.md`

---

### Task 1: Create TODO.md with Known Issues

**Files:**
- Create: `TODO.md`

**Step 1: Write TODO.md**

Move the 3 known issues from current CLAUDE.md into a new TODO.md:

```markdown
# TODO

## Known Issues

- [ ] **UX: Enable checkbox confusing** — "I did not recognize it as a checkbox". Defer to Google Stitch for redesign.
- [ ] **Old production LocalPush.app conflicts** — Kill before dev testing: `pkill -f LocalPush || true`
- [ ] **Port 1420 may be held** — `lsof -ti:1420 | xargs kill -9`
```

**Step 2: Verify file created**

Run: `wc -l TODO.md`
Expected: ~7 lines

**Step 3: Commit**

```bash
git add TODO.md
git commit -m "chore(localpush): create TODO.md with known issues from CLAUDE.md"
```

---

### Task 2: Write the new CLAUDE.md

**Files:**
- Modify: `CLAUDE.md` (full rewrite — 560 lines → ~445 lines)

**Step 1: Read design doc for reference**

Read: `docs/plans/2026-02-22-claude-md-comprehensive-improvement-design.md`

This contains all the new section content (target customers, decision philosophy, success criteria, delivery contract, macOS considerations, extensibility, GitHub issues protocol).

**Step 2: Write the complete new CLAUDE.md**

Structure (in order):

1. **Header** — Project name + one-line description (from current, unchanged)
2. **Product Coordinator** section:
   - Identity (field worker metaphor — from factory template, filled for localpush)
   - North Star (from design doc)
   - Target Customers table (from design doc — 3 personas)
   - Decision Philosophy table (from design doc — 6 principles)
   - Domain Ownership (from design doc — what you own / escalate)
   - Tool Ownership table (from design doc)
   - Permission Model (from design doc)
   - Success Criteria table (from design doc)
   - Walkie-Talkie (from design doc — Bob, Mira, Metrick, Aston)
3. **Delivery Guarantee Contract** (from design doc)
4. **macOS Platform Considerations** (from design doc)
5. **Source/Target Extensibility** (from design doc)
6. **Architecture** — Keep ASCII diagram + condensed data flow from current CLAUDE.md. Remove file tree and details. Add pointers: "For implementation details, read `src-tauri/CLAUDE.md`. For frontend patterns, read `src/CLAUDE.md`."
7. **Stack** — Keep condensed tech table from current (6 rows). Remove dev setup, debugging, deps.
8. **Verification Gates** — Keep unchanged from current.
9. **superpowers Integration** — From factory template (`templates/project/CLAUDE.md` lines 74-98).
10. **Communication Standards** — From factory template (lines 140-166). Include question formatting, decision requests, progress updates.
11. **Impediment-Driven Development** — From factory template (lines 169-201). Include four constraints, planning protocol, progress tracking.
12. **Human Attention Optimization** — From factory template (lines 204-246). Include session handoff, decision batching, async-first.
13. **Task System** — From factory template (lines 289-327).
14. **GitHub Issues Protocol** — From design doc.
15. **Coordinator Protocol** — Keep from current. Add MCP Context Discipline from factory template (lines 388-425).
16. **Plan Mode Context Protocol** — Keep from current.
17. **Human Testing Workflow** — From factory template (lines 491-531). Adapt port numbers for Tauri dev.
18. **Research Workflow** — From factory template (lines 547-572).
19. **Bob Rounds Awareness** — Keep from current, add standard acknowledgment from template.
20. **Key Files** — Update table to reflect new structure.
21. **References** — Keep from current.

**Step 3: Verify line count**

Run: `wc -l CLAUDE.md`
Expected: ~400-500 lines

**Step 4: Verify no broken references**

Check that all file paths mentioned in CLAUDE.md actually exist:
- `src-tauri/CLAUDE.md` ✓ (verified)
- `src/CLAUDE.md` ✓ (verified)
- `ROADMAP.md` ✓ (verified)
- `TESTING.md` ✓ (verified)
- `TODO.md` ✓ (created in Task 1)
- `.claude/agents/localpush-agent.md` ✓ (verified)
- `.claude/commands/bob.md` ✓ (verified)

**Step 5: Commit**

```bash
git add CLAUDE.md
git commit -m "feat(localpush): comprehensive CLAUDE.md improvement — CP-01 applied

Restructure from 560 lines of mixed identity/reference to rich Product
Coordinator identity with condensed technical context.

Changes:
- Rewrite PC section with field worker identity, target customers,
  decision philosophy, success criteria, walkie-talkie
- Add localpush-native sections: delivery guarantee contract, macOS
  platform considerations, source/target extensibility
- Add GitHub Issues protocol, superpowers integration, MCP context
  discipline
- Add operational protocols from factory template (IDD, HAO,
  communication standards, task system)
- Extract heavy reference material already covered in sub-CLAUDE.md
  files (Tauri commands, project tree, dev setup, debugging, common
  tasks, dependency versions)
- Keep condensed architecture diagram and stack table inline"
```

---

### Task 3: Verify sub-CLAUDE.md coverage

**Files:**
- Read: `src-tauri/CLAUDE.md`
- Read: `src/CLAUDE.md`

**Step 1: Check that extracted content has a home**

Verify each removed section is covered:

| Removed Content | Expected Location | What to Check |
|----------------|-------------------|---------------|
| Tauri commands list | `src-tauri/CLAUDE.md` lines 173-197 | Command pattern documented |
| Project structure tree | `src-tauri/CLAUDE.md` lines 54-78, `src/CLAUDE.md` lines 26-51 | Layout documented |
| Development Setup | `src/CLAUDE.md` lines 316-322 | Dev mode documented |
| Debugging instructions | `src-tauri/CLAUDE.md` lines 490-513 | Logging + SQL inspection documented |
| Common Tasks (add source/target) | `src-tauri/CLAUDE.md` lines 460-487 | Add source/target/command documented |
| Testing Strategy | `TESTING.md` (441 lines) | Full strategy documented |

**Step 2: Report any gaps**

If any critical content is missing from sub-files, add it. Otherwise, no changes needed.

---

### Task 4: Final verification

**Step 1: Read the new CLAUDE.md end-to-end**

Read: `CLAUDE.md`

Verify:
- PC identity section conveys role clearly (field worker, not generic Claude Code)
- North Star is actionable
- Target customers are specific enough to guide decisions
- Success criteria are measurable
- Architecture diagram is present and readable
- Verification gates are clear
- All file references resolve

**Step 2: Verify git status is clean**

Run: `git status`
Expected: Clean working tree, all committed.
