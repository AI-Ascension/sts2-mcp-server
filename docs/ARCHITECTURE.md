# Architecture

## Purpose and owner

`sts2-mcp-server` is the external MCP framing and gateway-mapping boundary. It owns the process that
speaks MCP to clients and sends only approved, authenticated requests to `sts2-gateway`. It does not
own a game host, game rules, game state, gateway lifecycle, model, provider, or experiment.

## Runtime topology

```text
MCP client or harness
          │ MCP framing
          ▼
  sts2-mcp-server ── versioned, authenticated gateway API ──► sts2-gateway
                                                               │
                                                               ▼
                                                        sts2-game-mod ──► game host
```

The gateway is the control plane for instance identity, lifecycle, leases, readiness, routing, and
downstream authorization. The game-mod/host is authoritative for legal game state and effects. The
harness coordinates experiments and owns model, provider, trajectory, replay, and artifact concerns.

## Ownership and dependency direction

Runtime communication and compile-time dependencies are distinct:

```text
Runtime:      harness/client -> MCP server -> gateway -> game-mod -> host
Shared data:   protocol target -> accepted language-neutral contracts only
MCP boundary:  MCP server -> declared gateway-interface descriptions
```

The current project decision recognizes `sts2-protocol` as the sixth target. Its scope is limited to
genuinely shared language- and transport-neutral contracts with named consumers and conformance. It
must not contain MCP tool catalogs, gateway routes, host objects, game rules, model behavior, or provider
semantics. Boundary-specific MCP contracts remain owned here. The POC consumes a checked-in artifact
copy and does not link a protocol implementation crate.

The MCP package may eventually consume an accepted protocol package and a versioned gateway description,
but it must not depend on game-mod implementation, host assemblies, gateway registry internals, or
harness crates. The current package has no cross-repository dependency and uses only local Rust types.

## Boundary responsibilities

The adapter may own:

- MCP initialization, protocol revision negotiation, identity, capabilities, and transport lifecycle;
- versioned tool descriptors, argument schemas, validation, bounded content, and profile selection;
- a fixed gateway method/path/header/body allowlist and identifier/session mapping;
- one-time error, timeout, retry, polling, cancellation, and progress translation at the boundary; and
- redacted diagnostics, compatibility metadata, conformance fixtures, and package behavior.

The adapter must not:

- contact a game process or listener directly, inspect host objects, or access saves;
- reimplement game rules, action legality, state extraction, settlement, or host-thread dispatch;
- allocate ports, supervise instances, store gateway leases, or become a lifecycle registry;
- call models/providers or own trajectories, replay, scoring, or artifact lineage; or
- act as an arbitrary HTTP proxy or infer authorization from localhost or tool descriptions.

## Safety invariants

Every request is bound to an authenticated target and explicit identifier namespace. Unknown routes,
profiles, tools, methods, fields, and downstream response shapes fail closed. Accepted downstream work is
never silently discarded. A client timeout or MCP acknowledgement does not prove that a gateway or game
operation completed; completion must be represented by the approved downstream contract and fresh state.

Transport, protocol, mapping, and test-support modules should each have one cohesive responsibility.
Pure validation and mapping must remain testable without a process, socket, clock, or game. Any future
unsafe or host-specific code belongs outside this target and requires a separate decision.

## Minimal POC mapping

The active catalog has exactly two tools: `get_state` maps to `GET /v1/instances/{instance}/state`,
and `submit_action` maps to `POST /v1/instances/{instance}/action`. Both require an explicit instance
and MCP session. The action tool additionally requires generation, `use_budget`, and bounded units;
it constructs the complete copied `poc-v1` action-request envelope. Gateway responses are reduced to
bounded, allowlisted state/error projections before MCP content is emitted, preserving downstream
status/error identity without reimplementing game legality.

## Runtime process adapter

ADR 0007 adds `crates/mcp-server/src/bin/sts2-mcp-server.rs` as an owner-local process entry point.
Its transport is bounded newline-delimited JSON-RPC over stdin/stdout. The runtime catalog is a
separate `runtime-v1-mcp` profile containing exactly `get_state` and `submit_action`. Mapping builds
only fixed gateway paths and a complete runtime action envelope; the TCP adapter rewrites the
configured instance/session/lease values, injects bearer authentication and correlation headers,
and rejects malformed or oversized responses.

Before a result reaches MCP content, the adapter requires the runtime protocol version, exact schema
digest, provenance, identity, epoch, generation, action, observation, status, and witness shape to
match its allowlist. HTTP 409 is preserved when it contains a valid structured stale action result;
other gateway rejection statuses become sanitized MCP tool errors. The MCP server remains a thin
adapter: it owns neither gateway lease authority nor host/game semantics.

