# Build-Fix Loop Pattern

**Date:** 2026-02-05
**Category:** ops
**Tags:** verification, build-loop, automation, gates, subagent
**Source:** LocalPush build — autonomous fix loop design
**Confidence:** high

---

## Problem

After parallel sub-agents write code, verification requires multiple rounds:
1. First round: 68 errors (cross-file naming mismatches)
2. Second round: 9 errors (threading, deprecated APIs)
3. Third round: 1 test failure (stale status bug)
4. Fourth round: 6 clippy warnings

Each round is: run gates → categorize errors → dispatch fix agents → repeat.

## Pattern

**Structured verification script + autonomous fix loop:**

`scripts/verify.sh` runs 5 ordered gates:
```bash
Gate 1: cargo check           # Does it compile?
Gate 2: cargo test            # Do tests pass?
Gate 3: cargo clippy --all-targets -- -D warnings  # Lint clean?
Gate 4: npm run build         # Frontend compiles?
Gate 5: npx vitest run        # Frontend tests?
```

Gates are ordered by dependency — no point running tests if compilation fails.

Output format for agent consumption:
```
===GATE_1_PASSED===
===GATE_2_FAILED===
===ERRORS_START===
test test_enqueue_and_claim ... FAILED
assertion `left == right` failed
  left: Pending
  right: InFlight
===ERRORS_END===
```

**Fix loop protocol:**
1. Run verify.sh
2. If all gates pass → done
3. Parse first failed gate's errors
4. Group errors by domain (file/module/error type)
5. Dispatch parallel fix agents (one per domain)
6. After agents return, goto 1
7. Max 5 iterations (bail if not converging)

## Anti-pattern

- Fixing errors one at a time sequentially
- Running all gates when compilation fails (waste)
- Not grouping errors by domain before dispatching agents
- Infinite loops without a bail condition
- Running verify.sh inside sub-agents (they should write code, coordinator runs verification)

## Related

- Bob skill: `skills/build-fix-loop.md`
- Sub-agent consistency: `learnings/universal-subagent-cross-file-consistency.md`
- Always budget 2-3 fix rounds after parallel agent dispatch
