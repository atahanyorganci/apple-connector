---
name: lesson
description: Lesson is a post-mortem record of unexpected behavior, a bug, or undocumented dependency behavior (most often Apple SQLite schemas or EventKit/Contacts framework quirks): knowledge that can only be learned by experience. After solving a difficult problem, write a lesson to capture the knowledge.
---

# /lesson

## Before writing

Would a reasonable, experienced developer have expected this behavior? If yes, it is not a lesson. Lessons are not part of the regular development process; they are part of the post-mortem, where we work out why a problem happened and how to prevent it.

The typical lesson in this repo is undocumented Apple behaviour, for example:

- `OccurrenceCache` first rows leave `occurrence_start_date` NULL; the start lives in `occurrence_date` (#152).
- A contact can have a NULL `ZCONTAINER`, implying the default container (#153).
- EventKit's `calendarItemIdentifier` lookup is case-sensitive, and `external_id` is the server's resource id, not the iCalendar UID (#151).

## Steps

1. Id: `<PREFIX>-L-0000`, using the prefix from the scope table in the `decision-record` skill (`REC` for the whole repo). Take the highest number in the scope's `docs/lessons/` folder plus one, zero-padded to four. Ids are never reused.
2. File `ID-slug.md` in `${repoRoot}/packages/<crate>/docs/lessons/`, or `${repoRoot}/docs/lessons/` at root. The slug is the subject in two or three words (`occurrence-start-null`, `eventkit-id-case`, `implied-container`); the rule is the title inside.
3. Fill `observed-on`: the macOS version, and the store and table/column or framework class where the behaviour shows up.
4. Fill `provenance`: links to the issue, PR, and commits. Prefer the reproducing `test:` commit and the `fix:` commit as a pair.
5. Graduate it. Encode the rule in the cheapest place that will catch a regression:
   1. **Fixture + offline test**: add a row to `packages/apple-connector/fixtures/<domain>/seed.sql` that mirrors what macOS writes, and a test that fails without the fix. Explain why the row exists in that fixture's `README.md`.
   2. **Live integration test**: for behaviour only the real framework shows, an `#[ignore]` test in `packages/apple-connector/tests/*_integration.rs` that uses only ids the API hands out and cleans up even on failure.
   3. **Fuzz**: for parser crashes or hangs, add the input to `fuzz/seeds/<target>/` (or a new target in `fuzz/fuzz_targets/`).
   4. **Build-time rule**: a `clippy.toml` `disallowed-methods` entry, a `cargo deny` rule, or a `compile_error!`.
   5. **Spec line**: a guarantee in the crate's `docs/SPEC.md` that names the test enforcing it.

**Personal data never goes in a lesson.** Describe the shape of the data (`ZCONTAINER is NULL`), never real names, numbers, or message text from live stores.

## Template

```markdown
---
id: REC-L-0000
status: active # active | graduated | archived
observed-on: "" # macOS version; store and table/column or framework class
graduated-to: "" # path to the test, fixture, fuzz seed, rule, or spec line
provenance: [] # links: the issue, PR, and commits it came from
---

# <one sentence: the thing we now know>

## What happened

<the failure, concretely: request or input, expected, actual>

## Why

<the cause, once known: what Apple's store or framework actually does>

## Rule

<the sentence a test or a spec line will enforce>

## Why it is not a test yet

<required while it is not; on graduation this section becomes `## Graduated`, naming the test and what it asserts>
```
