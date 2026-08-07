#!/usr/bin/env bash
# Fail CI when handlers leak internal errors or use coarse ApiError helpers.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

TARGET='packages/apple-connector/src'

fail=0

check() {
  local pattern=$1
  local message=$2
  local matches
  matches="$(rg -n --glob '*.rs' -e "$pattern" "$TARGET" || true)"
  if [[ -n "$matches" ]]; then
    echo "$message" >&2
    echo "$matches" >&2
    fail=1
  fi
}

check 'ApiError::internal\([^)]*\.to_string\(\)' \
  'Banned: ApiError::internal(...to_string()) leaks backend details'

check 'ApiError::not_found\(' \
  'Banned: ApiError::not_found — use ErrorCode::*NotFound via ApiError::new/with_details'

check 'ApiError::validation(_with_details)?\(' \
  'Banned: ApiError::validation* — use InvalidLimit/InvalidCursor/InvalidParameter/InvalidTimestamp'

check 'ApiError::forbidden\(' \
  'Banned: ApiError::forbidden — use typed permission ErrorCode values'

check 'ApiError::service_unavailable\(' \
  'Banned: ApiError::service_unavailable — use domain *_unavailable / *_database_unavailable codes'

check 'ApiError::conflict\(' \
  'Banned: ApiError::conflict — use AmbiguousEventKitMatch or Conflict via ApiError::with_message'

check 'ApiError::unprocessable(_with_details)?\(' \
  'Banned: ApiError::unprocessable* — use typed ErrorCode via ApiError::new/with_details'

if [[ "$fail" -ne 0 ]]; then
  exit 1
fi

echo 'api error leakage check passed'
