# ADR 0005: Wave 2 local MCP initialization seam

- Status: Accepted for codebase initialization
- Date: 2026-09-02

## Context

The target needs a non-empty compile/test seam before final MCP and gateway contracts are frozen. A
live listener, external SDK, gateway dependency, or cross-repository path dependency would introduce
runtime and ownership assumptions that this wave is not authorized to settle.

## Decision

Add one target-owned Rust package under crates/mcp-server with four local responsibilities:

- bounded single-frame JSON parsing and deterministic response encoding;
- MCP-shaped request/response handling with local initialization and tools/list/tools/call semantics;
- a deliberately small preparation catalog containing one read-only state tool; and
- a typed GatewayAdapter seam that receives one fixed state route, explicit MCP correlation fields,
  and bounded response/error mapping.

Tests use an in-memory fake gateway to prove valid mapping, malformed input rejection, unsupported
capability rejection, correlation preservation, catalog behavior, and gateway authorization-error
translation. The package has no dependencies, no listener, no provider, no game access, and no
cross-repository path dependency.

The local catalog revision wave2-local-v0 is not the final MCP protocol/profile contract. The
historical 71-tool surface is not imported or implied. Adoption of sts2-protocol or a gateway crate
requires a later contract decision naming ownership, version, consumers, and conformance.

## Consequences

The repository has a real owned source/test seam and can run offline with deterministic fixtures. The
seam proves mapping mechanics, not external MCP compatibility, gateway readiness, authentication,
lifecycle, host behavior, or game-effect settlement. Future implementation must preserve the process
boundary and replace the preparation catalog only through an explicit contract change.