The process and mapping are source/build-confirmed. The authorized host trace confirms the real
downstream listener and safe probe effect for STS2 v0.107.1 on Windows x86-64; the action is not a
gameplay mutation and broader host compatibility remains unverified.

## Runtime-v2 gameplay-operation mapping

The executable defaults to `runtime-v1`; `STS2_RUNTIME_PROFILE=runtime-v2` selects the separate
`runtime-v2-mcp` catalog, while unknown profile values fail closed. The v2 catalog contains exactly
`get_state`, `submit_action`, and `reconcile_action`. `get_state` maps to
`GET /v2/instances/{id}/state`, `submit_action` maps to `POST /v2/instances/{id}/action`, and
`reconcile_action` maps to `GET /v2/instances/{id}/operations/{operation_id}` with no
mutation-bearing body. Submission accepts only bounded instance/session/lease context, a stable
`operation_id`, the expected `generation`, and the fixed `end_turn` action with no action arguments.
Reconciliation requires the same bounded context and operation identity and cannot dispatch a second
mutation.

Both calls carry the complete copied Runtime-v2 envelope to the gateway. Valid gateway results retain
all envelope fields, including the exact status and `error_code` origin. Unknown envelope fields,
metadata drift, identity mismatch, invalid fences, malformed observations, and invalid witnesses fail
closed. Timeout or disconnect uncertainty becomes an `unknown` result and is never retried automatically.
`accepted` is admission only. The adapter reports `settled` only for a downstream `settled` result with
a fresh observation whose generation advances past the request and a matching `turn_end_settled`
witness. It does not infer settlement from an acknowledgement or from a state read. Runtime-v1's
catalog, routes, and projection remain unchanged.

The executable validates the configured MCP session in both correlation metadata and the MCP
header before connecting, independently of gateway-session authority. HTTP 403 becomes a typed
scope-denial error, so authorization rejection does not become an uncertain gameplay operation.

Runtime-v2 HTTP 429 guidance preserves only bounded `error_code`, `retryable: true`, and
`retry_after_ms` between zero and 60,000 milliseconds. Invalid guidance fails closed and private
fields are omitted. The adapter never automatically redispatches; synthetic tests cover valid and
out-of-range delays. Gateway support for this guidance is an independent consumer integration gate.

The standalone process defaults its MCP session to `mcp-session-1`, independently of the gateway
session default `session-1`. Explicit `STS2_MCP_SESSION_ID` values remain authoritative; same-session
setups must configure both variables. [ADR 0011](decisions/0011-composition-mcp-session-default.md)
records the configuration compatibility change.

## Runtime-v3 semantic mapping

ADR 0009 adds a separate Runtime-v3 profile with exactly six tools: observe, legal actions,
dispatch, wait, reobserve, and recover. The adapter carries bounded identity/generation context,
maps each call to one fixed gateway route, retains typed status and transition witnesses, and turns
timeout or disconnect uncertainty into a recovery-required unknown result. It exposes no raw host
object, arbitrary route, shell, coordinate, or process tool.

Legal-action responses must match the state and generation used to request the catalog. Observation
and reobservation reads may discover newer host generations. A settled dispatch witness must start
at the dispatch generation and name the returned state; wait/recovery retain operation-bound receipts
without requiring their settlement generation to exceed a caller's subsequently refreshed generation.
The host remains responsible for independent action-completion evidence; envelope validation alone
cannot establish that an effect occurred.

`sts2.recover` carries the recovery vocabulary (`reobserve`, `reconcile`, `release_lease`,
`stop_episode`) to one fixed route, `POST /v3/instances/{id}/recover`. The adapter exposes and
shape-checks the vocabulary but does not own lifecycle: lease release belongs to the gateway and
episode stop to the harness. The gateway authorizes recovery with the `control` scope and decides
whether it happens; the adapter constructs no lifecycle route of its own, holds no lease or episode
state, and reports a scope denial as a typed error rather than an uncertain operation.

Byte limits are profile-scoped. The poc, runtime-v1, and runtime-v2 profiles keep their historical
16 KiB MCP frame, 64 KiB gateway response body, and 16 KiB projected content limits; only the
Runtime-v3 profile accepts 256 KiB frames, 128 KiB bodies, and 128 KiB projected content. The catalog
owns the frame limit (`ToolCatalog::max_frame_bytes`) and the executable selects the body limit
together with the catalog, so the Runtime-v3 addition changes no bound a legacy consumer sees.

