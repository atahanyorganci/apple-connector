---
id: REC-0003
status: superseded
issues:
  - https://github.com/atahanyorganci/apple-connector/issues/28
provenance:
  - https://github.com/atahanyorganci/apple-connector/pull/29
supersedes: [REC-0001]
superseded-by: [REC-0010]
---

# Responses carry Unix seconds; query bounds accept RFC 3339 or Unix seconds.

## Question

Reminders stores Core Data seconds since 2001, Messages stores nanoseconds since 2001. With two
domains, RFC 3339 response strings ([REC-0001](REC-0001-rfc3339-timestamps.md)) had to be
produced from two epochs in many places, and clients had to parse them back.

## Options

- **Keep RFC 3339 everywhere.**
- **Unix seconds in responses, RFC 3339 still accepted in queries**: breaking for readers, not
  for existing query strings.
- **Unix seconds everywhere.**

## Decision

Unix seconds as JSON integers in every response (`UnixTimestamp`, `apple_types/timestamp.rs`);
query bounds (`before`, `after`, `due_before`, `due_after`) keep accepting ISO-8601 alongside
integers to avoid breaking query strings in the same change.

## Consequences

Two parsers per timestamp query parameter, with domains drifting in which forms they accepted.

Superseded by [REC-0010](REC-0010-unix-seconds.md) (#104, PR #139), which dropped the RFC 3339
query forms.

## Evidence

- #28: "Query params: accept ISO-8601 **or** Unix int; normalize internally."
- PR #29: "Query bounds (`before`, `after`, `due_before`, `due_after`) still accept ISO-8601 input."
