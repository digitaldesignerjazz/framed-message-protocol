#!/usr/bin/env bash
#
# fuzz.sh - Convenient wrapper for cargo-fuzz on Framed Message Protocol
#
# Usage examples:
#   ./fuzz.sh decode_raw              # Run main security target (default 30 min if no duration given)
#   ./fuzz.sh decode_raw 15             # Run decode_raw for 15 minutes
#   ./fuzz.sh roundtrip                 # Run roundtrip target
#   ./fuzz.sh both 20                   # Run both targets sequentially (20 min each)
#   ./fuzz.sh decode_raw -- -max_len=1024   # Pass extra flags to cargo-fuzz

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FUZZ_DIR="$SCRIPT_DIR/rust/fuzz"

print_usage() {
    echo -e "${BLUE}Framed Message Protocol - Fuzzing Helper${NC}"
    echo ""
    echo "Usage:"
    echo "  $0 <target> [minutes] [extra cargo-fuzz args...]"
    echo ""
    echo "Targets:"
    echo "  decode_raw     Main security target (raw byte decoding)"
    echo "  roundtrip      Roundtrip + invariant checking"
    echo "  both           Run decode_raw then roundtrip"
    echo ""
    echo "Examples:"
    echo "  $0 decode_raw"
    echo "  $0 decode_raw 30"
    echo "  $0 roundtrip 10"
    echo "  $0 both 15"
    echo "  $0 decode_raw -- -jobs=4 -max_len=2048"
}

if [[ $# -eq 0 ]]; then
    print_usage
    exit 1
fi

TARGET="$1"
shift

DURATION_MINUTES=""
EXTRA_ARGS=()

# Parse arguments
while [[ $# -gt 0 ]]; do
    case "$1" in
        [0-9]*)
            DURATION_MINUTES="$1"
            shift
            ;;
        --)
            shift
            EXTRA_ARGS=("$@")
            break
            ;;
        *)
            echo -e "${RED}Unknown argument: $1${NC}"
            print_usage
            exit 1
            ;;
    esac
done

# Validate target
case "$TARGET" in
    decode_raw|roundtrip|both)
        ;;
    *)
        echo -e "${RED}Unknown target: $TARGET${NC}"
        print_usage
        exit 1
        ;;
esac

# Ensure we are in the right place
if [[ ! -d "$FUZZ_DIR" ]]; then
    echo -e "${RED}Error: Could not find fuzz directory at $FUZZ_DIR${NC}"
    echo "Please run this script from the repository root."
    exit 1
fi

cd "$FUZZ_DIR" || exit 1

# Check for cargo-fuzz
if ! command -v cargo-fuzz &> /dev/null; then
    echo -e "${YELLOW}cargo-fuzz not found. Installing...${NC}"
    cargo install cargo-fuzz
fi

run_fuzzer() {
    local t="$1"
    local minutes="$2"

    echo -e "${GREEN}=== Starting fuzz target: $t ===${NC}"

    local cmd=(cargo +nightly fuzz run "$t")

    if [[ -n "$minutes" ]]; then
        local seconds=$(( minutes * 60 ))
        cmd+=(-- -max_total_time="$seconds" "${EXTRA_ARGS[@]}")
    else
        cmd+=(-- "${EXTRA_ARGS[@]}")
    fi

    echo -e "${BLUE}Command: ${cmd[*]}${NC}"
    "${cmd[@]}"
}

case "$TARGET" in
    decode_raw|roundtrip)
        run_fuzzer "$TARGET" "$DURATION_MINUTES"
        ;;
    both)
        echo -e "${YELLOW}Running both targets sequentially...${NC}"
        run_fuzzer "decode_raw" "$DURATION_MINUTES"
        echo ""
        run_fuzzer "roundtrip" "$DURATION_MINUTES"
        ;;
esac

echo -e "${GREEN}Fuzzing session finished.${NC}"