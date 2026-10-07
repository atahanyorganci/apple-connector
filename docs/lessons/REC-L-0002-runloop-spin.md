---
id: REC-L-0002
status: active
observed-on: "macOS 26.5.2; NSRunLoop on a thread with no input sources (EventKit and Contacts permission prompts)"
graduated-to: ""
provenance:
  - https://github.com/atahanyorganci/apple-connector/issues/75
  - https://github.com/atahanyorganci/apple-connector/pull/141
---

# Pumping `NSRunLoop.currentRunLoop` on a thread with no input sources returns immediately, so a poll loop around it spins.

## What happened

While a permission prompt was on screen, the authorization wait polled `try_recv` and pumped
`NSRunLoop::currentRunLoop` between polls. The thread used a full CPU core for up to two minutes.

## Why

A run loop with no input sources or timers has nothing to wait on; running it returns at once. The
completion handler for an EventKit or Contacts access request is not delivered through that run
loop, so pumping it does nothing but return.

## Rule

Wait for a framework completion handler with a blocking receive and a timeout
(`recv_timeout`), never with a loop that pumps a run loop the thread does not own sources on.

## Why it is not a test yet

The spin only shows while a real TCC prompt is pending, which a test cannot produce. The fix is in
`packages/apple-eventkit/src/auth.rs` and `packages/apple-contacts/src/auth.rs`
(`wait_for_auth` uses `recv_timeout(PROMPT_TIMEOUT)`); a lint against `NSRunLoop` in those crates
would make it enforceable.
