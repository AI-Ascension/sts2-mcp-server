# Compatibility

## Compatibility dimensions

Compatibility is tracked separately for the MCP wire/profile contract, the gateway API and mapping
contract, the Rust toolchain/runtime, operating system and architecture, configuration, and package
contents. This adapter does not claim compatibility with a game host merely because another component
does.

## Current baseline

The target now contains a local no-I/O package and fake-gateway tests, but no accepted external MCP
profile, frozen gateway API, live server, or public release. It consumes the protocol owner's
release-like POC artifact copy. The current status is
`unverified` for live MCP framing, gateway mapping, authentication, downstream readiness,
timeout/cancellation behavior, and end-to-end operation effects.

| Subject | Current identity | Evidence |
| --- | --- | --- |
| Target package | sts2-mcp-server 0.0.0; local revision wave2-local-v0 | source-derived; not an external compatibility claim |
| Repository policy | target-local Rust repo-policy, version 0.0.0 | source-derived after local checks |
| Rust toolchain | `1.97.1`, edition 2024 | declared; compilation is a local gate |
| MCP protocol revision | `2025-06-18` for the POC handshake | statically pinned; live transport compatibility remains unverified |
| POC protocol artifact | `sts2-protocol/poc-v1`; schema digest `242b8f9233e915a55ea8d2e72ca476c1258169a67e62de72ee5aed848a6a0a19` | copied artifact/schema/checksum tests; protocol head `cad3c85d` |
| Gateway API revision | not frozen | unverified; consume an approved versioned description |
| Game host/loader | outside this target | not applicable to adapter source; owned by game-mod |
| Harness/model/provider | outside this target | not applicable to adapter authority |

## Versioning rules

An internal refactor with identical observable behavior is a patch-level implementation change. An
additive tool, field, or route is minor only after capability advertisement, mapping, fixtures, and
compatibility review. A renamed, removed, or semantically changed MCP or gateway contract is breaking or
must use a separately versioned profile/path with a migration window.

Do not infer the MCP revision from the game version, or the gateway version from the MCP server version.
Keep MCP session, gateway session, game instance, MCP request, gateway request, and host/action IDs in
separate namespaces with explicit mappings and restart/reuse rules.

## Evidence levels

- `build-only`: target-local source and governance tooling compile and pass deterministic checks;
- `transport`: a pinned MCP client exercises a disposable server transport;
- `gateway`: a disposable gateway double proves allowlists, auth, mapping, errors, and lifecycle signals;
- `host`: game-mod proves its own host boundary; this target does not establish it;
- `end-to-end`: the complete client-to-game path proves effect settlement and fresh observation.

Only the first level is available during this preparation wave. A parse, build, handshake, or tool
acknowledgement must not be reported as downstream readiness or game-effect compatibility.
