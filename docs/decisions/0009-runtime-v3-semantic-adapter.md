# ADR 0009: Runtime-v3 and co-op MCP adapter

- Status: Accepted as a source-level adapter profile
- Date: 2026-09-04

## Context

The model-facing surface needs semantic observations and current host-generated actions while MCP
remains a thin transport adapter. Co-op synchronization is read-only coordination metadata; it
cannot authorize a mutation during peer disagreement or an unknown shared effect.

## Decision

The Runtime-v3 profile advertises exactly six tools: `sts2.observe`, `sts2.legal_actions`,
`sts2.dispatch_action`, `sts2.wait_for_transition`, `sts2.reobserve`, and `sts2.recover`. Each
request carries bounded instance/session/lease/epoch/generation context and maps to one fixed
gateway route. Dispatch accepts one typed action, while timeout or disconnect becomes an explicit
unknown/recovery result and is never retried. Responses are projected through an exact allowlist
with profile digest, identity, generation, status, and witness checks; raw host or arbitrary-route
tools are not exposed.

The additive co-op mapping exposes synchronization only through a bounded read path and rejects
unknown peers, duplicate/missing identities, unsafe values, and unsupported fields before gateway
forwarding. It does not duplicate gateway lease or host legality authority.

## Evidence

Mapping, projection, catalog, co-op, and exact-six-tool tests are source-derived. Live MCP/gateway
transport, provider execution, host compatibility, and end-to-end settlement remain unverified.
