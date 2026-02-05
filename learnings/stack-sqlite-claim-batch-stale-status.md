# SQLite claim_batch Stale Status Bug

**Date:** 2026-02-05
**Category:** stack
**Tags:** sqlite, ledger, status, claim, select-then-update
**Source:** LocalPush build — test_enqueue_and_claim failure
**Confidence:** high

---

## Problem

The `claim_batch` method follows a SELECT-then-UPDATE pattern:

1. SELECT entries WHERE status = 'pending'
2. UPDATE those entries SET status = 'in_flight'
3. Return the entries

But the returned Vec contains the **original SELECT results** with `status = Pending`, not the updated `InFlight` status. Tests asserting `entry.status == InFlight` fail.

## Pattern

Map the status before returning:

```rust
fn claim_batch(&self, limit: usize) -> Result<Vec<DeliveryEntry>, LedgerError> {
    // ... SELECT entries ...
    // ... UPDATE to in_flight ...

    // Fix: map status to reflect the UPDATE
    Ok(entries.into_iter().map(|mut e| {
        e.status = DeliveryStatus::InFlight;
        e
    }).collect())
}
```

Alternative approaches (not used, but valid):
- Re-SELECT after UPDATE (extra query, wasteful)
- Use `RETURNING` clause (SQLite 3.35+, not all rusqlite versions support it)
- Use a single UPDATE...RETURNING statement

## Anti-pattern

- Returning raw SELECT results after a status-changing UPDATE
- Assuming in-memory structs reflect database state after writes
- Not testing the return value's status field (only testing side effects)

## Related

- Same pattern applies to any SELECT-then-UPDATE workflow
- The ledger uses a 5-state machine: Pending → InFlight → Delivered/Failed/DLQ
- `recover_orphans()` resets stale InFlight entries back to Failed on startup
