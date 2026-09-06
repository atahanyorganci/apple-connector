#!/usr/bin/env bash
# Build every fuzz target and run it briefly against its committed seeds.
#
# This is a smoke test, not a fuzzing campaign: it proves the targets still
# compile against the current APIs and that the seeds do not crash them. Real
# campaigns run out of band with a longer FUZZ_MAX_TOTAL_TIME.
#
# Generated inputs go to a scratch corpus so the committed seeds stay a curated
# set rather than growing on every run.
set -euo pipefail

RUNS="${FUZZ_RUNS:-2000}"
MAX_TIME="${FUZZ_MAX_TOTAL_TIME:-20}"
MAX_LEN="${FUZZ_MAX_LEN:-65536}"

TARGETS=(
  typedstream_parse
  notes_body_decode
  vcard_parse
  icalendar_parse
  caldav_multistatus
  carddav_multistatus
)

cd "$(dirname "$0")/../fuzz"

scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT

for target in "${TARGETS[@]}"; do
  echo "==> fuzzing $target"
  mkdir -p "$scratch/$target"
  cargo fuzz run "$target" "$scratch/$target" "seeds/$target" -- \
    -runs="$RUNS" \
    -max_total_time="$MAX_TIME" \
    -max_len="$MAX_LEN"
done

echo "all fuzz targets completed without a crash"
