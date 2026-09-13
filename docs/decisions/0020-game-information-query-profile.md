# ADR 0020: game-information-query-v1 MCP profile

- Status: Accepted for the bounded MCP/fake-gateway seam; gateway and game-mod integration remains
  unverified
- Date: 2026-09-13
- Owner: `sts2-mcp-server`
- Protocol consumer: `sts2-protocol/game-information-query-v1`

## Context

Issue #51 needs agent-visible content definitions, live entity fields, and availability without
exposing internal HTTP, host access, shell commands, or mutation. Protocol PR #49 is merged into
`sts2-protocol/main` at `34f68b182c09472c3a0573ff478e17e6ed53c91f`; this profile pins that main
digest in `GAME_INFORMATION_PROTOCOL_SOURCE_COMMIT` and the schema digest
`376845b0c86b4afcd2c79ffba753eb7e7e416f5410da26b4dae970cfee2221d9`. The exact schema and
goldens are copied under `schemas/game-information-query-v1.schema.json`,
`protocol-artifact/game-information-query-v1/`, and the conformance fixture tree. Their
`SHA256SUMS` inventory is verified before the executable advertises the profile.

Gateway issue #52 is also not merged. MCP therefore owns a typed route port and a deterministic
fake, with the following expected route contract in one place for reconciliation:

| MCP operation | Method and fixed path | Body |
| --- | --- | --- |
| capabilities | `GET /v1/instances/{instance_id}/game-information/capabilities` | none |
| list/search/get/detail/availability | `POST /v1/instances/{instance_id}/game-information/query` | the pinned `query_request` envelope, including the exact query and provenance |

The adapter adds explicit MCP-session and gateway-authority headers, never selects or provisions an
instance, and never forwards an arbitrary path.

## Decision

`ToolCatalog::game_information_query_v1()` has revision
`game-information-query-v1-mcp` and advertises exactly:

* `sts2.game_information_capabilities`
* `sts2.game_information_list`
* `sts2.game_information_search`
* `sts2.game_information_get`
* `sts2.game_information_detail`
* `sts2.game_information_availability`

All six tools are read-only, non-destructive, idempotent reads with a closed input schema
(`additionalProperties: false`). Definitions use content-manifest/entity-kind/namespaced-ID
references; live instance IDs additionally carry run/epoch/entity identity. Descriptions make that
distinction, detail projections, page/text byte bounds, and stale-snapshot/cursor re-query behavior
explicit. Returned game text remains data in a structured field and is never interpreted as a tool
instruction.

Static queries bind content manifest, locale, and visibility scope. Live detail and live
availability additionally require a matching instance reference, snapshot reference, generation,
and parent observation. Cursors are bounded opaque values; their binding is validated
without changing the cursor or silently returning a mixed page. The profile advertises only the
five query kinds and producer entity/projection/detail values in the accepted contract. No
feature-owner specialized producer query is currently registered; unsupported kinds and fields are
not advertised or synthesized.

The MCP boundary validates exact envelopes, provenance, schema digest, correlation, references,
field kinds/units, per-field availability, deterministic ordering, canonical byte accounting,
pagination, generation, and the 262,144-byte message ceiling. Page items, item/page/text bytes,
identities, cursors, and field groups retain the protocol bounds. Invalid input is an MCP invalid-params
response; denied, unsupported, missing, stale, size, and transport failures remain distinguishable
through typed protocol error codes or sanitized gateway error codes. A timeout/disconnect does not
retry or claim settlement.

The executable selects the profile only with
`STS2_RUNTIME_PROFILE=game-information-query-v1`, verifies the copied artifact, uses a 256 KiB
MCP/gateway response budget, and rejects foreign or missing authority before opening a connection.
Existing catalogs, routes, annotations, and legacy byte limits are unchanged.

## Deterministic evidence and external gate

`tests/game_information_query.rs` proves descriptor closure, annotations, artifact checksums,
capabilities/list/search/get/detail/availability mapping, static pagination, live detail, stale
errors, foreign/oversized/unknown inputs, read-only response validation, and transport errors with
a fake gateway. Executable binding/profile tests prove the fixed route and authority fence; a
loopback HTTP test crosses the capabilities route.

These checks establish source/component behavior only. Gateway #52 must supply the admitted
authenticated route and limits, and `sts2-game-mod` must supply authoritative content/live data and
snapshot freshness before integrated acceptance. The harness must separately verify agent-facing
tool consumption. No game is launched and no host/save/profile is mutated by this profile.
