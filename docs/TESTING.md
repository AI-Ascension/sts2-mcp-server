# Testing and evidence

## Test layers

| Layer | Purpose | Current state |
| --- | --- | --- |
| Unit | policy parsing, diagnostics, framing, validation, and pure mapping decisions | checker and MCP unit tests are present |
| Protocol | exact MCP serialization and copied POC mapping | two-tool local fixtures are present |
| Component | bounded transport, mapping, auth, timeout, cancellation, fake gateway | fake-gateway mapping and real stdio notification tests are present; cancellation effects are not established |
| Integration | real disposable process/socket composition | documented Runtime-v1 component evidence is separate from local unit tests; new runs require authorization |
| Host | game-mod/host load and effect behavior | owned by other targets |
| Release smoke | exact package bytes in a clean environment | not started |

## Foundation commands

Run from this target root:

```bash
cargo metadata --locked --offline --no-deps --format-version 1
for profile in poc-v1 runtime-v1 runtime-v2; do
  (cd "protocol-artifact/$profile" && sha256sum --check SHA256SUMS)
done
cargo test --locked --offline --package sts2-mcp-server --test artifact
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --all-features --locked
cargo run --locked --package repo-policy -- --strict
```

These commands validate the local MCP seam, copied artifact identity/checksums/schema fixtures, exactly two tool descriptors,
fixed GET/POST fake-gateway mappings, policy tool, and repository structure. They do not establish a
live MCP transport, gateway readiness, authentication, game compatibility, model/provider behavior,
or end-to-end action settlement.

## Future product tests

Before implementation, freeze the MCP revision, profile, gateway API description, route allowlist, error
mapping, identifier ledger, and bounded resource limits. Then add readable golden fixtures for every
advertised method, capability, tool, argument, result, error, and mapping. Use a deterministic fake
gateway for malformed input, unknown tool/route, session and instance mismatch, auth failure, oversized
content, downstream errors, retryability, timeout, cancellation before/after forwarding, reconnect, and
shutdown.

Every accepted operation must resolve to success, explicit rejection, or explicit cancellation. A client
timeout or acknowledgement is not evidence that the downstream game effect completed. Host or end-to-end
claims require exact versions, hashes, disposable data, setup, requests, observations, cleanup, and the
evidence level.

## Evidence language

Use `confirmed` only for controlled reproduced behavior, `source-derived` for exact source/config
facts, `inferred` for reasoned consequences, `proposed` for new decisions, and `unverified` when an
external precondition or runtime lane is absent. A skipped check remains visible and is not a pass.

## Runtime profile checks

`runtime_v1_shape.rs` exercises every required response field, closed root/nested objects, state
null fields, accepted/rejected result combinations, and matching witness/envelope generations.
Its three MIT response fixtures are copied from `sts2-protocol` commit
`40bdfc30cedcc11eea001ad28f4a6e58c788f98a`, `artifacts/runtime-v1/golden/` (schema digest
`a76086d7a68668fd4cff53999369d2b450b0d6623827393882f458f2aa1f93eb`). Their exact legacy MCP
projections remain unchanged. These are synthetic protocol-owner goldens, not host-effect evidence.

`runtime_v2_uncertainty.rs` checks that invalid action receipts remain unknown without retry or
untrusted payload leakage; state-read failures report missing observation rather than inventing an
operation. Operation IDs containing `/`, and the bare segments `.` and `..`, are rejected before
submission, because they cannot be looked up as one plain segment of the current gateway
reconciliation route.

`runtime_v2_sessions.rs` checks distinct configured MCP/gateway sessions for state, submission and
reconciliation, rejects foreign MCP sessions before forwarding, and exercises the actual executable
against a disposable loopback HTTP peer. It verifies both outbound header namespaces and the gateway
envelope, not only library construction. No host process or provider participates.

`json_notifications.rs` checks raw Unicode and surrogate-pair round trips, duplicate-key and
leading-zero rejection, and a 64-value nesting limit before recursive parsing. It also exercises the
actual binary's stdin/stdout to prove notifications produce no output, not even a blank line. The
integer-only boundary remains intentional; this is not general-purpose JSON-number support.
Notifications do not dispatch request-only tools. The synchronous transport still does not interrupt
an in-flight gateway call on cancellation; notification silence is not evidence of cancellation.

`runtime_mapping.rs` confirms the two runtime tool calls, fixed routes, complete action envelope,
effect-witness projection, and structured stale-generation preservation. `runtime_artifact.rs`
confirms the copied manifest and schema bytes. The runtime binary has bounded stdin frame and HTTP
adapter paths, configured MCP-session propagation, and pre-forward session rejection, and builds with
the pinned Rust toolchain.

A controlled component lane may run this process against the real gateway and a synthetic downstream;
that confirms MCP/gateway transport and mapping only. It does not prove the managed mod, Godot
main-thread execution, STS2 host compatibility, a disposable game profile, or gameplay mutation.

The executable HTTP unit tests use ephemeral loopback listeners, not a live gateway or game. They
cover slow-drip headers/bodies against one total deadline, an unread request writer, expired deadlines,
strict JSON content type and Content-Length framing, duplicate headers, unsupported encodings,
truncation, header-size limits, and outbound header injection. Configuration tests reject DNS names,
non-loopback addresses, port zero, and control/non-ASCII token bytes.

The executable binding tests reject missing/mismatched bodyless authority before any connection and
reject foreign response instance/session/lease/epoch, wrong correlation, or wrong route-specific kind.
The Runtime-v2 mapping regression verifies that reconciliation preserves all four authority headers
without adding a mutation body.

The executable binding tests reject missing or foreign MCP-session headers and correlation before
connecting, while accepting distinct configured MCP and gateway sessions. A disposable loopback
HTTP 403 peer verifies typed scope-denial classification without forwarding private denial details.
These are source/component checks and establish no host settlement or provider evidence.

Runtime-v2 HTTP 429 guidance preserves only bounded `error_code`, `retryable: true`, and
`retry_after_ms` between zero and 60,000 milliseconds. Invalid guidance fails closed and private
fields are omitted. The adapter never automatically redispatches; synthetic tests cover valid and
out-of-range delays. Gateway support for this guidance is an independent consumer integration gate.

A pure configuration-selection test covers the standalone `mcp-session-1` default, explicit distinct
and same-session overrides, empty values, and invalid-Unicode configuration without mutating shared
process environment. This verifies default selection, not cross-process readiness.

`runtime_v3_gameplay_artifact` verifies the complete earlier protocol PR #7 artifact checksums,
all eight schema goldens and the producer's targeted reconciliation response through MCP.
`runtime_v3_gameplay_mapping` covers nonzero-card/target reconciliation, mismatched witnesses,
exact dispatch payload correlation and uncertainty without a fabricated read receipt.
`runtime_v2_uncertainty` covers malformed transport/envelope results after submission; these
remain unknown under the original operation ID without mutation retries or raw-payload leakage.
