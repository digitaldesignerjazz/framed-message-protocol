#!/usr/bin/env bash
#
# Convenience wrapper for running cargo-fuzz targets for Framed Message Protocol
# Usage:
#   ./fuzz.sh decode_raw          # run main security target
#   ./fuzz.sh roundtrip           # run roundtrip target
#   ./fuzz.sh decode_raw 300      # run for 5 minutes

set -euo pipefail

TARGET=${1:-decode_raw}
DURATION=${2:-}

cd "$(dirname "$0")/rust" || exit 1

if ! command -v cargo-fuzz &> /dev/null; then
    echo "cargo-fuzz not found. Installing..."
    cargo install cargo-fuzz
fi

CMD=(cargo +nightly fuzz run "$TARGET")

if [[ -n "$DURATION" ]]; then
    # Convert minutes to seconds for -max_total_time
    SECONDS=$(( DURATION * 60 ))
    CMD+=(-- -max_total_time="$SECONDS")
fi

 echo "Running: ${CMD[*]}"
 exec "${CMD[@]}"