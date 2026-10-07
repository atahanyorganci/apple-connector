---
name: worklog
description: Create or update the WORKLOG.md on a task branch. Where I am, what I tried, what is next, open threads, which checks pass. Use at the start of a session, before ending one, whenever direction changes, or when the user asks for a worklog update.
---

# /worklog

`WORKLOG.md` records the state of a task branch: a **state header** rewritten each session, and an **append-only log** of what happened, including steps that were backtracked or reverted.

## Rules

- Lives at `${repoRoot}/WORKLOG.md`. It is git-ignored and never committed.
- The state header (everything above `## Log`) reflects the branch now; rewrite it freely.
- The `## Log` section is append-only. Never edit or delete past entries; a reverted step gets a new entry saying so and why.
- Link the GitHub issues the branch works on. Issues track progress only; decisions, lessons, and specs are the authoritative record. For multiphase work, link the parent issue and the child issue for the current phase.
- Reference decisions (`docs/decisions/…`), lessons (`docs/lessons/…`), specs, and tests by path.
- No personal data from live stores (messages, contacts, notes, events), not even in a git-ignored file. Describe shapes, not contents.

## Steps

1. If the file exists, read it fully before changing anything. If it doesn't, create it from the template, using `git branch --show-current` for the branch name.
2. Rewrite the state header: plan, open threads, decisions, checks, next up.
3. Append a dated entry to `## Log`: what was done, what was tried and abandoned, and the references (commits, decisions, lessons, tests) it produced.
4. Before the branch is merged:
   - Every entry under `## Decisions` has reached a decision record (`decision-record` skill) or the PR description.
   - Every surprise worth keeping has become a lesson (`lesson` skill).
   - Every check in `## Checks` that applies is ticked.
   - Commits follow conventional commits, and the PR links its issues.

## Template

```markdown
# WORKLOG: <branch>

Issues: #<parent> → #<current phase>

_What this branch does, in 2–3 sentences._

## Plan

- [ ] _Step, in order; note what it depends on._

## Open threads

- _Unanswered question or blocker, and who or what can resolve it._

## Decisions

- _Decision taken on this branch → destination (decision record id or "PR description")._

## Checks

_Tick when run and passing on the current HEAD; strike through those that don't apply._

- [ ] `cargo test --workspace --all-targets`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `nix fmt`
- [ ] `nix flake check --no-write-lock-file`
- [ ] Ignored live tests (`eventkit_integration`, `contacts_integration`, `integration`); TCC grants present: _Full Disk Access / Reminders / Calendars / Contacts_
- [ ] `bash scripts/sqlx-prepare-all.sh` and `packages/apple-connector/sqlx/` committed (if queries changed)
- [ ] `cargo run -p apple-connector --bin export-openapi docs/openapi.json` (if handlers or schemas changed)
- [ ] `pnpm check` (Raycast format, lint, typecheck, and generated-client staleness; root formatting)

## Next up

_The very next action, concrete enough to start without rereading the log._

## Log

### YYYY-MM-DD

- _What was done or tried, the result, and references._
```
