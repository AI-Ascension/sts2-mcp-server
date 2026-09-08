# ADR 0016: additive Runtime-map MCP profile

- Status: Accepted for the `runtime-map-v1` consumer
- Date: 2026-09-07
- Owner: `sts2-mcp-server`
- Consumer: `ascension-map-visualizer` through the MCP tool boundary

## Context

The visualizer needs the complete visible map projection while existing MCP profiles retain their
catalogs, wire limits, and error behavior. The map contract is a read-only, generation-bound
projection owned by the protocol and game-mod boundaries. MCP owns tool discovery, argument shape,
session binding, fixed gateway mapping, and safe result projection.

The profile consumes protocol commit `7c448bd8d7a695ada48830176f3d738286caafe4` with schema
digest `ceab0d2dfc471d1ec36d12edaf4654b8c7fdced06548bf47265e11c63f98115b`. The corrected contract
keeps graph node IDs, host action IDs, and opaque serialized action-option IDs independent.

## Decision

`ToolCatalog::runtime_map_v1()` is an additive catalog: it clones the six
`runtime-v3-gameplay` tools and appends exactly one read-only tool, `sts2.map_snapshot`. Its
arguments are the configured instance, MCP session, lease, lease epoch, and requested generation;
unknown arguments, unsafe identifiers, mismatched configured sessions, and out-of-range counters
are rejected before the gateway adapter is called.

The executable selects this catalog only when `STS2_RUNTIME_PROFILE=runtime-map-v1`. The profile
has a 256 KiB MCP frame, gateway response, and projected-content budget. Legacy profiles retain
their existing catalogs and bounds.

The tool emits one bodyless `GET /v1/instances/{instance_id}/map-snapshot` request with the existing
MCP session and correlation headers plus explicit gateway authority. It never emits a map mutation,
chooses a downstream path, or retries a request. The projection validates the exact root envelope,
provenance, digest, response kind, identity and generation fences, timeout metadata, and full
visible snapshot shape. It preserves overlapping coordinates and disconnected visible components,
requires current position/history IDs to refer to visited nodes, and validates acyclic edges,
terminal references, and independent graph/host/action-option bindings.

Malformed, stale, foreign, over-limit, or semantically invalid gateway responses become a sanitized
MCP tool error. The adapter does not expose hidden host state, provider output, or action reasoning.
Known HTTP status is retained as an error result; no response is converted into a successful map
when validation fails.

## Deterministic oracle

`runtime_map_v1.rs` checks the seven-tool catalog, fixed bodyless route, session and authority
context, complete golden projection, stale/foreign/unknown-field rejection, and unsupported input
rejection before gateway access. `runtime_map_v1_artifact.rs` verifies the copied manifest, schema,
three goldens, and checksum inventory. The real TCP adapter test sends the profile through the
executable HTTP boundary and receives the complete corrected response. Profile-bound tests prove
the 256 KiB limit and preserve legacy limits.

These checks establish MCP source/component behavior and artifact-copy integrity. They do not
establish a live game-mod, host map freshness, visualizer rendering, or gameplay effect.
