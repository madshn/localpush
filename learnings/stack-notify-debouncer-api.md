# notify-debouncer-full API Changes

**Date:** 2026-02-05
**Category:** stack
**Tags:** notify, debouncer, file-watching, fsevents, deprecated
**Source:** LocalPush build — compilation errors from deprecated API
**Confidence:** high

---

## Problem

`notify-debouncer-full` v0.4 deprecated the `.watcher()` method that returns the inner `RecommendedWatcher`. Sub-agents and LLMs trained on older docs will write:

```rust
let debouncer = new_debouncer(...)?;
debouncer.watcher().watch(path, RecursiveMode::NonRecursive)?;
```

This compiles with a deprecation warning but is the wrong pattern.

## Pattern

Call `.watch()` and `.unwatch()` directly on the `Debouncer` instance:

```rust
let mut debouncer = new_debouncer(
    Duration::from_secs(2),
    None,  // tick_rate
    tx,    // channel sender
)?;

debouncer.watch(path, RecursiveMode::NonRecursive)?;
// later:
debouncer.unwatch(path)?;
```

Also, the channel type needs explicit annotation:

```rust
let (tx, rx) = std::sync::mpsc::channel::<notify_debouncer_full::DebounceEventResult>();
```

Without the turbofish, Rust can't infer the type from usage alone.

## Anti-pattern

- Using `.watcher()` to get inner watcher — deprecated, will be removed
- Omitting channel type annotation — causes "type annotations needed" error
- Using `Ok(events: Vec<DebouncedEvent>)` in match arms — Rust doesn't support type annotations in patterns

## Related

- EventHandler type can get complex: `Arc<Mutex<Option<Arc<dyn Fn(FileEvent) + Send + Sync>>>>`
- Create a type alias (`type EventHandler = ...`) to satisfy clippy::type_complexity
- The debouncer spawns its own thread — no need for async, just mpsc channels
