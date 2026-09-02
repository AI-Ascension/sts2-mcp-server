# ADR 0007: `runtime-v1` MCP process adapter

- Status: Accepted for the bounded component slice; downstream host compatibility remains unverified
- Date: 2026-09-02

## Context

The existing MCP seam proves in-memory mapping only. The next sprint needs a real MCP process and a
real gateway connection while retaining MCP's thin-adapter boundary. Direct game access or an
unrestricted proxy would bypass gateway lifecycle and authorization.

## Decision

Add a runtime binary that reads bounded newline-delimited JSON-RPC frames from stdin and writes one
response per line to stdout. Its `runtime-v1-mcp` catalog contains exactly `get_state` and
`submit_action`. Calls map only to the fixed gateway state/action paths. The adapter injects the
configured bearer token and instance/caller/session/lease/epoch/correlation headers, rewrites the
runtime envelope to the configured lease identity, and bounds both request and response bytes.

The only admitted action is `show_runtime_probe`. Structured 409 action responses are preserved so
the stable `sts2.game-mod/stale_generation` result reaches the harness; non-structured gateway
errors become sanitized MCP tool errors. Runtime response metadata and allowlisted observation,
action, status, error, and effect-witness fields are validated before projection.

The MCP process does not contact the game directly, issue leases, launch processes, call providers,
or interpret game rules. `sts2-gateway` owns authentication and fencing; `sts2-game-mod` owns host
authority; `sts2-harness` owns coordination.

## Consequences and evidence

The process, mapping, copied artifact, and deterministic tests are `confirmed` at source/build
level. A real gateway/MCP/synthetic-downstream lane can confirm component network interoperation.
The managed host callback, STS2 runtime effect, disposable profile, and gameplay mutation remain
`unverified` until separately authorized.
