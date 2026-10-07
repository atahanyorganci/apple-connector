---
id: REC-0014
status: proposed
issues:
  - https://github.com/atahanyorganci/apple-connector/issues/74
  - https://github.com/atahanyorganci/apple-connector/issues/82
provenance:
  - https://github.com/atahanyorganci/apple-connector/pull/141
supersedes: [REC-0006]
superseded-by: []
---

# Each Objective-C store is built on, owned by, and only touched from one dedicated thread.

## Question

[REC-0006](REC-0006-shared-store-access.md) shared one store across threads through a raw
pointer, with timeouts that could outlive the store. The fix had to make framework access serial
and the store's lifetime unconditional.

## Options

- **Re-justify the `unsafe impl Send/Sync`** with a tighter lock.
- **Run everything on the main queue**: the server has no main run loop to serve it.
- **A dedicated owner thread** that receives jobs over a channel.

## Decision

A dedicated owner thread (`worker.rs`, identical in both crates). `Worker::spawn` builds the
resource on its thread; `Worker::run` sends a boxed closure that borrows it for one job and
returns a `Send` result over a oneshot channel. The single-consumer channel makes framework calls
strictly serial; a timeout abandons the result, not the borrow, so the store can never dangle.
Both `unsafe impl Send/Sync` blocks are gone. Permission prompts are jobs too.

## Consequences

- New framework work is a job submitted to the store; never `spawn_blocking`, never a
  `Retained<_>` crossing threads (AGENTS.md).
- One slow framework call delays every queued call behind it. Budgets: 30 s per operation,
  400 s for a job that may show a permission prompt (120 s prompt timeout inside it).
- Dropping the handle closes the channel; the thread ends and releases the store on its own
  thread. It is not joined, so dropping from async code never blocks the runtime.

## Evidence

- `packages/apple-eventkit/src/worker.rs` tests: `jobs_never_overlap` drives the worker with a
  `RefCell`, which is neither `Send` nor `Sync` — that it compiles proves the resource never
  leaves the thread.
- `packages/apple-eventkit/src/store.rs`: `OPERATION_TIMEOUT` 30 s; `auth.rs`:
  `PROMPT_JOB_BUDGET` 400 s, `PROMPT_TIMEOUT` 120 s.
- `diff packages/apple-eventkit/src/worker.rs packages/apple-contacts/src/worker.rs` is empty.
