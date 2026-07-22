#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
schema="${root}/chat.schema.sql"
database="${root}/chat.db"

rm -f "${database}"
sqlite3 "${database}" <"${schema}"

echo "Created empty Messages fixture at ${database}"
