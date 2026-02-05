# LocalPush Installation & Uninstall Guide

## Install via Homebrew (Recommended)

```bash
brew tap madshn/localpush
brew install --cask localpush
```

## Install from DMG

1. Download the latest DMG from [GitHub Releases](https://github.com/madshn/localpush/releases)
2. Open the DMG and drag LocalPush to Applications
3. Launch from Applications or Spotlight

## Uninstall

### Via Homebrew
```bash
# Standard uninstall
brew uninstall --cask localpush

# Full cleanup (removes ALL data including delivery history)
brew uninstall --zap --cask localpush
```

### Manual Uninstall
1. Quit LocalPush (right-click tray icon → Quit)
2. Delete `/Applications/LocalPush.app`
3. Optionally remove data:
   - `~/Library/Application Support/com.localpush.app/` — Database, config
   - `~/Library/Logs/com.localpush.app/` — Log files
   - `~/Library/Caches/com.localpush.app/` — Cache
   - `~/Library/Preferences/com.localpush.app.plist` — Preferences
   - `~/Library/Saved Application State/com.localpush.app.savedState/` — Window state

## Auto-Update

LocalPush checks for updates on startup (configurable in Settings).
Updates are downloaded from GitHub Releases automatically.

To disable: Settings → uncheck "Automatically check for app updates"

## Version Update (Manual)

```bash
brew upgrade --cask localpush
```

## System Requirements

- macOS 12.0 (Monterey) or later
- Universal binary (Apple Silicon and Intel)

## Troubleshooting

### "LocalPush.app is damaged and can't be opened"

This is a Gatekeeper security warning. To fix:

```bash
xattr -cr /Applications/LocalPush.app
```

Then launch the app again.

### Permission Issues

LocalPush needs:
- **File System Access** — To watch files you configure
- **Network Access** — To send webhooks

These are requested on first use.
