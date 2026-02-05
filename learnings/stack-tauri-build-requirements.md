# Tauri Build Requirements and Icon Gotchas

**Date:** 2026-02-05
**Category:** stack
**Tags:** tauri, icons, rgba, png, build, clippy
**Source:** LocalPush build — icon generation failures, clippy gaps
**Confidence:** high

---

## Problem 1: Icons Must Be RGBA

Tauri's `generate_context!()` macro validates icon files at **compile time**. Icons must be:
- PNG format with RGBA color type (color type 6, not RGB type 2)
- Specific sizes: 32x32, 128x128, 128x128@2x (256x256)
- Located at paths specified in `tauri.conf.json`

An RGB PNG (without alpha channel) causes:
```
error: failed to load icon: invalid color type
```

Generate correct icons with Python:
```python
from PIL import Image
img = Image.new('RGBA', (128, 128), (76, 175, 80, 255))
img.save('icon.png')
```

## Problem 2: Icons in .gitignore

`src-tauri/icons/` is often in `.gitignore` (Tauri's default template). Use `git add -f` to force-add generated icons.

## Problem 3: cargo clippy --tests

`cargo clippy` without `--tests` flag misses test-only warnings:
- `assert_eq!(x, true)` should be `assert!(x)` (bool_assert_comparison)
- Unused imports in test modules
- Dead code in test helpers

## Pattern

Full verification gate order:
```bash
cargo check                           # Compilation
cargo test                            # All tests pass
cargo clippy --all-targets -- -D warnings  # Lint (--all-targets catches test code)
npm run build                         # Frontend build
npx vitest run                        # Frontend tests
```

Use `--all-targets` instead of `--tests` to also catch benchmarks and examples.

## Anti-pattern

- Generating RGB PNGs for Tauri icons
- Running `cargo clippy` without `--all-targets`
- Forgetting to force-add icons past `.gitignore`
- Treating `cargo check` success as sufficient (clippy catches more)

## Related

- `tauri.conf.json` icon paths are relative to the `src-tauri` directory
- macOS `.icns` is generated automatically by Tauri from the PNG sources during `cargo build --release`
- `scripts/verify.sh` automates all 5 gates with structured error output
