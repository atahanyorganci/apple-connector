---
id: REC-0011
status: proposed
issues:
  - https://github.com/atahanyorganci/apple-connector/issues/107
  - https://github.com/atahanyorganci/apple-connector/issues/129
provenance:
  - https://github.com/atahanyorganci/apple-connector/pull/139
  - https://github.com/atahanyorganci/apple-connector/pull/141
supersedes: []
superseded-by: []
---

# Before 1.0, breaking changes ship without deprecation shims, and each PR lists the ones it makes.

## Question

The correctness epics (#107, #129, #83) found contract mistakes: dead parameters, coarse error
codes, silently ignored fields. Fixing them breaks clients.

## Options

- **Deprecate first**: keep old forms working beside new ones for a period.
- **Break now, document in the PR**: no aliases, the old form is removed.

## Decision

Break now. #107's guiding principles: "No deprecation shims — remove dead params, drop RFC 3339
query aliases, delete coarse error codes"; "Prefer reject over coerce"; "Prefer fail-fast";
"OpenAPI + tests are the contract". A PR with breaking changes has a "Breaking changes" section.

## Consequences

- `docs/errors.md` and `docs/openapi.json` state that the API is unstable before 1.0.
- Clients pin to a commit. The only in-repo client, the Raycast extension, is regenerated from the
  contract ([REC-0002](REC-0002-generated-artifacts.md)).
- At 1.0 this record is superseded by a compatibility policy.

## Evidence

- PR #29 "Breaking changes (#28)", PR #141 "Breaking API changes (pre-1.0)".
- `docs/errors.md`: "Pre-1.0: codes and HTTP mappings may change without deprecation shims."
