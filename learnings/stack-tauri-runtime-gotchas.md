# Tauri 2.0 Runtime Gotchas

**Date:** 2026-02-05
**Category:** stack
**Tags:** tauri, tokio, runtime, async, spawn, threading
**Source:** LocalPush build session — runtime crash on startup
**Confidence:** high

---

## Problem

Tauri's `setup` hook runs on the main thread **before** the Tokio runtime is fully available. Calling `tokio::spawn()` inside `setup` causes a panic:

```
thread 'main' panicked at src/delivery_worker.rs:82:5:
there is no reactor running, must be called from the context of a Tokio 1.x runtime
```

The app initializes fully (state, ledger, config, file watcher, source manager) then crashes at the first `tokio::spawn()` call.

## Pattern

**Always use `tauri::async_runtime::spawn()` instead of `tokio::spawn()` in Tauri apps.**

Tauri manages its own async runtime. The `tauri::async_runtime` module wraps Tokio but ensures the runtime is available from any context, including the setup hook.

```rust
// WRONG — panics in setup hook
pub fn spawn_worker(...) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move { ... })
}

// CORRECT — works everywhere in Tauri
pub fn spawn_worker(...) -> tauri::async_runtime::JoinHandle<()> {
    tauri::async_runtime::spawn(async move { ... })
}
```

Also applies to:
- `tokio::time::interval` — works fine inside the spawned future (the future runs on the Tauri runtime)
- `tokio::time::sleep` — same, fine inside spawned futures
- The issue is only with the **spawn entry point**, not code inside the future

## Anti-pattern

- Using `tokio::spawn` anywhere in Tauri application code
- Using `tokio::runtime::Runtime::new()` to create a second runtime (wasteful, conflicts)
- Assuming Tauri's setup hook has Tokio context

## Related

- Tauri State wraps in Arc internally — don't double-wrap: `State<'_, AppState>` not `State<'_, Arc<AppState>>`
- Tauri's `generate_context!()` macro requires icon files at compile time (RGBA PNG, not RGB)
- `std::mem::forget(_guard)` pattern needed to keep tracing-appender file writer alive
