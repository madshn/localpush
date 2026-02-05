# Homebrew Tap Release Process

This document describes how to maintain the Homebrew Cask for LocalPush.

## Repository Structure

Two repositories:

1. **Main repo:** `github.com/madshn/localpush`
   - Contains the app source code
   - GitHub Actions builds DMG on tag push
   - `scripts/update-cask.sh` updates the Cask formula

2. **Tap repo:** `github.com/madshn/homebrew-localpush`
   - Contains only the Homebrew Cask formula
   - Users add via `brew tap madshn/localpush`
   - Formula points to GitHub Releases for DMG

## Release Workflow

### 1. Tag and Release

```bash
# In main repo: /Users/madsnissen/dev/localpush
git tag v0.1.0
git push origin v0.1.0
```

This triggers GitHub Actions to:
- Build universal DMG
- Create GitHub Release
- Upload DMG to release assets

### 2. Update Cask Formula

After the release is published (wait for GitHub Actions to complete):

```bash
# In main repo: /Users/madsnissen/dev/localpush
./scripts/update-cask.sh 0.1.0
```

This script:
- Downloads the DMG from GitHub Releases
- Computes SHA256 hash
- Updates version and hash in the Cask formula
- Prints next steps

### 3. Commit and Push Tap

```bash
cd /Users/madsnissen/dev/homebrew-localpush
git add Casks/localpush.rb
git commit -m "Update to v0.1.0"
git push origin main
```

### 4. Users Update

Users will see the new version within ~24 hours (Homebrew cache).

To force immediate update:
```bash
brew update
brew upgrade --cask localpush
```

## First-Time Tap Setup

The tap repo needs to be pushed to GitHub once:

```bash
cd /Users/madsnissen/dev/homebrew-localpush

# Create GitHub repo (using gh CLI)
gh repo create madshn/homebrew-localpush --public --source=. --remote=origin --description="Homebrew tap for LocalPush"

# Push
git push -u origin main
```

## Cask Details

**Bundle ID:** `com.localpush.app` (from `src-tauri/tauri.conf.json`)

**DMG URL Pattern:**
```
https://github.com/madshn/localpush/releases/download/v{VERSION}/LocalPush_{VERSION}_universal.dmg
```

**Minimum macOS:** 12.0 (Monterey)

**Auto-updates:** Enabled (Tauri updater)

## Uninstall Paths

Standard uninstall removes:
- `/Applications/LocalPush.app`

`brew uninstall --zap` additionally removes:
- `~/Library/Application Support/com.localpush.app/`
- `~/Library/Logs/com.localpush.app/`
- `~/Library/Caches/com.localpush.app/`
- `~/Library/Preferences/com.localpush.app.plist`
- `~/Library/Saved Application State/com.localpush.app.savedState/`

## Testing

Test the Cask locally before pushing:

```bash
# Install from local tap
brew install --cask /Users/madsnissen/dev/homebrew-localpush/Casks/localpush.rb

# Verify installation
ls -la /Applications/LocalPush.app

# Test uninstall
brew uninstall --cask localpush

# Test zap
brew install --cask /Users/madsnissen/dev/homebrew-localpush/Casks/localpush.rb
brew uninstall --zap --cask localpush
ls ~/Library/Application\ Support/ | grep localpush  # Should be empty
```

## Troubleshooting

### SHA Mismatch

If users report SHA mismatch:
1. Download the DMG manually
2. Compute hash: `shasum -a 256 LocalPush_0.1.0_universal.dmg`
3. Update Cask with correct hash
4. Push fix

### DMG Not Found

If the update script fails to download:
- Check GitHub Release is published
- Verify DMG filename matches pattern
- Check GitHub Actions completed successfully
