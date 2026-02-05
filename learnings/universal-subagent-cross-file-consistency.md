# Sub-Agent Cross-File Consistency

**Date:** 2026-02-05
**Category:** universal
**Tags:** subagent, parallel, consistency, naming, api-mismatch
**Source:** LocalPush build — multiple rounds of fixing agent output
**Confidence:** high

---

## Problem

When dispatching parallel sub-agents to write different files, they independently invent:
- **Type/struct names** — One agent writes `MockCredentialStore`, another writes `InMemoryCredentialStore`
- **API signatures** — One agent writes `fn new() -> Result<Self>`, another calls `Client::new()` without `?`
- **Module paths** — One agent uses `crate::mocks::RecordedWebhookClient`, another uses `crate::test_utils::MockWebhook`

These mismatches only surface at compile time, often producing 20+ cascading errors.

## Pattern

**Always verify cross-file references after parallel sub-agent work.**

Prevention strategies (in order of effectiveness):

1. **Provide exact signatures in agent prompts:**
   ```
   Implement WebhookClient trait. The constructor MUST be:
   pub fn new() -> Result<Self, WebhookError>

   The mock in mocks/mod.rs MUST be named:
   pub struct RecordedWebhookClient
   ```

2. **Sequential for coupled files:**
   Write the trait/interface first, then dispatch implementors that READ the trait file.

3. **Post-merge verification:**
   After all agents return, immediately run `cargo check` before doing anything else.
   Budget 1-2 fix rounds into the plan.

4. **Single agent for tightly coupled files:**
   If two files share more than 3 symbols, one agent should write both.

## Anti-pattern

- Dispatching agents for trait + all implementations simultaneously without shared context
- Assuming agents will use identical names for the same concept
- Trusting agent output without a compilation check
- Fixing errors one-at-a-time instead of batching (dispatch parallel fix agents per error domain)

## Related

- `notify-debouncer-full` API changed between versions — agents may write code for wrong version
- Rust match pattern syntax: agents sometimes write `Ok(x: Type)` which is invalid
- Channel type inference: agents may omit turbofish annotation that the compiler needs
