---
name: decision-record
description: Write a decision record for the whole repo at `${repoRoot}/docs/decisions/` (prefix REC) or for a single crate at `packages/${crate}/docs/decisions/` using the prefix from the scope table. Use when a technical decision with multiple options, consequences, reasoning and evidence needs to be recorded.
---

# /decision-record

## Where it goes

- Everyone must know it to write correct code anywhere in the repo: `docs/decisions/`, prefix `REC`.
- It only constrains one crate: `packages/<crate>/docs/decisions/`, prefix from the table.

This table is the single source of prefixes; the `lesson` skill uses it too.

| Scope                  | Path                             | Prefix    |
| ---------------------- | -------------------------------- | --------- |
| Whole repo             | `docs/`                          | `REC`     |
| `apple-connector`      | `packages/apple-connector/`      | `CONN`    |
| `apple-eventkit`       | `packages/apple-eventkit/`       | `EK`      |
| `apple-contacts`       | `packages/apple-contacts/`       | `CN`      |
| `apple-notes-protobuf` | `packages/apple-notes-protobuf/` | `NOTES`   |
| `apple-typedstream`    | `packages/apple-typedstream/`    | `TSTREAM` |
| `serde-vcard`          | `packages/serde-vcard/`          | `VCARD`   |
| `serde-carddav`        | `packages/serde-carddav/`        | `CARDDAV` |
| `serde-caldav`         | `packages/serde-caldav/`         | `CALDAV`  |
| `serde-icalendar`      | `packages/serde-icalendar/`      | `ICAL`    |
| `raycast-extension`    | `packages/raycast-extension/`    | `RAY`     |

A new crate gets a new row before its first record. Retired prefixes are never reused; when a crate is folded into another, its records are renumbered under the new prefix, keeping their slugs, with the rename noted at the end of each record.

## Steps

1. Next id: the highest number in that folder plus one, zero-padded to four. Ids are never reused.
2. File name `ID-slug.md`, e.g. `REC-0001-hybrid-read-write.md`. The slug is the subject in two or three words (`worker-thread`, `event-identifiers`), never the sentence; the sentence is the title inside. Lowercase, hyphens. The file never moves after this.
3. Set the status:
   - `open`: not yet decided.
   - `proposed`: tentatively implemented, or to be implemented later.
   - `accepted`: implemented and approved by a human review (see the gate below).
   - `rejected`: not implemented, or its implementation was reverted.
   - `superseded`: replaced by a newer record and no longer relevant.
4. Link the issue. The record is authoritative; the issue only tracks progress. Put the issue in `issues`, and comment on the issue with a link to the record.
5. Never edit an `accepted` record's body. Write a successor and cross-link `supersedes` and `superseded-by`.
6. A promised follow-up decision gets its own `open` record, linked from the Consequences section. Ordinary implementation TODOs belong in issues, not records.

## The gate for `accepted`

Only a human review can allow a decision to be accepted.

## Evidence is the observations and the numbers

Most decisions here rest on how Apple's stores and frameworks behave, not on timings. The Evidence section carries what the decision rests on, in full:

- **Observed behaviour**: the store and table/column (`OccurrenceCache.occurrence_start_date`) or framework API (`EKEventStore calendarItemWithIdentifier:`), what was observed, and the **macOS version** it was observed on. Apple changes schemas and framework behaviour between releases; an observation without its OS version is comparable to nothing.
- **Reproductions**: links to the fixture rows (`packages/apple-connector/fixtures/<domain>/seed.sql`) and to the tests, including `--ignored` live integration tests, that show the behaviour.
- **Numbers**: tables of rows measured, before and after. Every timing table names the machine class (chip and macOS version, e.g. `M3 Max, macOS 26`), never a hostname or a path.
- **Refused options** carry their evidence too, because a rejection without evidence is not a rejection.

Command lines stay out; the provenance links keep them reachable.

**Personal data never goes in a record.** Messages, Contacts, Notes, Calendar and Reminders are the user's own data. Describe the shape (`ZCONTAINER is NULL`, `a 2-member group chat`) or use fixture data; never paste rows, names, numbers, or message text from live stores.

## Template

```md
---
id: REC-0000
status: open # open | proposed | accepted | rejected | superseded
issues: [] # links: the GitHub issues this decides
provenance: [] # links: the commits, PRs, or documents this was written from
supersedes: []
superseded-by: []
---

# _One sentence: the decision._

## Question

_What had to be decided, and what forces deciding it now?_

## Options

- **Option A**: _Description of the first option._
- **Option B**: _Description of the second option._

## Decision

_Which option, and why the forcing function points at it._

## Consequences

_What the rest of the repo now has to do; what to watch for. Link follow-up `open` records here._

## Evidence

_Observed behaviour (with macOS version), fixtures, tests, numbers (with machine class). Appending later is fine._
```