## Read-only co-op synchronization

The explicit `coop-synchronization-v1` profile exposes one fixed GET tool. Gateway serializes
the complete shared response from its bounded coordinator-report ledger, and MCP validates
and projects every field. Both are actual consumers of the same copied artifact. The broader
prototype remains preserved in history under ADR 0012; ADR 0015 implements its admitted
read-only scope. The response's source label is retained, and no action/vote/effect tool or
peer-report ingestion capability is exposed. Gateway owns roster, freshness and authority.

## Runtime-map profile

ADR 0016 adds the additive `runtime-map-v1-mcp` catalog. It keeps the six Runtime-v3 gameplay
tools and appends the read-only `sts2.map_snapshot` tool. MCP owns its bounded argument schema,
configured MCP-session fence, fixed bodyless `GET /v1/instances/{id}/map-snapshot` mapping, and
projection of the complete visible graph. Gateway owns lease and gateway identity admission; the
game-mod owns host observation and map meaning.

The selected profile uses 256 KiB frame, gateway-body, and projected-content limits. Its projection
consumes the corrected protocol artifact at merged main commit `b3d3034f32e68d70c9e681f906ee37d74db153c4` and
digest `ceab0d2dfc471d1ec36d12edaf4654b8c7fdced06548bf47265e11c63f98115b`. It preserves
overlapping coordinates and disconnected visible components, checks visited position/history
relationships, and keeps graph, host-action, and opaque action-option identities separate. Legacy
profiles and their bounds remain unchanged. This source/component adapter does not prove host map
freshness, visualizer rendering, or navigation effects.

## Runtime-v4 expert REST-action profile

The additive `runtime-v4-expert-rest-action` profile is selected with
`STS2_RUNTIME_PROFILE=runtime-v4-expert-rest-action`. It exposes exactly `sts2.expert_state`,
`sts2.expert_rest_action`, and `sts2.expert_rest_reconcile`, mapped to the fixed routes
`GET /v4/instances/{id}/expert-state`, `POST /v4/instances/{id}/expert-rest-action`, and
`GET /v4/instances/{id}/expert-rest-actions/{operation_id}`. State is read-only; action submission
and reconciliation use the same operation identity, and reconciliation has no mutation-bearing body.

An action operation is bound to its MCP session, instance, gateway session, lease and epoch, request
generation, state ID, and typed action. Reusing an operation ID with different binding or action data
fails before gateway forwarding. Accepted, rejected, unknown, cancelled, and transport-failure paths
retain that binding; reconciliation reuses it and never resubmits the action. Response status mapping
is strict: accepted is HTTP 202, settled is 200, rejected is 400 or 409, unknown is 404/408/502/504,
and cancelled is 499. Structured HTTP 503 behavior remains gateway-owned.

Smith and Mend rest-option selections use a bounded admission catalog. At most 128 active or pending
selection keys are admitted, with pending reservations taken before forwarding; a 129th admission
fails closed. Terminal selections reclaim active capacity. The operation ledger retains a valid
selection admission, generation, and terminal marker, so a late progress or completion receipt can
still be validated after selector-key eviction without reactivating a completed selector. Newer
generations supersede older receipts; an older receipt cannot rewind the retained admission.

The copied candidate artifact has schema digest
`bb3555fae28eb1f79d08a15e9884696a579e4c20836f5016509f17e0f4c36fbd`, 16 golden vectors, 22 mutation
fixtures, and Smith/Mend producer fixtures. Artifact and synthetic mapping checks establish
source/component behavior only. Native host legality, provider execution, deployment, and release
compatibility remain unverified.

## Co-op receipt-query profile

The additive `coop-receipt-query-v1` profile is selected with
`STS2_RUNTIME_PROFILE=coop-receipt-query-v1` and exposes only `sts2.coop_receipt_query`. It maps a
read-only request to `POST /v1/instances/{id}/coop/receipt-query`, canonicalizes the ordered request
fields as compact UTF-8 bytes, and caps the request and response body at 16 KiB. The gateway lookup
uses the original operation and identity fields to retrieve a retained receipt; the MCP boundary does
not observe current state, reconcile an operation, retry, queue, or mutate the game.

The copied artifact is `proposed_unadmitted`, with schema digest
`3e3eaedb93926b26025abb09d8028491e2632896753688c1182c698fed7d3f7c`. It currently has no admitted
consumers. Its checksum and canonical-wire tests are contract evidence only; native producer, host,
provider, deployment, and release compatibility remain unverified.
