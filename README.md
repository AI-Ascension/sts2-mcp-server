<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/AI-Ascension/.github/main/profile/assets/banner-dark.svg">
  <img alt="AI-Ascension — Inspect how AI requests to a game get fenced, one Rust contract at a time. Runtime: unverified. Deterministic tests: confirmed." src="https://raw.githubusercontent.com/AI-Ascension/.github/main/profile/assets/banner-light.svg" width="100%">
</picture>

# sts2-mcp-server

> **AI-Ascension · tier 3: thin MCP adapter** — Thin MCP tool adapter that maps approved calls to the authenticated gateway API without bypassing it.
>
> **Status:** deterministic in-memory tests `confirmed` at the pinned commit · runtime, host, and game compatibility `unverified` · nothing is live.
> **Proof:** [45-second browser replay](https://ai-ascension.github.io/proof.html) · [Evidence ledger](https://ai-ascension.github.io/evidence.html) · [This repository on the map](https://ai-ascension.github.io/repositories.html#sts2-mcp-server)
> **Seam tests:** [crates/mcp-server/tests/seam.rs](crates/mcp-server/tests/seam.rs) — one tool call maps to one gateway request; malformed frames are rejected before the gateway.
> **Owner:** `sts2-mcp-server` owns the external MCP process boundary: framing, server identity and capabilities, tool schemas, bounded validation, and the versioned mapping to the gateway API.
> **Contribute:** [Organization guide](https://github.com/AI-Ascension/.github/blob/main/CONTRIBUTING.md) · [First tasks](https://ai-ascension.github.io/contributing.html)
>
> AI-Ascension is an independent project. It is not affiliated with or endorsed by Mega Crit or Valve and grants no rights to game files, assets, or marks.

Status: Wave 2 codebase initialization. The target-owned MCP seam includes the two-tool `poc-v1`
mapping and deterministic fake gateway tests; this directory contains a small local MCP seam, not a
live product service. It is intentionally not initialized as a Git repository.

## Owner and consumers

`sts2-mcp-server` owns the external MCP process boundary: framing, server identity and capabilities,
tool schemas, bounded validation, and the versioned mapping from approved MCP calls to the authenticated
gateway API. Its consumers are MCP clients/agents and the Rust harness coordinator. The gateway is the
downstream contract owner; the game-mod and host remain behind it.

## Boundary

```text
MCP client or harness --MCP--> sts2-mcp-server --authenticated gateway API--> sts2-gateway
                                                                            --> sts2-game-mod --> game host
```

The adapter does not own game rules, host objects, saves, game listeners, gateway lifecycle or registry
state, model/provider calls, trajectory/artifact storage, or harness orchestration. It must never route
around the gateway or accept arbitrary downstream paths, headers, or methods.

The current build-completion decision recognizes `sts2-protocol` as the sixth target, but that target is
limited to genuinely shared language- and transport-neutral contracts. This repository owns MCP wire and
tool schemas; it consumes a checked-in copy of the `sts2-protocol/poc-v1` release-like artifact and
versioned gateway descriptions without making the protocol target a second source of boundary behavior.

## Evidence and provenance

No live MCP transport, gateway connection, game load, host compatibility, provider call, release, or
deployment has been run from this target. Those boundaries are `unverified`. The local seam
and fake-gateway tests are deterministic build/test evidence only; they cover exactly two local tools,
fixed GET/POST mappings, and copied-artifact identity. Documentation, policy tooling, and fixtures
must be original or carry explicit provenance and redistribution rights. Proprietary game files,
saves, credentials, personal paths, and copied implementation source do not belong here.

## Local validation

The workspace contains the target-owned sts2-mcp-server crate and Rust repo-policy tool. From this
directory run:

```bash
cargo metadata --locked --no-deps --format-version 1
sha256sum -c --ignore-missing protocol-artifact/poc-v1/SHA256SUMS
cargo test --locked --package sts2-mcp-server --test artifact
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --all-features --locked
cargo run --locked --package repo-policy -- --strict
```

These commands prove local framing/mapping tests and repository policy only. They do not prove a live MCP
transport, gateway readiness, host behavior, lifecycle, model behavior, or end-to-end readiness.
