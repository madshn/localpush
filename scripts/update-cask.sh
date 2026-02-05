#!/usr/bin/env bash
# Updates the Homebrew Cask formula with new version and SHA
set -euo pipefail

VERSION="${1:?Usage: update-cask.sh <version>}"
TAP_DIR="/Users/madsnissen/dev/homebrew-localpush"
CASK_FILE="$TAP_DIR/Casks/localpush.rb"

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
