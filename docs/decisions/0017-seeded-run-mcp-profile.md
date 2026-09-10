# ADR 0017: seeded-run-v1 MCP profile

- Status: Accepted for the `seeded-run-v1` source/component consumer
- Date: 2026-09-09
- Owner: `sts2-mcp-server`
- Contract: `sts2-protocol/seeded-run-v1`

## Context

The seeded-run consumer needs one explicit seed launch operation and a safe way to resolve an
uncertain launch. The protocol artifact keeps admission, authoritative host settlement, selected
native context, and reconciliation separate. MCP owns the tool catalog, bounded argument checks,
session mapping, fixed gateway routes, and envelope validation; the gateway and host retain lease,
lifecycle, readiness, and effect authority.

## Decision

`ToolCatalog::seeded_run_v1()` advertises exactly `start_seeded_run` and
`reconcile_seeded_run`. The executable selects it only with `STS2_RUNTIME_PROFILE=seeded-run-v1`.
The profile maps a start to `POST /v2/instances/{instance_id}/seeded-run` with the complete
`seeded-run-v1` request envelope. Reconciliation is bodyless `GET
/v2/instances/{instance_id}/seeded-operations/{operation_id}`; it reuses the stable operation ID
and never retries the seed mutation. Because the operation ID is placed in a path, MCP accepts
only the gateway's path-safe subset: bounded ASCII letters, digits, `_`, `.`, `:`, and `-`, with
no slash or `..` segment.

MCP session identity and gateway session identity remain separate. The supplied
`mcp_session_id` is checked against the bound MCP session and stays in correlation metadata and
`x-mcp-session-id`; the bound gateway session is placed in the envelope and `x-sts2-session-id`.
Both operations carry explicit instance, lease, and lease-epoch authority, and the runtime
adapter rejects a bodyless start route or a seeded request with a different schema digest.

Start requires a concrete selected context. MCP validates its exact fields, fixed standard
mode/character, bounded and ordered collections, compatibility and baseline identities, and the
content-addressed `selected_context.context_digest`; the start tool's `seed` is copied byte-for-byte
to native `requested_seed`, and MCP derives the native top-level `context_digest` from that selected
context. The digest is computed from the schema's declared field order, so JSON object ordering
cannot change the selected context identity. Settled results require fresh
native observation and a `run_started` witness. A timeout or unavailable start is returned as
`unknown` with the same operation identity and must be resolved through reconciliation.

The profile is additive. `runtime-v3-gameplay` retains its catalog, routes, session mapping, and
profile-scoped limits unchanged. This decision establishes MCP source/component behavior and
copied-artifact integrity; live gateway, host settlement, gameplay, deployment, and release
compatibility remain unverified here.

## Deterministic evidence

`crates/mcp-server/tests/seeded_run_mapping.rs` checks the exact two-tool catalog, golden request
and settled response mapping, bodyless reconciliation, unknown-after-timeout behavior, invalid
metadata rejection, separate MCP/gateway sessions for both routes, and the copied manifest,
schema, conformance case, golden vectors, and checksum inventory. Runtime binding tests cover the
fixed route and schema-digest fence. The normal locked workspace format, Clippy, test, and strict
repository-policy gates remain required.
