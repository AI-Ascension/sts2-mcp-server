# ADR 0009: Runtime-v2 MCP and gateway session binding

- Status: Accepted for the component/process adapter
- Date: 2026-09-02

## Context

Runtime-v2 has a gateway-owned `session_id` in its frozen envelope and the
MCP transport has a separate `mcp_session_id` in tool arguments and
correlation metadata. Treating those values as one identity makes it
impossible to prove that a client session is bound to exactly one configured
MCP process while preserving the gateway's own lease fence.

## Decision

The executable MCP process accepts `STS2_SESSION_ID` for the gateway session
and `STS2_MCP_SESSION_ID` for the MCP session. The process binds both values
at startup. Runtime-v2 mappings reject a tool call whose `mcp_session_id`
does not match the configured MCP session before contacting the gateway.

The gateway session remains the `session_id` in the unchanged Runtime-v2
envelope. The MCP session remains in the gateway adapter's correlation and
`x-mcp-session-id` header. The adapter validates both before forwarding. No
MCP-session value is added to the frozen protocol artifact or forwarded to
the game-mod as mutation authority.

The default MCP session is `mcp-session-1`, independently of the gateway session,
as updated by [ADR 0011](0011-composition-mcp-session-default.md). Existing same-session
setups must set `STS2_MCP_SESSION_ID` explicitly to their gateway session. The gateway
and harness must use the same MCP-session value for a connected run.

## Rejection and compatibility

Missing, malformed, or mismatched MCP-session values fail closed before
gateway forwarding. Runtime-v1 mappings retain their existing envelope
behavior while receiving the same process binding. Existing callers that do
not configure a separate MCP session continue to use the gateway session as
the default. The Runtime-v2 schema digest and message fields are unchanged.

## Evidence and limits

Deterministic mapping tests prove distinct session preservation and
pre-forward rejection; the gateway component test proves a mismatched
`x-mcp-session-id` is rejected before downstream forwarding. These are
component evidence only. They do not prove external identity issuance,
multi-process supervision, host compatibility, or live gameplay settlement.
