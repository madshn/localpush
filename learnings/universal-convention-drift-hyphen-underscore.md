# Convention Drift Between Backend IDs and Frontend Lookups

**Date:** 2026-02-14
**Category:** universal
**Tags:** naming-conventions, kebab-case, snake_case, id-mapping, silent-failures
**Source:** Codex audit confirmed source ID and target ID convention drift in LocalPush
**Confidence:** high

---

## Problem

Backend defines canonical IDs in one convention (kebab-case: `claude-stats`, `n8n-abc123`), but frontend hardcodes lookup maps in another convention (snake_case: `claude_code_stats`, split on `_`). Lookups silently return `undefined` — no runtime error, just missing icons, missing labels, broken parsing.

## Pattern

1. **Define canonical ID format once** — pick kebab-case or snake_case and enforce it
2. **Frontend lookup maps must use the same convention as backend source IDs** — no translation layer that can drift
3. **Use TypeScript enums or const objects** derived from a single source of truth when possible
4. **Test ID mapping** — contract tests that fail when a new source/target is added without updating frontend maps

## Anti-pattern

- Hardcoding ID lookup maps in frontend with different conventions than backend
- Using `string.split("_")` to parse IDs that are actually hyphenated
- Assuming IDs will stay consistent without explicit enforcement
- Silent fallback to default (empty icon, "unknown" label) instead of logging a warning

## Related

- Future: typed IPC boundary / shared contract package would eliminate this class of bug entirely
