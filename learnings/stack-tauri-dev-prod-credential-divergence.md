# Dev/Prod Credential Store Divergence in Tauri Apps

**Date:** 2026-02-15
**Category:** stack
**Tags:** tauri, credentials, keychain, dev-mode, production, debugging
**Source:** Google Sheets target missing from production build — 4 credentials only in dev store
**Confidence:** high

---

## Problem

A Tauri app with separate credential backends for dev mode (file-based JSON) and production (macOS Keychain) can silently lose all target connections when switching from dev to release builds. Targets, bindings, and auth headers set up during `npx tauri dev` exist only in the dev credential store. The production build reads from Keychain, finds nothing, and silently skips restoration with a `tracing::warn`.

## Pattern

### How It Manifests

1. All setup and testing happens in dev mode (`npx tauri dev`)
2. Credentials stored in `dev-credentials.json` (file-based)
3. Build release → install → launch
4. Production uses `KeychainCredentialStore` → finds nothing
5. Targets silently skip with warn-level log → user sees missing targets
6. Cascading failures: deliveries fall through to wrong paths (webhook POST to non-webhook URLs)

### How to Detect

Check both stores when targets are missing:

```bash
# Dev store
cat ~/Library/Application\ Support/com.localpush.app/dev-credentials.json | python3 -m json.tool

# Keychain (production)
security dump-keychain 2>/dev/null | grep -A 10 'svce.*com.localpush'
```

If dev store has entries that Keychain doesn't, that's the divergence.

### How to Fix (Immediate)

Sync dev credentials to Keychain:

```python
import json, subprocess
with open('dev-credentials.json') as f:
    creds = json.load(f)
for key, value in creds.items():
    subprocess.run([
        'security', 'add-generic-password',
        '-s', 'com.localpush.app', '-a', key, '-w', value
    ])
```

### How to Prevent

1. **Startup sync**: On dev builds, mirror credentials to Keychain as well (write to both stores)
2. **Build-time check**: Before `cargo build --release`, verify Keychain has all expected credential keys
3. **Loud failures**: Escalate missing credentials from `warn` to `error` with user-visible notification, not just a log line
4. **First-run detection**: If production build finds config entries referencing targets but no credentials, surface a "re-authenticate" prompt

## Anti-pattern

- Assuming dev-mode setup carries over to production
- Using `tracing::warn` for user-impacting credential failures (invisible unless checking logs)
- Testing only in dev mode before shipping
- Silent `continue` in restoration loops — the user has no idea why targets disappeared

## Related

- `stack-tauri-serde-camelcase-auto-conversion.md` — another dev/prod behavioral difference
- Keychain service name: `com.localpush.app` (from `credential_store.rs`)
- Dev credential path: `~/Library/Application Support/com.localpush.app/dev-credentials.json`
