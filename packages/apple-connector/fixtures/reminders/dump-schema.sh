#!/usr/bin/env bash
set -euo pipefail

stores_dir="${1:-${HOME}/Library/Group Containers/group.com.apple.reminders/Container_v1/Stores}"
root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
schema="${root}/reminders.schema.sql"

if [[ ! -d ${stores_dir} ]]; then
  echo "Reminders stores directory not found at ${stores_dir}" >&2
  exit 1
fi

best_db=""
best_count=-1
best_mtime=0

for db in "${stores_dir}"/Data-*.sqlite; do
  [[ -f ${db} ]] || continue
  count="$(sqlite3 "${db}" "SELECT COUNT(*) FROM ZREMCDREMINDER WHERE ZMARKEDFORDELETION = 0;" 2>/dev/null || echo 0)"
  mtime="$(stat -f '%m' "${db}" 2>/dev/null || stat -c '%Y' "${db}")"
  if ((count > best_count)) || { ((count == best_count)) && ((mtime > best_mtime)); }; then
    best_db="${db}"
    best_count="${count}"
    best_mtime="${mtime}"
  fi
done

if [[ -z ${best_db} ]]; then
  echo "No Reminders store databases found in ${stores_dir}" >&2
  exit 1
fi

{
  cat <<EOF
-- Reminders.app SQLite schema dump for local development fixtures.
-- Source: ${best_db}
-- Reminder count: ${best_count}
-- Regenerate empty DB: fixtures/reminders/create-empty-db.sh

EOF
  sqlite3 "${best_db}" ".schema" | rg -v '^CREATE TABLE sqlite_(stat1|sequence)'
} >"${schema}"

echo "Wrote schema from ${best_db} (${best_count} reminders) to ${schema}"
