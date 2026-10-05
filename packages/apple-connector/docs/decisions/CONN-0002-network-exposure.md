---
id: CONN-0002
status: proposed
issues:
  - https://github.com/atahanyorganci/apple-connector/issues/8
  - https://github.com/atahanyorganci/apple-connector/issues/9
  - https://github.com/atahanyorganci/apple-connector/issues/12
provenance:
  - https://github.com/atahanyorganci/apple-connector/pull/15
supersedes: []
superseded-by: []
---

# The server binds loopback by default, has no authentication, TLS, or CORS, and leaves exposure to a reverse proxy.

## Question

The API serves someone's messages, contacts, and calendar. It has to be reachable by local
clients, and possibly by other machines, without the project owning an auth system.

## Options

- **Built-in auth and TLS**: tokens, certificates, rotation — a product of its own.
- **Loopback only, no way out**: blocks reverse-proxy deployments.
- **Loopback by default; an explicit all-interfaces bind for deployments behind a proxy or
  firewall that provides TLS and auth.**

## Decision

The third. `--address` defaults to `127.0.0.1`, `--port` to `3000`. The address must be an IPv4
loopback, `::1`, or `0.0.0.0`; any other address is refused at argument parsing. Binding
`0.0.0.0` logs a warning that the server is unauthenticated. There is no CORS layer, so browsers
on other origins cannot read responses.

## Consequences

- Any local process can read everything the server can. That is the threat model: the server
  runs with the user's Full Disk Access.
- Every response carries `X-Content-Type-Options: nosniff`, `X-Frame-Options: DENY`,
  `Referrer-Policy: no-referrer`, and `Cache-Control: no-store`.

## Evidence

- `packages/apple-connector/src/cli.rs`: `validate_address`, `parse_port` (rejects `0`),
  `warns_about_public_binding`; tests `defaults_to_loopback_port_and_home_database` and friends.
- `packages/apple-connector/src/api/middleware.rs`: `security_headers`; test
  `security_headers_are_applied` asserts no `access-control-allow-origin`.
