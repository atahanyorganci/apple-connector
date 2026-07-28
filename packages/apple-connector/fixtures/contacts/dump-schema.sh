#!/usr/bin/env bash
set -euo pipefail

sources_dir="${1:-${HOME}/Library/Application Support/AddressBook/Sources}"
root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
schema="${root}/contacts.schema.sql"

if [[ ! -d ${sources_dir} ]]; then
  echo "AddressBook sources directory not found at ${sources_dir}" >&2
  exit 1
fi

best_db=""
best_count=-1
best_mtime=0

shopt -s nullglob
for db in "${sources_dir}"/*/AddressBook-v*.abcddb; do
  [[ -f ${db} ]] || continue
  count="$(sqlite3 "${db}" "SELECT COUNT(*) FROM ZABCDRECORD WHERE Z_ENT = 22;" 2>/dev/null || echo 0)"
  mtime="$(date -r "${db}" +%s 2>/dev/null || echo 0)"
  if ((count > best_count)) || { ((count == best_count)) && ((mtime > best_mtime)); }; then
    best_db="${db}"
    best_count="${count}"
    best_mtime="${mtime}"
  fi
done

if [[ -z ${best_db} ]]; then
  echo "No AddressBook databases found in ${sources_dir}" >&2
  exit 1
fi

{
  cat <<EOF
-- AddressBook SQLite schema dump for local development fixtures.
-- Source: ${best_db}
-- Contact count: ${best_count}
-- Regenerate empty DB: fixtures/contacts/create-empty-db.sh

EOF
  sqlite3 "${best_db}" ".schema" | rg -v '^CREATE TABLE sqlite_(stat1|sequence)'
} >"${schema}"

echo "Wrote schema from ${best_db} (${best_count} contacts) to ${schema}"
