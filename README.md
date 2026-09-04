<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/AI-Ascension/.github/main/profile/assets/banner-dark.svg">
  <img alt="AI-Ascension — Inspect how AI requests to a game get fenced, one Rust contract at a time. Bounded runtime host trace confirmed. Deterministic tests: confirmed." src="https://raw.githubusercontent.com/AI-Ascension/.github/main/profile/assets/banner-light.svg" width="100%">
</picture>

# sts2-mcp-server

> **AI-Ascension · tier 3: thin MCP adapter** — Thin MCP tool adapter that maps approved calls to the authenticated gateway API without bypassing it.
>
> **Status:** deterministic tests and one bounded `runtime-v1` host trace `confirmed` for STS2 v0.107.1 on Windows x86-64 · broader compatibility and gameplay mutation `unverified`.
> **Proof:** [45-second browser replay](https://ai-ascension.github.io/proof.html) · [Evidence ledger](https://ai-ascension.github.io/evidence.html) · [This repository on the map](https://ai-ascension.github.io/repositories.html#sts2-mcp-server)
> **Seam tests:** [crates/mcp-server/tests/seam.rs](crates/mcp-server/tests/seam.rs) — one tool call maps to one gateway request; malformed frames are rejected before the gateway.
> **Owner:** `sts2-mcp-server` owns the external MCP process boundary: framing, server identity and capabilities, tool schemas, bounded validation, and the versioned mapping to the gateway API.
> **Contribute:** [Organization guide](https://github.com/AI-Ascension/.github/blob/main/CONTRIBUTING.md) · [First tasks](https://ai-ascension.github.io/contributing.html)
>
> AI-Ascension is an independent project. It is not affiliated with or endorsed by Mega Crit or Valve and grants no rights to game files, assets, or marks.

Status: Wave 2 codebase initialization plus bounded runtime seams. The target-owned MCP seam includes
the two-tool `poc-v1` mapping, the separate `runtime-v1` process profile, and the deterministic
`runtime-v2` gameplay-operation mapping. Runtime-v2 source/fake tests are confirmed; live host
settlement and gameplay mutation remain unverified.

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
The checked-in `protocol-artifact/runtime-v2` package is the byte-verified Runtime-v2 release-like copy;
its digest is pinned in one owner-local metadata module.

## Evidence and provenance

No provider call, release, or deployment has been run from this target. The controlled component
lane exercises the real MCP process against the attached gateway and a synthetic downstream; the
authorized runtime lane additionally exercised the exact packaged host path. The local seam and
fake-gateway tests remain deterministic build/test evidence and cover exactly two local tools,
fixed GET/POST mappings, and copied-artifact identity. Documentation, policy tooling, and fixtures
must be original or carry explicit provenance and redistribution rights. Proprietary game files,
saves, credentials, personal paths, and copied implementation source do not belong here.

## Local validation

The workspace contains the target-owned sts2-mcp-server crate and Rust repo-policy tool. From this
directory run:

```bash
cargo metadata --locked --no-deps --format-version 1
sha256sum -c --ignore-missing protocol-artifact/poc-v1/SHA256SUMS
(cd protocol-artifact/runtime-v2 && sha256sum -c SHA256SUMS)
cargo test --locked --package sts2-mcp-server --test artifact
cargo test --locked --package sts2-mcp-server --test runtime_v2_artifact --test runtime_v2_mapping
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --all-features --locked
cargo run --locked --package repo-policy -- --strict
```

These commands prove local framing/mapping tests and repository policy only. They do not prove a live MCP
transport, gateway readiness, host behavior, lifecycle, model behavior, or end-to-end readiness.

## Runtime process profile

The `sts2-mcp-server` runtime binary reads one bounded newline-delimited JSON-RPC request per stdin
line and writes one response per stdout line. In its `runtime-v1` profile it exposes exactly
`get_state` and `submit_action`, maps them to fixed gateway paths, injects the configured bearer and
lease identity, and projects only allowlisted runtime results. It is a real MCP-to-gateway TCP
adapter, not an MCP provider and not a direct game client.

The executable defaults to `runtime-v1`; setting `STS2_RUNTIME_PROFILE=runtime-v2` selects the
separate `runtime-v2-mcp` catalog, and any other profile value fails closed. Runtime-v2 exposes
`get_state`, `submit_action`, and `reconcile_action`: state maps to `GET /v2/instances/{id}/state`,
submission maps to `POST /v2/instances/{id}/action`, and reconciliation maps to
`GET /v2/instances/{id}/operations/{operation_id}` with no mutation-bearing body. Submission admits
exactly `end_turn` with a required stable `operation_id`, lease epoch, and expected generation; the
reconcile call uses that same operation identity without dispatching another mutation. Both profiles
keep their existing v1/v2 mapping paths isolated. Runtime-v2 gateway timeout/disconnect uncertainty
is surfaced as `unknown` with no automatic retry. `accepted` is admission only; MCP reports `settled`
only when the downstream result contains a fresh post-action observation and the
`turn_end_settled` witness. MCP does not infer settlement from an acknowledgement or a state read.

The fixed action is the safe host-visible `show_runtime_probe`, with a fresh effect witness and
stable stale-generation rejection. Runtime artifact metadata is checked before projection. Local
Rust and mapping tests are confirmed; the authorized host trace confirms the gateway/mod path for
STS2 v0.107.1 on Windows x86-64. Gameplay mutation and broader compatibility remain `unverified`.
