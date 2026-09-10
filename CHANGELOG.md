# Changelog

All notable changes are recorded here. The project follows Semantic Versioning once a release contract
exists.

## Unreleased

- 2026-09-09: Add the additive `runtime-v4-expert-rest-action-mcp` profile with
  `sts2.expert_state`, `sts2.expert_rest_action`, and `sts2.expert_rest_reconcile`. The fixed gateway
  paths are `GET /v4/instances/{id}/expert-state`, `POST /v4/instances/{id}/expert-rest-action`, and
  `GET /v4/instances/{id}/expert-rest-actions/{operation_id}`. Its candidate protocol artifact pins
  schema digest `bb3555fae28eb1f79d08a15e9884696a579e4c20836f5016509f17e0f4c36fbd`, with 16 goldens,
  22 mutation fixtures, and Smith/Mend producer fixtures. Operation identity and action bindings are
  retained through accepted, unknown, and transport-failure outcomes; reconciliation uses the same
  operation. Selector admission is bounded to 128 active or pending entries, rejects the 129th before
  forwarding, reclaims terminal entries, and retains per-operation admission/terminal state so a late
  progress receipt remains valid without reactivating a completed selector. HTTP 404/408/502/504 map to
  `unknown`, HTTP 499 to `cancelled`; structured HTTP 503 handling remains gateway-owned. This is
  source/component and synthetic contract evidence; native host legality, settled effects, provider
  execution, deployment, cross-consumer integration, and release remain unverified.

- 2026-09-09: Add the additive `coop-receipt-query-v1-mcp` profile with the read-only
  `sts2.coop_receipt_query` tool and fixed `POST /v1/instances/{id}/coop/receipt-query` path. It
  consumes the proposed-unadmitted `coop-receipt-query-v1` artifact at schema digest
  `3e3eaedb93926b26025abb09d8028491e2632896753688c1182c698fed7d3f7c`, using canonical compact
  UTF-8 ordered bytes and a 16 KiB request/response bound. The tool performs retained receipt lookup
  only; it does not observe, reconcile, retry, or mutate, and the artifact has no admitted consumers.
  Native producer, host, provider, deployment, and release compatibility remain unverified.

- 2026-09-07: Record the Runtime-v4 expert source/component binding at MCP head
  `901c9edd94833fca6bfe322e0c515f91c8b2b281`, integrated in merge
  `5fc337b880b6c38389661c865003e304a02d1136`. Action and reconcile responses are bound to
  correlation, instance, gateway session, lease/epoch, and operation identities. Settled responses
  bind nested `state_id`/`generation` to the outer response; settled dispatch responses also require
  `before_generation` to match the dispatch generation. Copied schema IDs and digests are
  `sts2-runtime-v4-expert` / `0ee034d5da83f34e9fa0ba23038738d56ef8cfccb1c6e752af3ab63d212c8e42`
  and `sts2-runtime-v4-expert-action` /
  `393318bda8c3522c0ecbacc78b95471a9f4dc3f825169d2048f4c74a7b7f2929`. This is synthetic
  source/component evidence; native host legality, settled effects, provider execution,
  cross-consumer integration, deployment, and release remain unverified.

- Add the additive `runtime-map-v1-mcp` profile with seven tools, including the read-only
  `sts2.map_snapshot` bodyless gateway mapping. Pin protocol commit
  `b3d3034f32e68d70c9e681f906ee37d74db153c4` and schema digest
  `ceab0d2dfc471d1ec36d12edaf4654b8c7fdced06548bf47265e11c63f98115b`; validate complete visible
  graph projections at 256 KiB while preserving all legacy profile catalogs and limits. Live host
  map freshness and visualizer compatibility remain unverified.

- Record the merged current MCP main source head `10c4532167fcb91d577ddf3a71cb4759c28bef08` for
  the bounded `runtime-map-v1` profile and copied-artifact consumer. This is source/component and
  artifact-copy evidence; host extraction, native map visibility, navigation, gameplay, release,
  and publication remain unverified.

- Complete the read-only co-op synchronization tool as the explicit executable profile
  `coop-synchronization-v1`, with strict configured-identity admission, complete response
  validation, and real gateway/MCP transport verification. The unpublished broader prototype
  remains in history; no action/vote/effect tools or host mutation authority are added. The
  executable check covered coordinator report convergence, disagreement, disconnect/recovery and
  fencing with zero downstream game connections; it is not native multiplayer evidence.

