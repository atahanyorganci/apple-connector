#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
schema="${root}/contacts.schema.sql"
seed="${root}/seed.sql"
db="${1:-${root}/empty.abcddb}"

if [[ ! -f ${schema} ]]; then
  echo "Missing schema at ${schema}; run dump-schema.sh first" >&2
  exit 1
fi

rm -f "${db}"
sqlite3 "${db}" <"${schema}"
if [[ -f ${seed} ]]; then
  sqlite3 "${db}" <"${seed}"
fi

echo "Created fixture database at ${db}"
