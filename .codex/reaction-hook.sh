#!/usr/bin/env bash
set -uo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TARGET="${ROOT_DIR}/bin/reaction-hook.sh"

# Delegate to bin/reaction-hook.sh if it exists; otherwise no-op.
[ -x "$TARGET" ] && exec "$TARGET" "$@"
exit 0