- Carry the bounded native runtime-v3 Windows/Linux campaign and replay paths through the MCP
  process for the named v0.107.1 fixtures. Model-played Victory and broader compatibility remain
  unverified; see the harness campaign records.
- Consume the coordinated Runtime-v3 continuation schema with argument-free proceed,
  confirm-selection and cancel-selection actions; reject mixed revisions and extra arguments.

- Scoped the Runtime-v3 byte limits to the `runtime-v3-gameplay` profile: poc, runtime-v1, and
  runtime-v2 keep their historical 16 KiB MCP frame, 64 KiB gateway response body, and 16 KiB
  projected content limits (PR #8 had raised them globally to 256/128/128 KiB, unreleased);
  `ToolCatalog::max_frame_bytes` reports the profile's frame limit. Documented that `sts2.recover`
  exposes but does not own the `release_lease`/`stop_episode` lifecycle vocabulary (the gateway
  decides under its `control` scope) and pinned the single fixed recover route with a regression
  test. ADR 0012.

- Default the standalone MCP session to `mcp-session-1` for harness/gateway composition; existing
  same-session configurations must set `STS2_MCP_SESSION_ID` explicitly.

- Complete frozen POC and Runtime-v1 artifact inventories with canonical conformance cases,
  schemas and goldens; verify every copied checksum in CI without ignoring missing entries.

- Preserved independent Runtime-v2 process fixes from PR #7: configured MCP-session admission and
  sanitized HTTP 403 scope-denial mapping, and bounded HTTP 429 retry guidance, without changing
  frozen artifacts or settlement rules.

- Rejected the bare operation-ID segments `.` and `..` before Runtime-v2 dispatch (fail-closed, in
  addition to the existing `/` rejection), and made two loopback tests portable to Windows socket
  semantics without changing product code.
- Preserved structured operation uncertainty when a custom gateway returns an oversized
  Runtime-v3 mutation receipt.
- Split the co-op schema, library catalog, mapping, and tests into a separate unadmitted proposal;
  its source is retained on review/mcp-coop-proposal-source-20260905 pending shared-contract admission.

- Added the exact six-tool Runtime-v3 semantic catalog, bounded fair-play projection, fixed gateway
  mapping and fail-closed timeout handling. For that source/component entry, live
  MCP/gateway/provider execution and target-game settlement were unverified; later dated campaign
  records are scoped separately.

- Added the separate `runtime-v2-mcp` catalog and fixed `submit_action`/`reconcile_action` mapping for
  the argument-free `end_turn` operation, including full-envelope projection, fencing, uncertainty,
  and deterministic accepted/settled/rejected/unknown/cancelled/idempotency tests.
- Copied and checksum-verified the handed-off `sts2-protocol/runtime-v2` release-like artifact with
  schema digest `f7963b19c8ed5bbdc02c08e83c7a2e16c4771ed5eb798b29a8208d7a917a86c2`.
- Added explicit executable profile selection: Runtime-v1 remains the default, while
  `STS2_RUNTIME_PROFILE=runtime-v2` selects the v2 catalog and invalid values fail closed. Runtime-v2
  now maps state/action/reconciliation to fixed v2 routes, rejects configured-identity mismatches,
  recognizes `reconcile_response`, and verifies every local `SHA256SUMS` entry.

- Added the `runtime-v1-mcp` stdin/stdout process profile, real bounded gateway TCP adapter,
  allowlisted runtime projection, and structured stale-generation handling for
  `show_runtime_probe`.

- Confirmed the MCP adapter in the authorized exact-host runtime trace through the gateway and
  managed game-mod probe.

- Added the offline `sts2-protocol/poc-v1` artifact copy and exactly two MCP tools, `get_state` and
  `submit_action`, with deterministic fixed GET/POST gateway mapping tests. No live transport or
  runtime claim is added.
- Added target-local repository governance, policy tooling, workflow guards, and tailored architecture
  documentation for the external MCP-to-gateway boundary.
- Added a non-live Rust MCP framing, capability/catalog, and gateway-adapter mapping seam with a
  deterministic fake-gateway test suite.
- Added no live listener, cross-repository dependency, game integration, provider call, or release
  artifact.
