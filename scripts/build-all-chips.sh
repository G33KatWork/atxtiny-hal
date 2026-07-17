#!/usr/bin/env bash
#
# Build the HAL library and every example for every supported chip.
#
# For each chip this overrides the -C target-cpu that .cargo/config.toml
# pins to the attiny817 (an environment RUSTFLAGS takes precedence over
# config-file rustflags). Cargo skips examples whose `required-features`
# the chip does not provide, which is exactly the "build examples only
# where the peripheral exists" story.
#
# Usage: scripts/build-all-chips.sh [cargo-args...]
#   e.g. scripts/build-all-chips.sh --release

set -euo pipefail

cd "$(dirname "$0")/.."

CHIPS=(
    # tinyAVR 0-series
    attiny202 attiny204 attiny402 attiny404 attiny804 attiny1604 attiny1606
    # tinyAVR 1-series
    attiny212 attiny214 attiny412 attiny414 attiny416 attiny417
    attiny816 attiny817 attiny1614 attiny1617 attiny3217
)

failed=()

for chip in "${CHIPS[@]}"; do
    echo "==== ${chip}"
    if ! RUSTFLAGS="-C target-cpu=${chip}" \
         cargo build --lib --examples --features "${chip}" "$@"; then
        failed+=("${chip}")
    fi
done

if ((${#failed[@]})); then
    echo "FAILED chips: ${failed[*]}" >&2
    exit 1
fi

echo "All ${#CHIPS[@]} chips built successfully."
