#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
rm -f calendar.db
sqlite3 calendar.db < calendar.schema.sql
