---
id: REC-0006
status: superseded
issues:
  - https://github.com/atahanyorganci/apple-connector/issues/50
  - https://github.com/atahanyorganci/apple-connector/issues/62
provenance:
  - https://github.com/atahanyorganci/apple-connector/pull/55
  - https://github.com/atahanyorganci/apple-connector/pull/65
supersedes: []
superseded-by: [REC-0014]
---

# The framework store is a process-wide singleton behind a mutex, with work dispatched to other threads.

## Question

`EKEventStore` and `CNContactStore` are Objective-C objects that are not `Send`/`Sync`. Async
handlers on a multi-threaded runtime needed to use them.

## Options

- **Singleton behind a mutex, dispatched to the main queue or a blocking pool, with a timeout
  around the call** (#50: "EventKit save/remove on main dispatch queue; bridge completion
  handlers to async via `block2` + oneshot channels (30s timeout)"; #62: "mutex-wrapped
  `CNContactStore`, `run_on_main`, 30s timeout").
- **A dedicated thread that owns the store.**

## Decision

The singleton. Both crates asserted `unsafe impl Send`/`Sync` for their store wrappers.

## Consequences

As implemented, `store.rs` held the mutex only long enough to turn `Retained::as_ptr` into a
`usize`, then rebuilt a reference on a blocking-pool thread. Concurrent requests touched one store
from several threads, and the timeout wrapped the join rather than the work, so a timed-out job
kept dereferencing a pointer the owner could drop (PR #141). Separate synchronous `ensure_*`
helpers called the framework from the caller's thread and blocked the runtime while a permission
prompt was on screen.

Superseded by [REC-0014](REC-0014-framework-worker.md) (#74, PR #141).

## Evidence

- #74 summary: "Remove pointer-through-`usize`, unsynchronized framework access, detached
  lifetime, and unsafe Send/Sync assumptions."
- PR #141, "The memory-safety bug (#74)".
