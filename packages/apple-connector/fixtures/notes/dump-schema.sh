#!/usr/bin/env bash
set -euo pipefail

notes_db="${1:-${HOME}/Library/Group Containers/group.com.apple.notes/NoteStore.sqlite}"
root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
schema="${root}/notes.schema.sql"

if [[ ! -f ${notes_db} ]]; then
  echo "Notes store database not found at ${notes_db}" >&2
  exit 1
fi

note_count="$(sqlite3 "${notes_db}" "SELECT COUNT(*) FROM ZICCLOUDSYNCINGOBJECT WHERE Z_ENT = 12 AND ZMARKEDFORDELETION = 0;" 2>/dev/null || echo 0)"

{
  cat <<EOF
-- Notes.app SQLite schema dump for local development fixtures.
-- Source: ${notes_db}
-- Note count: ${note_count}
-- Regenerate empty DB: fixtures/notes/create-empty-db.sh

EOF
  sqlite3 "${notes_db}" ".schema" | rg -v '^CREATE TABLE sqlite_(stat1|sequence)'
} >"${schema}"

echo "Wrote schema from ${notes_db} (${note_count} notes) to ${schema}"
