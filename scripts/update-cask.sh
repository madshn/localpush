#!/usr/bin/env bash
# Updates the Homebrew Cask formula with new version and SHA
set -euo pipefail

VERSION="${1:?Usage: update-cask.sh <version>}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
DEFAULT_TAP_DIR="$(cd "$REPO_ROOT/.." && pwd)/homebrew-localpush"
DEFAULT_TAP_DIR="$REPO_ROOT/tmp/homebrew-localpush"
TAP_DIR="${LOCALPUSH_TAP_DIR:-$DEFAULT_TAP_DIR}"
CASK_FILE="$TAP_DIR/Casks/localpush.rb"

if [ ! -f "$CASK_FILE" ]; then
  echo "Homebrew tap repo not found at: $TAP_DIR" >&2
  echo "Set LOCALPUSH_TAP_DIR to your local homebrew-localpush checkout." >&2
  exit 1
fi

# Download DMG and compute SHA
DMG_URL="https://github.com/madshn/localpush/releases/download/v${VERSION}/LocalPush_${VERSION}_universal.dmg"
echo "Downloading $DMG_URL..."
SHA=$(curl -sL "$DMG_URL" | shasum -a 256 | cut -d' ' -f1)

# Update version and SHA in cask
sed -i '' "s/version \".*\"/version \"${VERSION}\"/" "$CASK_FILE"
sed -i '' "s/sha256 .*/sha256 \"${SHA}\"/" "$CASK_FILE"

echo ""
echo "✅ Updated Cask to v${VERSION}"
echo "   SHA256: ${SHA}"
echo ""
echo "Next steps:"
echo "1. cd $TAP_DIR"
echo "2. git add Casks/localpush.rb"
echo "3. git commit -m \"Update to v${VERSION}\""
echo "4. git push origin main"
