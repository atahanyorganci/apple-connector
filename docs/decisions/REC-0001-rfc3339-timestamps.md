---
id: REC-0001
status: superseded
issues:
  - https://github.com/atahanyorganci/apple-connector/issues/14
provenance:
  - https://github.com/atahanyorganci/apple-connector/pull/15
supersedes: []
superseded-by: [REC-0003]
---

# Timestamps cross the HTTP boundary as RFC 3339 strings.

## Question

The first API (Messages, #8) had to pick one wire format for instants in responses and query
parameters. Apple stores them as nanoseconds (Messages) or seconds (Core Data) since 2001-01-01.

## Options

- **RFC 3339 strings**: human-readable, self-describing, what most JSON APIs use.
- **Unix seconds as integers**: compact, no parsing, no timezone ambiguity.

## Decision

RFC 3339 strings, for readability while the contract was being designed (#14: "Define tagged
message content, RFC 3339 timestamps").

## Consequences

Every DTO carried `Option<String>` timestamps and every query bound was parsed from RFC 3339.

Superseded by [REC-0003](REC-0003-unix-response-timestamps.md) in PR #29, one day later, when
Reminders arrived and the strings had to be produced from two different Apple epochs.

## Evidence

- #14 scope list; PR #15 shipped it.
- PR #29 "Breaking changes (#28)": responses moved to integer Unix seconds.
