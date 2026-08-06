#!/usr/bin/env bash
set -euo pipefail

db="${1:-${HOME}/Library/Group Containers/group.com.apple.calendar/Calendar.sqlitedb}"
root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
schema="${root}/calendar.schema.sql"

if [[ ! -f ${db} ]]; then
  echo "Calendar database not found at ${db}" >&2
  exit 1
fi

event_count="$(sqlite3 "${db}" "SELECT COUNT(*) FROM CalendarItem WHERE title IS NOT NULL;" 2>/dev/null || echo 0)"

{
  cat <<EOF
-- Calendar.app SQLite schema dump for local development fixtures.
-- Source: ${db}
-- Event count: ${event_count}
-- Regenerate empty DB: fixtures/calendar/create-empty-db.sh

EOF
  sqlite3 "${db}" ".schema" | rg -v '^CREATE TABLE sqlite_(stat1|sequence)'
} >"${schema}"

echo "Wrote schema from ${db} (${event_count} events) to ${schema}"
