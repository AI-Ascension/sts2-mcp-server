# ADR 0010: bind process MCP sessions and preserve scope denials

Status: accepted for the independent Runtime-v2 correction split from PR #7.

## Context

The executable binds separate MCP and gateway sessions. Library mapping already checks the MCP
session, but the TCP adapter must enforce its own configured correlation and header boundary.
HTTP 403 is a known scope denial; treating its ordinary gateway error body as a malformed gameplay
receipt obscures the denial as an unknown operation.

## Decision

Before connecting, reject an absent or foreign `x-mcp-session-id` header or a foreign correlation
MCP session. Compare each against the configured MCP session, independently of gateway-session
checks. Keep valid distinct MCP and gateway session configurations supported.

Add `GatewayError::Forbidden` to the public adapter error enum and classify HTTP 403 into it.
MCP mapping emits the sanitized scope-denial error `-32007` without raw gateway details, synthetic
settlement, or an unknown-operation receipt. This additive enum variant requires exhaustive Rust
consumers to add an explicit match arm; no external stable Rust API compatibility is claimed.

## Consequences and evidence

Frozen Runtime-v2 schemas, artifacts, route names, and settlement-generation rules remain unchanged.
The split preserves current main's operation-ID and portable socket-test corrections.
Negative admission, disposable loopback HTTP, and fake-gateway mapping tests cover these boundaries.
Their results establish source/component behavior only; host settlement and provider execution are
unverified. See [testing](../TESTING.md) for the required validation commands.

Runtime-v2 HTTP 429 guidance preserves only bounded `error_code`, `retryable: true`, and
`retry_after_ms` between zero and 60,000 milliseconds. Invalid guidance fails closed and private
fields are omitted. The adapter never automatically redispatches; synthetic tests cover valid and
out-of-range delays. Gateway support for this guidance is an independent consumer integration gate.
