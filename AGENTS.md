# Repository instructions

## Scope and authority

Direct user instructions and accepted target decisions take precedence. The detailed local standards
are in [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md), [`docs/PRODUCT.md`](docs/PRODUCT.md),
[`docs/CODING_STANDARDS.md`](docs/CODING_STANDARDS.md), [`docs/TESTING.md`](docs/TESTING.md),
[`docs/COMPATIBILITY.md`](docs/COMPATIBILITY.md), [`docs/LICENSING.md`](docs/LICENSING.md),
[`docs/WORKFLOWS.md`](docs/WORKFLOWS.md), [`docs/POLICY_AS_CODE.md`](docs/POLICY_AS_CODE.md), and
[`RELEASING.md`](RELEASING.md).

This target is an external MCP adapter. It is not a game-host integration layer, a gateway, a game
rules engine, a model runner, or a provider client.

## Ownership rules

- This repository owns MCP transport framing, initialization, capabilities, tool descriptors, bounded
  input/output handling, and the explicit mapping to the approved gateway contract.
- `sts2-gateway` owns lifecycle, instance/session/lease selection, routing policy, authentication
  authority, readiness, and registry state.
- `sts2-game-mod` owns host access and authoritative game HTTP behavior; the host owns legal game state
  and effects. `sts2-game-core` owns host-independent domain meaning.
- `sts2-harness` owns coordination, experiment/model/provider ports, trajectories, replay, and artifacts.
- `sts2-protocol` is the accepted sixth target for only genuinely shared language- and transport-neutral
  contracts. It does not own MCP catalogs, gateway routes, game rules, or host behavior.

Runtime communication and compile-time dependencies are separate. The runtime path is client or harness
to MCP server to gateway to an isolated game-mod instance. The MCP server must not contact a game process
directly, depend on host/game-mod implementation crates, or proxy arbitrary downstream requests.

## Safety and provenance

Preserve existing files and user work. This scaffold is not a Git repository; do not initialize Git,
stage, commit, push, merge, publish, deploy, install, launch a game, call a provider, or mutate a profile
or save unless separately authorized. Never add proprietary host assemblies, credentials, personal paths,
saves, or generated build output. Do not copy or transliterate a reference implementation.

Treat every unimplemented runtime or external contract fact as `unverified` or `proposed`, with a safe
probe recorded. A successful parse, build, handshake, or acknowledgement is not proof of downstream
readiness or game-effect settlement.

## Change and validation rules

Before editing, inspect the target tree and preserve unrelated work. Product source must be Rust-first,
with narrow typed boundary modules and no hidden global authority. Keep transport, gateway mapping, and
MCP content separate from domain or host concerns. Reject arbitrary routes, unbounded content, and
unknown authority assumptions at the boundary.

Every change runs the foundation entrypoints:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --all-features --locked
cargo run --locked --package repo-policy -- --strict
```

The workspace now contains tools/repo-policy and the non-empty crates/mcp-server seam. Do not add
another placeholder or generic common crate. Further product initialization requires an owned consumer,
accepted contract, source responsibility, and deterministic test seam.
