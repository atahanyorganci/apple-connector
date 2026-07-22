#!/usr/bin/env bash
set -euo pipefail

source_db="${1:-${HOME}/Library/Messages/chat.db}"
root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
schema="${root}/chat.schema.sql"

if [[ ! -f ${source_db} ]]; then
  echo "Messages database not found at ${source_db}" >&2
  exit 1
fi

{
  cat <<'EOF'
-- Messages.app chat.db schema dump for local development fixtures.
-- Source: ~/Library/Messages/chat.db
-- Dumped with: sqlite3 "$HOME/Library/Messages/chat.db" ".schema"
-- Excludes sqlite internal tables: sqlite_sequence, sqlite_stat1
-- Regenerate empty DB: fixtures/messages/create-empty-db.sh

EOF
  sqlite3 "${source_db}" ".schema" | rg -v '^CREATE TABLE sqlite_(stat1|sequence)'
} >"${schema}"

echo "Wrote schema to ${schema}"
