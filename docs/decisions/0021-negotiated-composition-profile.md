# ADR 0021: negotiated gameplay and game-information composition

- Status: Accepted for the bounded MCP composition seam; producer/gateway and host integration remain
  unverified
- Date: 2026-09-13
- Owner: `sts2-mcp-server`
- Revision: `negotiated-composition-v1-mcp`

## Context

The runtime-map profile contains the six Runtime-v3 gameplay operations and `sts2.map_snapshot`;
the game-information profile owns bounded static and live lookups. Their constructors are
independent and must remain compatible. The negotiated MCP profile uses those local catalogs as
source descriptors, while the fixed Gateway mapping intentionally has no `sts2.map_snapshot`
operation. A composed session therefore needs an explicit capability contract instead of adding
every local tool to a legacy catalog.

## Decision

`ToolCatalog::compose_profiles` merges descriptors by unique operation name and negotiates each
operation across MCP, gateway, producer, and caller layers. Each source operation revision is derived
and validated before deduplication, so identical descriptors that declare conflicting source
revisions fail composition instead of being merged under the composition revision. Operation
revisions are compared exactly, with only the transport suffix `-mcp` treated as compatible. A
conflicting revision fails the negotiation; a missing or unsupported layer records an unavailable
reason and leaves unrelated operations available.

The effective scope is the intersection of all four layers. The six feature groups are
`static_reference`, `live_details`, `gameplay_actions`, `maps`, `profile_reads`, and
`research_reads`. Effective request, response, content, and page-item limits are the minimum of
the participating offers and are bounded by the MCP transport ceiling. `sts2.capabilities` is an
MCP-local, read-only discovery tool; it is never forwarded and requires only caller read scope.

The executable opt-in `STS2_RUNTIME_PROFILE=negotiated-composition-v1` composes the existing
runtime-map and game-information profiles with their explicit fixed Gateway mappings. Legacy
profile constructors and catalogs are unchanged. The composed dispatcher routes only the
negotiated Runtime-v3 and game-information operations; the local `sts2.map_snapshot` source
descriptor is omitted because it has no Gateway mapping, and no second gameplay adapter or
arbitrary downstream route is introduced.

Executable startup verifies the copied Gateway negotiated-capabilities artifact
(`sts2-gateway-negotiated-capabilities-v1`, schema digest
`24491a0a539ac377f889e56910f777152ca3a9e6b16f7238475d122f43582ccb`, source
`sts2-gateway` merge `e15248cd41f89188706a8a19e974f97bf5880a9f`). It fetches the closed,
16 KiB-bounded snapshot over the configured authenticated Gateway connection and checks its exact
instance, caller, gateway/MCP sessions, lease and correlation against runtime configuration. The
producer profile/schema and Runtime-v3 witness are pinned. Gateway offers map through a fixed
operation table only; optional lookup-binding is admitted only after the configured owner-derived
discovery request succeeds and its binding, manifest, run, and epoch agree with the snapshot witness.

`STS2_LOOKUP_BINDING_DISCOVERY_REQUEST_JSON` is a closed, duplicate-key-rejecting, 2 KiB maximum
startup input. It carries only the authenticated owner's scope, positive authority epoch and fixed
discovery correlation; it cannot supply a credential, grant, endpoint, or offer. Startup sends it
to the fixed lookup-binding route before fetching the negotiated snapshot. Invalid owner input,
foreign identity, stale lease, unsupported schema, mismatched witness, or oversized snapshot fails
before the MCP server accepts requests.

Gateway `wire_limits` apply only after a known tool maps to its exact Gateway request and response.
They never set MCP `tools/call` argument limits. The MCP-owned descriptor retains its request and
response bounds, while Gateway `content_limits` constrain semantic response content and page items.
The negotiated recovery schema and dispatcher allow only `reobserve` and `reconcile`; legacy
profiles keep their existing recovery vocabulary.

Producer restart, content reload, permission changes, and tool-set revision changes advance the MCP
session epoch, invalidate tracked snapshot references, require a fresh negotiated catalog, and queue
`notifications/tools/list_changed`. In this profile, snapshot-dependent calls must reference a
snapshot registered by the current session; unknown or untracked references fail closed with the
stale negotiation error before argument validation or gateway access. A standalone profile exposes no
registration flow, so it keeps forwarding a live reference that no lifecycle event invalidated. A
pending tool-set revision constraint is retained across unrelated lifecycle events until a compliant
refresh satisfies it, and only a call the dispatcher explicitly admits, recorded at the gateway
hand-off rather than inferred from the response, may change snapshot tracking state. Reinitialization
and `tools/list` expose the refresh flag and session epoch.

## Evidence and exclusions

Unit tests cover composition of gameplay/lookups, explicit omission of the unmapped map operation,
duplicate names, revision conflicts, missing producer features, scope and limit intersections,
local discovery, lifecycle invalidation, and pre-forward stale rejection. The shipped stdio
executable is also tested from cold startup against
a strict loopback Gateway peer: it performs owner discovery and snapshot negotiation, lists only
the resulting catalog, then forwards capabilities, state, and lookup-binding calls on their fixed
routes. Invalid discovery, stale lease, wrong schema, oversized snapshot, and duplicate JSON keys
are rejected, including producer/identity drift, witness drift, duplicate offers, and unsupported
recovery kinds. This confirms MCP/Gateway-boundary behavior against a synthetic peer, not live
Gateway readiness, producer authority, game-host behavior, or settled gameplay.

Live Gateway readiness, authoritative producer state, host extraction, snapshot freshness,
harness delivery, and native/provider/release support require their owning repositories and remain
unverified.
