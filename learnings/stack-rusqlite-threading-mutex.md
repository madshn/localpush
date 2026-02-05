# rusqlite Connection Threading

**Date:** 2026-02-05
**Category:** stack
**Tags:** rusqlite, sqlite, threading, mutex, sync, send
**Source:** LocalPush build — 68 compilation errors from threading issues
**Confidence:** high

---

## Problem

`rusqlite::Connection` contains `RefCell` internally, making it `Send` but **not `Sync`**. When shared across threads (e.g., via `Arc<dyn Trait>` in Tauri state), the compiler rejects it:

```
error[E0277]: `RefCell<InnerConnection>` cannot be shared between threads safely
```

This cascades — every struct holding a Connection becomes non-Sync, breaking Tauri's `app.manage()` which requires `Send + Sync`.

## Pattern

Wrap the Connection in `Mutex<Connection>`:

```rust
pub struct DeliveryLedger {
    conn: Mutex<Connection>,  // NOT just Connection
}

impl DeliveryLedgerTrait for DeliveryLedger {
    fn enqueue(&self, ...) -> Result<...> {
        let conn = self.conn.lock().unwrap();
        conn.execute(...)?;
        Ok(...)
    }
}
```

This applies to **every** struct that holds a rusqlite Connection:
- `DeliveryLedger` — the WAL-backed delivery queue
- `AppConfig` — the key-value config store

Both need `Mutex<Connection>`.

## Anti-pattern

- Wrapping in `Arc<Connection>` — Arc provides shared ownership, not thread safety for non-Sync types
- Using `RefCell<Connection>` — adds another RefCell layer, still not Sync
- Using `RwLock` — overkill for SQLite (it serializes writes anyway), Mutex is correct
- Making Connection `pub` — always encapsulate behind methods that lock internally

## Related

- Tauri's `app.manage()` requires `Send + Sync + 'static`
- All trait objects in state (`Arc<dyn Trait>`) inherit Send + Sync requirements
- `open_in_memory()` should be `pub` (not `#[cfg(test)]`) if integration tests need it
