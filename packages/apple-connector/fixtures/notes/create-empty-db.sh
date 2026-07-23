#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
schema="${root}/notes.schema.sql"
database="${root}/notes.db"

rm -f "${database}"
sqlite3 "${database}" <"${schema}"

echo "Created empty Notes fixture at ${database}"
