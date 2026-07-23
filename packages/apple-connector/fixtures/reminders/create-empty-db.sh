#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
schema="${root}/reminders.schema.sql"
database="${root}/reminders.db"

rm -f "${database}"
sqlite3 "${database}" <"${schema}"

echo "Created empty Reminders fixture at ${database}"
