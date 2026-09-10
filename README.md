<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/AI-Ascension/.github/main/profile/assets/banner-dark.svg">
  <img alt="AI-Ascension — Inspect how AI requests to a game get fenced, one Rust contract at a time. Bounded runtime host trace confirmed. Deterministic tests: confirmed." src="https://raw.githubusercontent.com/AI-Ascension/.github/main/profile/assets/banner-light.svg" width="100%">
</picture>

# Slay the Spire 2 MCP

Part of [Ascension](https://github.com/AI-Ascension/sts2-harness), the AI
Ascension flagship toolkit. The repository slug remains `sts2-mcp-server`;
**The Climb — by AI Ascension** does not broaden this adapter's authority.

> **AI-Ascension · tier 3: thin MCP adapter** — Thin MCP tool adapter that maps approved calls to the authenticated gateway API without bypassing it.
>
> **Status:** deterministic tests, the bounded `runtime-v1` host trace, the runtime-v3 gameplay adapter path, the read-only `coop-synchronization-v1` executable profile, and the additive `seeded-run-v1` and `coop-native-v1` source/component profiles are `confirmed` for the recorded STS2 v0.107.1 evidence · native multiplayer, live seeded-run settlement, and broader compatibility `unverified`.
> **Proof:** [45-second browser replay](https://ai-ascension.github.io/proof.html) · [Evidence ledger](https://ai-ascension.github.io/evidence.html) · [This repository on the map](https://ai-ascension.github.io/repositories.html#sts2-mcp-server)
> **Seam tests:** [crates/mcp-server/tests/seam.rs](crates/mcp-server/tests/seam.rs) — one tool call maps to one gateway request; malformed frames are rejected before the gateway.
> **Owner:** `sts2-mcp-server` owns the external MCP process boundary: framing, server identity and capabilities, tool schemas, bounded validation, and the versioned mapping to the gateway API.
> **Contribute:** [Organization guide](https://github.com/AI-Ascension/.github/blob/main/CONTRIBUTING.md) · [First tasks](https://ai-ascension.github.io/contributing.html)
>
> AI-Ascension is an independent project. It is not affiliated with or endorsed by Mega Crit or Valve and grants no rights to game files, assets, or marks.

Status: Wave 2 codebase initialization plus bounded runtime seams. The target-owned MCP seam includes
the two-tool `poc-v1` mapping, the separate `runtime-v1` process profile, the deterministic
`runtime-v2` gameplay-operation mapping, and the six-tool runtime-v3 gameplay mapping. Dated
Windows/Linux campaign and replay records confirm the MCP path for the named v0.107.1 fixtures.
The separate `coop-synchronization-v1` profile is read-only coordinator reporting; it is not native
multiplayer gameplay. The additive `seeded-run-v1` profile maps one bounded start and one read-only
reconciliation tool through the leased gateway; its source/component checks do not establish a live
native seeded run, profile/save isolation, or release compatibility.

The additive `coop-native-v1` profile is selected with `STS2_RUNTIME_PROFILE=coop-native-v1`. It
exposes seven typed tools for native observation, legal catalogs, local actions, shared votes, peer
rejoin, same-operation recovery, and response-only effect projection. Observation uses the bodyless
`GET /v1/instances/{id}/coop/native/observation` route; the other gateway calls use fixed POST routes
for `legal-catalog`, `action`, `vote`, `rejoin`, and `recover`. The profile consumes the checked-in
`coop-native-v1` artifact at schema digest
`2f3bc99e53080fa11b39592b64fb0ab964a16f568719a2622d0b2caf766ab629`, validates receipt/effect/
observation generation and identity relations, and rejects status or route drift. These are
source/component and synthetic gateway-boundary checks; a disposable two-peer host-backed native
session, provider participation, deployment, and release support remain unverified. See
[ADR 0018](docs/decisions/0018-coop-native-component-consumer.md).

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

The controlled component and co-op executable checks provide no provider-call, release, or deployment
evidence. The controlled component lane exercises the real MCP process against the attached gateway and
a synthetic downstream; the authorized runtime lane additionally exercised the exact packaged host path.
Dated runtime-v3 campaign/replay records and the separate co-op executable check exercised the real MCP
process with reviewed gateway binaries. The local seam and fake-gateway tests remain deterministic
build/test evidence and cover fixed mappings, copied-artifact identity, and bounded projections.
Documentation, policy tooling, and fixtures must be original or carry explicit provenance and
redistribution rights.
Proprietary game files, saves, credentials, personal paths, and copied implementation source do not
belong here.

## Local validation

The workspace contains the target-owned sts2-mcp-server crate and Rust repo-policy tool. From this
directory run:

```bash
cargo metadata --locked --no-deps --format-version 1
for profile in poc-v1 runtime-v1 runtime-v2 runtime-v3-gameplay runtime-v4-expert runtime-v4-expert-action runtime-v4-expert-rest-action runtime-map-v1 coop-synchronization-v1 coop-receipt-query-v1 seeded-run-v1 coop-native-v1; do
  (cd "protocol-artifact/$profile" && sha256sum -c SHA256SUMS)
done
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

The fixed `runtime-v1` action is the safe host-visible `show_runtime_probe`, with a fresh effect
witness and stable stale-generation rejection. Runtime artifact metadata is checked before
projection. Local Rust and mapping tests are confirmed; the authorized host trace confirms the
gateway/mod path for STS2 v0.107.1 on Windows x86-64. The separate runtime-v3 mapping has been
used in the recorded Windows/Linux campaign and replay runs; model-played Victory, native
multiplayer, and broader compatibility remain `unverified`.

The additive `seeded-run-v1` profile is selected with `STS2_RUNTIME_PROFILE=seeded-run-v1`. It
exposes exactly `start_seeded_run` and `reconcile_seeded_run`, mapping to the fixed gateway paths
`POST /v2/instances/{id}/seeded-run` and bodyless
`GET /v2/instances/{id}/seeded-operations/{operation_id}`. Start accepts only the bounded standard
Ironclad, ascension-0, no-modifier context with ordered acts, profile baseline, save policy, and
compatibility identities; reconciliation uses the original operation identity and never resubmits a
seed mutation.

The copied `sts2-protocol/seeded-run-v1` artifact is schema digest
`5c659f344be78f84e8d783986925d462714f933cac95d18943358992f7d3e2b8`, aligned with protocol main
`d3ab5fca7d9d74bb31eeb3e5b343d8024ee44404`. Source/component tests cover catalog, fixed mapping,
identity/context validation, canonical seed and `run_started` witness projection, unknown outcomes,
and artifact checksums. They do not prove native host settlement, profile/save isolation, gameplay,
deployment, or release support.

The additive `runtime-v4-expert` profile is selected with `STS2_RUNTIME_PROFILE=runtime-v4-expert`.
Historical source/component evidence (2026-09-07) at MCP source head
`901c9edd94833fca6bfe322e0c515f91c8b2b281`,
integrated in merge `5fc337b880b6c38389661c865003e304a02d1136`, binds action and reconcile
responses to the request's correlation, instance, gateway session, lease/epoch, and operation
identities; dispatch also checks any returned action against the request. Settled responses bind
nested observation `state_id` and `generation` to the outer response; settled dispatch responses
also require `before_generation` to match the dispatch generation. The copied artifacts pin schema
IDs and digests
`sts2-runtime-v4-expert` / `0ee034d5da83f34e9fa0ba23038738d56ef8cfccb1c6e752af3ab63d212c8e42` and
`sts2-runtime-v4-expert-action` / `393318bda8c3522c0ecbacc78b95471a9f4dc3f825169d2048f4c74a7b7f2929`.
Independent source/component checks passed; native host legality, settled effects, provider
execution, deployment, and release remain `unverified`.

Current default-main source/component update (2026-09-10): MCP main
[`b5a9262f1c76da76ea6f84fca0f1ee821ff67001`](https://github.com/AI-Ascension/sts2-mcp-server/commit/b5a9262f1c76da76ea6f84fca0f1ee821ff67001)
contains the Runtime-v4 expert state/action mapping and merged REST expert-action selector recovery.
The copied admitted expert artifacts retain the digests above; the separate REST-action artifact
remains a candidate at digest
`bb3555fae28eb1f79d08a15e9884696a579e4c20836f5016509f17e0f4c36fbd`. These source/component and
bounded synthetic checks do not establish native host legality, settled effects, provider execution,
deployment, release, or live cross-consumer compatibility.

For the gateway's coordinator-reported peer agreement, select
`STS2_RUNTIME_PROFILE=coop-synchronization-v1`. Its only tool is
`sts2.coop_synchronization`, with explicit `instance_id`, `mcp_session_id`, `lease_id`, and
`lease_epoch`. It reads the current generation; no generation guess is required. The gateway
must have a configured roster and active lease. The MCP credential needs read scope only.
See [ADR 0015](docs/decisions/0015-executable-coop-synchronization.md) and
[executable verification](docs/TESTING.md#co-op-synchronization-verification) for exact scope.

The synchronization profile returns only the gateway's `gateway_peer_reports` response. Its
executable verification covered missing and partial reports, convergence, disagreement,
disconnect/recovery, stale lease fencing, and rejected unknown or regressing reports, with zero
downstream game connections. It provides no action, vote, shared-effect, or peer-game authority;
native multiplayer observation and actuation remain unverified. See the
[dated executable evidence](docs/evidence/coop-synchronization-20260906.md).

The additive `runtime-map-v1` profile selects the six Runtime-v3 gameplay tools plus the read-only
`sts2.map_snapshot` tool. It maps a bodyless GET to the gateway's fixed map snapshot route, validates
the corrected visible-map artifact and generation-bound graph projection, and uses 256 KiB frame,
response, and projected-content limits. Legacy profiles remain unchanged; host map freshness and
visualizer rendering remain unverified. See [ADR 0016](docs/decisions/0016-runtime-map-profile.md).

Historical source/component map update (2026-09-08): MCP main
`10c4532167fcb91d577ddf3a71cb4759c28bef08` contains the additive
`runtime-map-v1` profile and copied-artifact consumer. Its producer pin is merged protocol main
`b3d3034f32e68d70c9e681f906ee37d74db153c4` at schema digest
`ceab0d2dfc471d1ec36d12edaf4654b8c7fdced06548bf47265e11c63f98115b`. This records
source/component and artifact-copy scope; host extraction, live map freshness, visualizer
validation, native map visibility, navigation, gameplay, release, and publication remain
unverified.

Current default-main map source/component update (2026-09-10): MCP main
[`b5a9262f1c76da76ea6f84fca0f1ee821ff67001`](https://github.com/AI-Ascension/sts2-mcp-server/commit/b5a9262f1c76da76ea6f84fca0f1ee821ff67001)
retains the additive `runtime-map-v1` profile and copied-artifact consumer, aligned with current
protocol main `d3ab5fca7d9d74bb31eeb3e5b343d8024ee44404` at schema digest
`ceab0d2dfc471d1ec36d12edaf4654b8c7fdced06548bf47265e11c63f98115b`. This is source/component
and artifact-copy evidence; host extraction, map freshness, visualizer validation, native map
visibility, navigation, gameplay, deployment, release, and publication remain unverified.

The additive `runtime-v4-expert-rest-action` profile is selected with
`STS2_RUNTIME_PROFILE=runtime-v4-expert-rest-action`. It exposes exactly `sts2.expert_state`,
`sts2.expert_rest_action`, and `sts2.expert_rest_reconcile`, using fixed routes
`GET /v4/instances/{id}/expert-state`, `POST /v4/instances/{id}/expert-rest-action`, and
`GET /v4/instances/{id}/expert-rest-actions/{operation_id}`. The candidate artifact is pinned to
schema digest `bb3555fae28eb1f79d08a15e9884696a579e4c20836f5016509f17e0f4c36fbd` and contains 16
goldens, 22 mutation fixtures, and Smith/Mend producer fixtures. Operation identity and action
bindings survive accepted, unknown, and transport-failure outcomes; reconciliation uses the same
operation identity. Selector admission allows 128 active or pending entries, rejects the 129th before
forwarding, and reclaims terminal entries. Per-operation admission and terminal state lets a late
progress receipt remain valid after selector-key eviction without reactivating a completed selector.
HTTP 404/408/502/504 are `unknown` and 499 is `cancelled`; structured 503 handling remains
gateway-owned. These are source/component and synthetic contract claims; native host legality,
provider execution, deployment, and release remain unverified.

The additive `coop-receipt-query-v1` profile is selected with
`STS2_RUNTIME_PROFILE=coop-receipt-query-v1`. Its only tool is `sts2.coop_receipt_query`, mapped to
`POST /v1/instances/{id}/coop/receipt-query`. The proposed-unadmitted artifact pins schema digest
`3e3eaedb93926b26025abb09d8028491e2632896753688c1182c698fed7d3f7c`, uses canonical compact
ordered UTF-8 bytes, and keeps the request/response body within 16 KiB. It performs a retained
receipt lookup only and does not observe, reconcile, retry, or mutate. The artifact has no admitted
consumers; native producer, host, provider, deployment, and release compatibility remain unverified.
