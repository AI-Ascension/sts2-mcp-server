# Testing and evidence

## Test layers

| Layer | Purpose | Current state |
| --- | --- | --- |
| Unit | policy parsing, diagnostics, framing, validation, and pure mapping decisions | checker and MCP unit tests are present |
| Protocol | exact MCP serialization and schemas | future; no product contract yet |
| Component | bounded transport, mapping, auth, timeout, cancellation, fake gateway | deterministic fake-gateway seam is present; live transport is future |
| Integration | real disposable process/socket composition | future and authorized only |
| Host | game-mod/host load and effect behavior | owned by other targets |
| Release smoke | exact package bytes in a clean environment | not started |

## Foundation commands

Run from this target root:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --all-features --locked
cargo run --locked --package repo-policy -- --strict
```

These commands validate the local MCP seam, fake-gateway mapping tests, policy tool, and repository
structure. They do not establish a live MCP transport, gateway readiness, authentication, game
compatibility, model/provider behavior, or end-to-end action settlement.

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

Use `confirmed` only for controlled reproduced behavior, `statically derived` for exact source/config
facts, `inferred` for reasoned consequences, `proposed` for new decisions, and `unverified` when an
external precondition or runtime lane is absent. A skipped check remains visible and is not a pass.
