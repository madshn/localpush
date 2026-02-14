# Using Codex for Code Review in VS Code

**Date:** 2026-02-14
**Category:** universal
**Tags:** codex, code-review, vscode, ai-audit, workflow
**Source:** Codex 5.3 architecture audit of LocalPush — 23 findings, 65% valid
**Confidence:** high

---

## Problem

Codex (OpenAI's agent in VS Code) can produce structured architecture audits, but its outputs need a specific workflow to extract value without acting on false positives.

## Pattern

### How to Run

1. Open project in VS Code with Codex extension
2. Prompt: ask for an architecture audit, security review, or correctness analysis
3. Codex runs tests (`cargo test`, `npm test`) and reads source files autonomously
4. Output: structured markdown with categories, line references, and severity ratings

### How to Treat Outputs

1. **Save the raw output as-is** (`_original.md` suffix) — never edit the original
2. **Verify every claim** against actual code before prioritizing
   a. Line numbers are approximate (off by 10-50 lines) — use as search hints
   b. Cross-reference with framework documentation (Codex misses framework magic)
   c. Check if "bugs" are actually intentional design choices
3. **Create an evaluation companion** (`_evaluation.md`) with verdict per claim:
   a. VALID — confirmed, action warranted
   b. PARTIALLY VALID — directionally correct, root cause misstated
   c. INVALID — wrong, framework-unaware, or intentional design
4. **Prioritize by actual severity**, not Codex's categorization
5. **Batch implement** confirmed fixes in a single version bump

### What Codex Is Good At

- Structural analysis: monolithic files, mixed concerns, naming inconsistencies
- Pattern matching: dead code, unused listeners, hardcoded values
- Cross-file consistency: DTO shape mismatches, ID convention drift
- Test coverage gaps: untested paths, dead assertions

### What Codex Gets Wrong

- Framework behavior (e.g., Tauri auto-serialization, serde defaults)
- Design intent ("intentional bypass" vs "accidental omission")
- Security context (desktop OAuth vs web OAuth threat models)
- Hallucinated file references (test files that don't exist)

### Expected Hit Rate

~65% valid/partially-valid findings. The 35% invalid rate means **never bulk-approve**.

## Anti-pattern

- Acting on all findings without verification
- Treating line numbers as exact
- Assuming all "security" findings are vulnerabilities without considering deployment context
- Skipping the evaluation companion (raw audit without verdicts becomes stale noise)

## Related

- `universal-ai-code-review-verification.md` — detailed verification methodology
- `stack-tauri-serde-camelcase-auto-conversion.md` — specific framework miss from this audit
