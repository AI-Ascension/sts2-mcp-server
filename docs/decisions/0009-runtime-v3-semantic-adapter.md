# ADR 0009: Runtime-v3 MCP adapter

- Status: Accepted as a source-level adapter profile
- Date: 2026-09-04

## Context

The model-facing surface needs semantic observations and current host-generated actions while MCP
remains a thin transport adapter.

## Decision

The Runtime-v3 profile advertises exactly six tools: `sts2.observe`, `sts2.legal_actions`,
`sts2.dispatch_action`, `sts2.wait_for_transition`, `sts2.reobserve`, and `sts2.recover`. Each
request carries bounded instance/session/lease/epoch/generation context and maps to one fixed
gateway route. Dispatch accepts one typed action, while timeout or disconnect becomes an explicit
unknown/recovery result and is never retried. Responses are projected through an exact allowlist
with profile digest, identity, generation, status, and witness checks; raw host or arbitrary-route
tools are not exposed.

Co-op was split into a separate unadmitted proposal, preserved on
review/mcp-coop-proposal-source-20260905. It exports no code or schema here. Shared-contract admission
requires at least two named actual serialized-contract consumers; source-only status does not waive
that gate.

## Evidence

Mapping, projection, catalog, and exact-six-tool tests are source-derived. Live MCP/gateway
transport, provider execution, host compatibility, and end-to-end settlement remain unverified.
