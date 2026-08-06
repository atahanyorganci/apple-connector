#!/usr/bin/env bash
# Reject non-macro sqlx::query / sqlx::query_as in production source.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SRC="${ROOT}/packages/apple-connector/src"

allow() {
  case "$1" in
  "${SRC}/fixtures.rs") return 0 ;;
  "${SRC}/api/handlers/attachments.rs") return 0 ;;
  esac
  return 1
}

mapfile -t matches < <(
  rg 'sqlx::query(_as)?\(' "${SRC}" -n | rg -v '!' || true
)

if ((${#matches[@]} == 0)); then
  exit 0
fi

failed=0
for entry in "${matches[@]}"; do
  file="${entry%%:*}"
  if allow "${file}"; then
    continue
  fi
  echo "runtime SQL API not allowed: ${entry}" >&2
  failed=1
done

exit "${failed}"
