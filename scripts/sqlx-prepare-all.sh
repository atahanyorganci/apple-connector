#!/usr/bin/env bash
# Regenerate offline SQLx metadata by preparing against each domain fixture in turn.
#
# Apple uses five incompatible SQLite schemas, so a single DATABASE_URL cannot
# describe every query. Each prepare pass validates newly cached queries against
# one fixture; previously cached queries stay available via SQLX_OFFLINE=true.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PKG="$ROOT/packages/apple-connector"
SQLX_DIR="$PKG/sqlx"
WORKSPACE_SQLX="$ROOT/.sqlx"

declare -a PREPARE_TARGETS=(
  "packages/apple-connector/fixtures/messages/create-empty-db.sh|sqlite:packages/apple-connector/fixtures/messages/chat.db"
  "packages/apple-connector/fixtures/reminders/create-empty-db.sh|sqlite:packages/apple-connector/fixtures/reminders/reminders.db"
  "packages/apple-connector/fixtures/notes/create-empty-db.sh|sqlite:packages/apple-connector/fixtures/notes/notes.db"
  "packages/apple-connector/fixtures/calendar/create-empty-db.sh|sqlite:packages/apple-connector/fixtures/calendar/calendar.db"
  "packages/apple-connector/fixtures/contacts/create-empty-db.sh|sqlite:packages/apple-connector/fixtures/contacts/contacts.abcddb"
)

mkdir -p "$SQLX_DIR" "$WORKSPACE_SQLX"
rsync -a "$SQLX_DIR/" "$WORKSPACE_SQLX/"

echo "Preparing SQLx offline metadata across ${#PREPARE_TARGETS[@]} fixture schemas..."

for entry in "${PREPARE_TARGETS[@]}"; do
  IFS='|' read -r script database_url <<< "$entry"
  echo "==> $database_url"
  bash "$ROOT/$script"
  (
    cd "$ROOT"
    export SQLX_OFFLINE=true
    export SQLX_OFFLINE_DIR="$WORKSPACE_SQLX"
    export DATABASE_URL="$database_url"
    cargo sqlx prepare --workspace -- --package apple-connector --all-targets || true
  )
  rsync -a "$WORKSPACE_SQLX/" "$SQLX_DIR/"
done

echo "Verifying merged offline metadata..."
(
  cd "$ROOT"
  export SQLX_OFFLINE=true
  export SQLX_OFFLINE_DIR="$SQLX_DIR"
  cargo sqlx prepare --workspace --check -- --package apple-connector --all-targets
)

echo "Updated offline metadata in $SQLX_DIR"
