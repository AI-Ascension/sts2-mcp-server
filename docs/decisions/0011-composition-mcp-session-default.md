# ADR 0011: align the standalone MCP session default

Status: accepted by the repository review orchestration decision.

## Context

The harness and gateway composition use `mcp-session-1` for MCP correlation. The standalone MCP
process previously inherited the gateway session (`session-1`) when `STS2_MCP_SESSION_ID` was absent.
Configured session admission then rejected otherwise compatible default composition requests.

## Decision

Default `STS2_MCP_SESSION_ID` to `mcp-session-1`, independently of `STS2_SESSION_ID`. Retain explicit
nonempty overrides and fail closed on empty or invalid-Unicode values. Existing same-session setups
must explicitly configure `STS2_MCP_SESSION_ID` to their gateway session value.

## Consequences and evidence

This is a configuration-default compatibility change; wire schemas, frozen artifacts, routes and
identity admission are unchanged. The gateway remains gateway-session authority. Deterministic pure
selection tests cover default and explicit values without shared environment mutation. Local tests
establish configuration behavior only, not live composition, host settlement or provider execution.
