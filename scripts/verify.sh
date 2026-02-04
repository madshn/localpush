#!/usr/bin/env bash
#
# LocalPush Verification Script
# Runs all verification gates in sequence with structured output
# Exit codes: 0=all pass, 1=failure with errors, 2=setup issue

set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CARGO_MANIFEST="$PROJECT_ROOT/src-tauri/Cargo.toml"

# Ensure tools are available
export PATH="$HOME/.cargo/bin:$PATH"

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Temp files for capturing output
TEMP_OUT=$(mktemp)
TEMP_ERR=$(mktemp)

# Cleanup on exit
trap 'rm -f "$TEMP_OUT" "$TEMP_ERR"' EXIT

# Track which gates passed
GATES_PASSED=0
TOTAL_GATES=5

print_header() {
    echo ""
    echo "=================================================="
    echo "  LocalPush Verification Suite"
    echo "=================================================="
    echo ""
}

print_gate() {
    echo -e "${YELLOW}Running:${NC} $1"
}

print_pass() {
    echo -e "${GREEN}PASS:${NC} $1"
    ((GATES_PASSED++))
}

print_fail() {
    echo -e "${RED}FAIL:${NC} $1"
}

run_gate() {
    local gate_name="$1"
    local gate_description="$2"
    shift 2
    local cmd=("$@")

    print_gate "$gate_description"

    if "${cmd[@]}" > "$TEMP_OUT" 2> "$TEMP_ERR"; then
        print_pass "$gate_name"
        return 0
    else
        print_fail "$gate_name"

        # Output structured error markers
        echo ""
        echo "===FAILED_GATE=$gate_name==="
        echo "===ERRORS_START==="

        # Print stderr first (usually has the actual errors)
        if [ -s "$TEMP_ERR" ]; then
            cat "$TEMP_ERR"
        fi

        # Then stdout (may have additional context)
        if [ -s "$TEMP_OUT" ]; then
            cat "$TEMP_OUT"
        fi

        echo "===ERRORS_END==="
        echo ""

        return 1
    fi
}

verify_tools() {
    local missing_tools=()

    command -v cargo >/dev/null 2>&1 || missing_tools+=("cargo")
    command -v npm >/dev/null 2>&1 || missing_tools+=("npm")
    command -v npx >/dev/null 2>&1 || missing_tools+=("npx")

    if [ ${#missing_tools[@]} -gt 0 ]; then
        echo -e "${RED}ERROR:${NC} Missing required tools: ${missing_tools[*]}"
        echo "Please install missing tools and try again."
        return 2
    fi

    if [ ! -f "$CARGO_MANIFEST" ]; then
        echo -e "${RED}ERROR:${NC} Cargo.toml not found at: $CARGO_MANIFEST"
        return 2
    fi

    if [ ! -f "$PROJECT_ROOT/package.json" ]; then
        echo -e "${RED}ERROR:${NC} package.json not found at: $PROJECT_ROOT"
        return 2
    fi

    return 0
}

# Main execution
main() {
    print_header

    # Verify setup
    if ! verify_tools; then
        exit 2
    fi

    echo "Project root: $PROJECT_ROOT"
    echo ""

    # Gate 1: Cargo check
    run_gate \
        "cargo-check" \
        "Gate 1/5: Rust compilation (cargo check)" \
        cargo check --manifest-path "$CARGO_MANIFEST" --all-targets || exit 1

    # Gate 2: Cargo test
    run_gate \
        "cargo-test" \
        "Gate 2/5: Rust tests (cargo test)" \
        cargo test --manifest-path "$CARGO_MANIFEST" || exit 1

    # Gate 3: Cargo clippy
    run_gate \
        "cargo-clippy" \
        "Gate 3/5: Rust linting (cargo clippy)" \
        cargo clippy --manifest-path "$CARGO_MANIFEST" --all-targets -- -D warnings || exit 1

    # Gate 4: Frontend build
    cd "$PROJECT_ROOT"
    run_gate \
        "npm-build" \
        "Gate 4/5: Frontend build (npm run build)" \
        npm run build || exit 1

    # Gate 5: Frontend tests
    run_gate \
        "vitest" \
        "Gate 5/5: Frontend tests (npx vitest run)" \
        npx vitest run || exit 1

    # All gates passed
    echo ""
    echo "=================================================="
    echo -e "${GREEN}SUCCESS:${NC} All $TOTAL_GATES gates passed"
    echo "=================================================="
    echo ""

    return 0
}

main "$@"
