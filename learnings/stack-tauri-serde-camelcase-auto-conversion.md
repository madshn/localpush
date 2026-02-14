# Tauri Auto-Converts snake_case to camelCase in IPC Serialization

**Date:** 2026-02-14
**Category:** stack
**Tags:** tauri, serde, ipc, serialization, camelCase, snake_case
**Source:** Codex audit falsely flagged Rust snake_case ↔ TS camelCase as a mismatch
**Confidence:** high

---

## Problem

Rust structs use snake_case fields. TypeScript interfaces use camelCase. This looks like a contract mismatch when reviewing code across the IPC boundary.

## Pattern

Tauri 2.x automatically converts Rust `#[derive(Serialize)]` struct fields from snake_case to camelCase when serializing responses to the frontend. No `#[serde(rename_all)]` attribute is needed — it's the default behavior.

This means:
- Backend: `pending_count: usize` → Frontend receives: `pendingCount`
- Backend: `last_delivery: Option<String>` → Frontend receives: `lastDelivery`

**Frontend hooks SHOULD use camelCase interfaces** — that's what actually arrives over the wire.

## Anti-pattern

- Flagging snake_case Rust ↔ camelCase TypeScript as a bug (it's framework-handled)
- Using snake_case in TypeScript interfaces for Tauri responses (will cause runtime type mismatches)
- Adding explicit `#[serde(rename_all = "camelCase")]` — redundant, Tauri already does this

## Related

- Tauri IPC documentation on serialization behavior
- `stack-tauri-runtime-gotchas.md`
