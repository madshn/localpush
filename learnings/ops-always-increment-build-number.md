# Always Increment Build Number for Each Build

**Date:** 2026-02-14
**Category:** ops
**Tags:** build, versioning, tauri
**Source:** user instruction
**Confidence:** high

---

## Problem

Multiple builds were produced with the same version number (e.g., 0.2.3), making it impossible to tell which build is actually running. This caused confusion when verifying that a new build was properly installed (e.g., context menu still showing 0.2.2 after what should have been a 0.2.3 install).

## Pattern

**Always increment the version before every `npx tauri build`.**

- Bump patch version at minimum (e.g., 0.2.3 → 0.2.4)
- Update version in all relevant files:
  - `src-tauri/Cargo.toml` (version field)
  - `src-tauri/tauri.conf.json` (version field)
  - `package.json` (version field)
- Verify the version appears correctly in the built artifact

## Anti-pattern

Building multiple times with the same version number. This makes it unclear whether a fresh build was properly installed, and creates confusion when debugging issues across builds.

## Related

- learnings/stack-tauri-dev-prod-credential-divergence.md (another build/install lesson)
