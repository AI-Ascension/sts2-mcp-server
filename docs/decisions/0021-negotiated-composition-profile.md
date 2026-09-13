# ADR 0021: negotiated gameplay, map, and game-information composition

- Status: Accepted for the bounded MCP composition seam; producer/gateway and host integration remain
  unverified
- Date: 2026-09-13
- Owner: `sts2-mcp-server`
- Revision: `negotiated-composition-v1-mcp`

## Context

The runtime-map profile already contains the six Runtime-v3 gameplay operations and
`sts2.map_snapshot`; the game-information profile owns bounded static and live lookups. Their
constructors are independent and must remain compatible. A composed session therefore needs an
explicit capability contract instead of adding every tool to a legacy catalog.

## Decision

`ToolCatalog::compose_profiles` merges descriptors by unique operation name and negotiates each
operation across MCP, gateway, producer, and caller layers. Operation revisions are compared exactly,
with only the transport suffix `-mcp` treated as compatible. A conflicting revision fails the
negotiation; a missing or unsupported layer records an unavailable reason and leaves unrelated
operations available.

The effective scope is the intersection of all four layers. The six feature groups are
`static_reference`, `live_details`, `gameplay_actions`, `maps`, `profile_reads`, and
`research_reads`. Effective request, response, content, and page-item limits are the minimum of
the participating offers and are bounded by the MCP transport ceiling. `sts2.capabilities` is an
MCP-local, read-only discovery tool; it is never forwarded and requires only caller read scope.

The executable opt-in `STS2_RUNTIME_PROFILE=negotiated-composition-v1` composes the existing
runtime-map and game-information profiles with their fixed gateway mappings. Legacy profile
constructors and catalogs are unchanged. The composed dispatcher routes gameplay/map operations to
the existing Runtime-v3 and Runtime-map adapters and routes lookups to the existing
game-information adapter; no second gameplay adapter or arbitrary downstream route is introduced.

Gateway and producer layers carry an owner-issued capability authority epoch and opaque digest.
Catalog-derived external layers are synthetic and cannot participate in negotiation until a caller
explicitly injects that authority evidence (the same boundary used by deterministic doubles).
Refresh validation binds both authority identities to the composed catalog and rejects a catalog
whose evidence was invalidated by a restart, content reload, or tool-set revision event.

Producer restart, content reload, permission changes, and tool-set revision changes advance the MCP
session epoch, invalidate tracked snapshot references, require a fresh negotiated catalog, and queue
`notifications/tools/list_changed`. Calls that would forward while refresh is required, or that
carry an invalidated snapshot reference, fail with the stale negotiation error before gateway access.
Reinitialization and `tools/list` expose the refresh flag and session epoch.

## Evidence and exclusions

Unit tests cover composition of gameplay/map/lookups, duplicate names, revision conflicts, missing
producer features, scope and limit intersections, local discovery, lifecycle invalidation, and
pre-forward stale rejection. Existing profile and artifact tests remain unchanged and pass.

This ADR establishes source/component behavior only. Dynamic producer capability discovery, gateway
readiness, authoritative game extraction, snapshot freshness, host compatibility, harness delivery,
and native/provider/release support require their owning repositories and remain unverified.
