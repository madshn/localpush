# AI Code Review Output Requires Framework-Aware Verification

**Date:** 2026-02-14
**Category:** universal
**Tags:** ai-review, codex, verification, code-audit, false-positives
**Source:** Codex 5.3 architecture audit of LocalPush, verified claim-by-claim
**Confidence:** high

---

## Problem

AI code reviewers (Codex, etc.) produce structured audits with specific file/line references that look authoritative. Without verification, teams may act on invalid findings or miss the nuance in partially-valid ones. In our case, 35% of findings were invalid.

## Pattern

1. **Verify every claim against actual code** before prioritizing action
2. **Check framework behavior** — AI reviewers analyze files in isolation; they miss framework magic (e.g., Tauri auto-converts snake_case → camelCase in serde serialization)
3. **Distinguish "pattern exists" from "pattern is a bug"** — intentional design choices (like skipping filtering for targeted deliveries) pattern-match to anti-patterns but aren't bugs
4. **Treat line numbers as approximate** — off by 10-50 lines is common; use them as search hints, not precise citations
5. **Cross-reference with a domain-aware reviewer** — have someone who knows the framework and design intent evaluate alongside the raw findings

## Anti-pattern

- Bulk-approving an AI audit because it's well-structured and cites line numbers
- Acting on "security findings" without considering the deployment context (desktop app OAuth ≠ web app OAuth)
- Treating all categories as equal priority — a silent data drop is ship-blocking; a 1400-line file is a backlog item

## Related

- `universal-subagent-cross-file-consistency.md` — similar theme of needing holistic context
