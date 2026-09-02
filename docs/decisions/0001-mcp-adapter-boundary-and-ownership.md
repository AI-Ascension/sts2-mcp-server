# ADR 0001: External MCP adapter boundary and ownership

- Status: Accepted for the foundation
- Date: 2026-09-02

## Context

The target must expose MCP to clients while the game host, game rules, authoritative mutations, and
instance lifecycle belong to other components. A direct bridge to a game listener would duplicate
authority and make authentication, readiness, and failure ownership ambiguous.

## Decision

`sts2-mcp-server` owns MCP framing, server identity, capabilities, tool/profile schemas, bounded input
and output, and an explicit versioned mapping to the authenticated `sts2-gateway` API. The mapping uses
a fixed method/path/header/body allowlist and preserves identifier namespaces and error origin.

`sts2-gateway` owns lifecycle, registry, ports, leases, instance/session targeting, readiness, routing,
and downstream authorization. `sts2-game-mod` and the host own game access and effects. `sts2-game-core`
owns host-independent game meaning. `sts2-harness` owns coordination, model/provider ports, trajectories,
replay, scoring, and artifacts.

The MCP server must not contact the game process directly, implement game rules or settlement, own
gateway lifecycle, store a registry, call providers, or act as an arbitrary proxy. Runtime communication
and compile-time dependency graphs are documented separately.

## Alternatives

1. Embed MCP in the mod: rejected because it merges host and external transport failure domains.
2. Let the server call game HTTP directly: rejected because it bypasses gateway lifecycle and auth.
3. Expose a generic proxy: rejected because it expands authority beyond approved contracts.

## Consequences

Mapping and transport tests can use a fake gateway without a game. Gateway and host compatibility remain
independent. The first product implementation must freeze its MCP profile, gateway revision, schemas,
error mapping, and conformance oracle before advertising behavior.
