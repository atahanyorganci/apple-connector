#!/usr/bin/env bash
# Regenerate the offline SQLx query cache in packages/apple-connector/sqlx.
#
# Apple uses five unrelated SQLite schemas, so no single DATABASE_URL describes every query. The
# cache is rebuilt from scratch with one `sqlx prepare` pass per store's fixture: each pass saves
# the queries its fixture can describe, and every other store's queries fail to compile ("no such
# table"). Those failures are expected, so a pass is only required to show that the tool ran: it
# exits 0, or 1 because `cargo check` failed, and it saved at least one query.
#
# An offline build against the rebuilt cache then proves that every tolerated failure was a
# cross-store one: a real Rust error would fail that build too, and so would a query that cannot
# be described against its own store, because it has no cache entry. The committed cache is
# replaced only after that build passes, so a failed run leaves it untouched and a successful one
# leaves no stale entries.
#
# Run inside `nix develop`. The dev shell's `sqlx` binary is called directly rather than through
# `cargo sqlx`: unless $CARGO_HOME/bin is on PATH, cargo searches it for subcommands before PATH,
# so a stale `cargo-sqlx` installed there would shadow the dev shell's (REC-L-0003).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PKG="$ROOT/packages/apple-connector"
SQLX_DIR="$PKG/sqlx"
# `sqlx prepare --workspace` always writes to <workspace>/.sqlx and empties it before each pass.
PASS_DIR="$ROOT/.sqlx"
WORK="$ROOT/target/sqlx-prepare-all"
STAGING="$WORK/cache"

STORES=(messages reminders notes calendar contacts)

die() {
  echo "sqlx-prepare-all: $*" >&2
  exit 1
}

database_url() {
  case "$1" in
  messages) echo "sqlite:packages/apple-connector/fixtures/messages/chat.db" ;;
  reminders) echo "sqlite:packages/apple-connector/fixtures/reminders/reminders.db" ;;
  notes) echo "sqlite:packages/apple-connector/fixtures/notes/notes.db" ;;
  calendar) echo "sqlite:packages/apple-connector/fixtures/calendar/calendar.db" ;;
  contacts) echo "sqlite:packages/apple-connector/fixtures/contacts/contacts.abcddb" ;;
  esac
}

count_queries() {
  if [[ -d $1 ]]; then
    find "$1" -maxdepth 1 -name 'query-*.json' | wc -l | tr -d ' '
  else
    echo 0
  fi
}

# --- The tool -----------------------------------------------------------------------------------

SQLX="$(command -v sqlx)" || die "sqlx is not on PATH; run inside nix develop"
CARGO="$(command -v cargo)" || die "cargo is not on PATH; run inside nix develop"
export CARGO

tool_version="$("$SQLX" --version 2>&1)" || die "\`$SQLX --version\` failed: $tool_version"
tool_minor="$(sed -nE 's/^sqlx-cli ([0-9]+\.[0-9]+)\..*/\1/p' <<<"$tool_version")"
lock_minor="$(awk '/^name = "sqlx"$/ { getline; print; exit }' "$ROOT/Cargo.lock" |
  sed -nE 's/^version = "([0-9]+\.[0-9]+)\..*/\1/p')"
[[ -n $tool_minor && -n $lock_minor ]] || die "cannot compare \`$tool_version\` with the sqlx version in Cargo.lock"
[[ $tool_minor == "$lock_minor" ]] || die "\`$tool_version\` does not match sqlx $lock_minor in Cargo.lock"

echo "Using $tool_version ($SQLX)"

# --- One pass per store -------------------------------------------------------------------------

rm -rf "$WORK"
mkdir -p "$STAGING" "$PASS_DIR"
cd "$ROOT"

for store in "${STORES[@]}"; do
  url="$(database_url "$store")"
  log="$WORK/$store.log"
  bash "$PKG/fixtures/$store/create-empty-db.sh" >/dev/null

  # `sqlx prepare` empties this itself, but a tool that never ran must not be credited with files
  # an earlier, aborted run left behind.
  find "$PASS_DIR" -maxdepth 1 -name 'query-*.json' -delete

  status=0
  DATABASE_URL="$url" "$SQLX" prepare --workspace -- --package apple-connector --all-targets \
    >"$log" 2>&1 || status=$?

  # Exit 1 is expected only when the compiler ran and reported the other stores' queries; any
  # other failure means the tool itself did not do its job.
  if ((status != 0)) && ! { ((status == 1)) && grep -q 'cargo check` failed with status: exit status: 101' "$log"; }; then
    tail -n 20 "$log" >&2
    die "the $store pass did not run (exit $status); full log: $log"
  fi

  saved="$(count_queries "$PASS_DIR")"
  ((saved > 0)) || die "the $store pass saved no queries; full log: $log"

  # A query valid against several fixtures (`SELECT 1`, `sqlite_master` probes, the Core Data
  # `Z_PRIMARYKEY` lookups) is saved by each of their passes under the same name. Every pass must
  # describe it the same way, or the cache would depend on the order of the passes.
  for file in "$PASS_DIR"/query-*.json; do
    name="$(basename "$file")"
    if [[ -e "$STAGING/$name" ]] && ! cmp -s "$file" "$STAGING/$name"; then
      die "the $store pass describes $name differently from an earlier pass"
    fi
  done
  cp "$PASS_DIR"/query-*.json "$STAGING"/
  echo "==> $store: $saved queries"
done

# --- Verify: the rebuilt cache must be complete -------------------------------------------------

echo "Verifying with an offline build against the rebuilt cache..."
# Nothing but the staging cache may answer a macro: clear the scratch directories the macros fall
# back to, and touch the crate so its macros expand again under the new environment.
find "$PASS_DIR" "$PKG/.sqlx" -maxdepth 1 -name 'query-*.json' -delete 2>/dev/null || true
touch "$PKG/src/lib.rs"
if ! SQLX_OFFLINE=true SQLX_OFFLINE_DIR="$STAGING" "$CARGO" check --package apple-connector \
  --all-targets >"$WORK/verify.log" 2>&1; then
  grep -E '^error' -A 6 "$WORK/verify.log" | head -n 40 >&2
  if grep -q 'there is no cached data for this query' "$WORK/verify.log"; then
    echo "A query with no cached data could not be described against its own store; its database" >&2
    echo "error is in that store's pass log in $WORK." >&2
  fi
  die "the rebuilt cache does not compile offline; full log: $WORK/verify.log"
fi

# --- Replace the committed cache ----------------------------------------------------------------

before="$(count_queries "$SQLX_DIR")"
mkdir -p "$SQLX_DIR"
find "$SQLX_DIR" -maxdepth 1 -name 'query-*.json' -delete
cp "$STAGING"/query-*.json "$SQLX_DIR"/
after="$(count_queries "$SQLX_DIR")"

echo "Updated $SQLX_DIR: $after queries (was $before)"
