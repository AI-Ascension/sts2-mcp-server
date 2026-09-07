# Watchdog recovery MCP schema evidence, 2026-09-06

Confirmed scope: the `watchdog-recovery-v1-mcp` catalog and its typed boundary
validation in the MCP adapter. This is component and synthetic executable
evidence; it is not proof of a deployed watchdog, gateway, game host, provider
session, or end-to-end crash recovery.

## Implementation

The catalog advertises exactly nine recovery tools. Each `tools/call` input is a
closed object containing a bounded MCP session identifier and a closed typed
payload. The payload schemas mirror the accepted
`sts2-protocol/watchdog-recovery-v1` request shapes, including UUID/digest/
timestamp/token bounds, enumerations, bounded Runtime-v3 action bytes, and
nested current/original authority contexts. The generated catalog response is
95,014 bytes, below the 262,144-byte recovery frame limit.

Recovery request validation rejects lease policies whose renewal interval is
equal to or greater than the TTL. The same invariant is represented in the
advertised JSON Schema for every lease context and bootstrap policy. The
published recovery artifact remains unchanged and is consumed with schema
digest `fb934d3157485aaf6e13e6ebbb213ec8a14c7fc6f5eeebc06b7a22c1f0009217`.

## Verification

All commands ran in the isolated MCP worktree on pinned Rust 1.97.1 with
`CARGO_TARGET_DIR=/home/timot/ascension-watchdog-work/20260906/targets/mcp-h6r`:

```text
cargo fmt --all --check                                      PASS
cargo test --locked --package sts2-mcp-server \
  --test recovery_artifact --test recovery_mcp                 PASS (5 + 7)
cargo clippy --workspace --all-targets --all-features --locked \
  -- -D warnings                                               PASS
cargo test --workspace --all-targets --all-features --locked   PASS (127 passed, 1 ignored)
cargo run --locked --package repo-policy -- --strict           PASS (147 sized files, 0 warnings, 0 errors)
```

The focused suite covers all valid request fixtures, closed/typed catalog
payloads, unknown fields and wrong payload types, identity and correlation
binding, fixed recovery routes, no retry after uncertainty, secret-redacted
responses, and a real MCP subprocess speaking to a loopback HTTP peer. The
subprocess test is synthetic: it does not launch a game or contact a deployed
gateway.

No native Luna Max model-setting metadata was available in this component
worktree, so Luna/max execution is not claimed as observed. No provider call,
host run, save, installation, or live Train deployment was used for this
evidence.
