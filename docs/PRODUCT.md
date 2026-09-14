# Product contract

## Purpose

The eventual product is a small external Rust process that presents an approved MCP profile to clients
and maps accepted calls to an authenticated `sts2-gateway` contract. It is an adapter, not an additional
game authority.

## Owner and consumers

The MCP server owns framing, server identity, capabilities, tool descriptions, argument validation,
bounded result content, and one explicit mapping per approved gateway operation. MCP clients/agents and
the `sts2-harness` coordinator consume the MCP surface. `sts2-gateway` consumes the downstream requests;
the game-mod and host remain the authoritative game boundary.

## In scope when implementation is approved

- MCP initialization, protocol revision negotiation, capabilities, sessions, and transport lifecycle;
- versioned tool/profile catalogs and exact schemas;
- gateway endpoint/target selection under authenticated lease/session rules;
- fixed route/method/header/body mapping and response/error translation;
- bounded timeout, retry, polling, cancellation, progress, and redaction behavior; and
- MCP serialization, mapping, fake-gateway, security, and compatibility conformance.

## Non-goals

- game-loader metadata, host objects, main-thread dispatch, saves, or direct game listeners;
- game rules, state extraction, action legality, settlement, or a second game adapter;
- gateway lifecycle, process supervision, ports, leases, registry storage, or arbitrary proxying;
- model/provider calls, prompts, scoring, replay, trajectories, datasets, or artifact ownership; and
- trust based only on localhost, a tool description, a client request ID, or a successful acknowledgement.

The sixth accepted target, `sts2-protocol`, owns the shared `poc-v1` language- and transport-neutral
artifact. MCP-specific catalogs and gateway-specific routing remain local to their boundary owners.

## Wave 2 initialization status

The initialized crate contains a bounded no-I/O frame decoder/encoder seam, an exactly two-tool local
catalog, fixed GET/POST gateway mappings, a copied-artifact verifier, and an in-memory fake-gateway test
suite. The separate runtime binary opens only its configured MCP stdin/stdout and gateway TCP
connection; it does not access a game, call a provider, or own gateway lifecycle. The POC remains
source/test evidence only, while the component lane and the authorized exact-host runtime lane are
separately classified.

## `runtime-v1` process profile

The first executable MCP lane is a stdin/stdout JSON-RPC process with a real bounded TCP adapter to
the gateway. It advertises only `get_state` and `submit_action`, uses fixed gateway paths and
configured bearer/lease identity, and rejects unsupported arguments, profiles, response metadata,
and action identities. It never contacts the game listener directly.

`submit_action` admits only `show_runtime_probe`. A successful result carries a fresh observation
and `status_overlay_visible` witness; a stale result carries the stable
`sts2.game-mod/stale_generation` rejection. This is an integration probe, not a gameplay mutation.
The source/build, copied-artifact, mapping, and exact-host downstream gates are `confirmed` for the
recorded host. The action remains a probe rather than gameplay mutation, and broader compatibility
is `unverified`.

## `runtime-v2-mcp` gameplay-operation profile

The executable defaults to Runtime-v1 for backward compatibility. `STS2_RUNTIME_PROFILE=runtime-v2`
selects the additive Runtime-v2 profile; invalid values fail closed. It exposes `get_state`,
`submit_action`, and `reconcile_action`. State maps to `GET /v2/instances/{id}/state`; submission
maps to `POST /v2/instances/{id}/action`; reconciliation maps to
`GET /v2/instances/{id}/operations/{operation_id}` and has no mutation-bearing body. Submission
admits exactly the argument-free `end_turn` action and requires a bounded operation identity plus
explicit lease and generation fences. Reconciliation carries the same `operation_id` to resolve an
uncertain prior submission; it is not a retry path.

The MCP layer preserves the complete versioned envelope and exact downstream status/error origin.
Timeout or disconnect uncertainty maps to `unknown`, not a generic successful or retryable result.
Only an explicit `settled` result with a fresh post-action observation and `turn_end_settled` witness
is surfaced as settled. The deterministic fake/source seam is confirmed; live host settlement,
gameplay mutation, and end-to-end compatibility are unverified.

## `coop-native-v1-mcp` component profile

The additive `coop-native-v1-mcp` profile is selected with `STS2_RUNTIME_PROFILE=coop-native-v1`.
It exposes typed native observation, legal catalog, local action, shared vote, peer rejoin, recovery,
and response-only effect tools. The adapter sends observation to the fixed bodyless observation route
and sends producer operations to the fixed native POST routes; it has no arbitrary downstream path or
direct host access.

The profile validates the checked-in component artifact, closed envelope and provenance, configured
MCP/gateway identity, route/status pairing, and receipt/effect/observation generation, digest,
authority, and checkpoint relations. Rejected and unknown outcomes stay errors and are never retried.
This confirms the MCP consumer boundary only. A host-backed two-peer session, model participation,
deployment, and release compatibility require separate acceptance evidence.

## `game-information-query-v1-mcp` profile

ADR 0020 defines the additive game-information profile selected with
`STS2_RUNTIME_PROFILE=game-information-query-v1`. It advertises capabilities, list, search, get,
live detail, and availability reads, all with strict closed schemas and read-only/non-destructive/
idempotent annotations. It maps only to the fixed gateway capabilities GET and query POST routes.
Definitions use content-manifest identities; live detail uses a separate instance/run/epoch and
snapshot identity. Bounded text, item/page bytes, cursor, requested fields, field availability,
provenance, and typed error validation remain at the MCP boundary. It consumes merged protocol
main `34f68b18` (schema `376845b0…`). No specialized producer query is advertised until a feature
owner registers one through the accepted protocol capability set. The deterministic fake/loopback
evidence does not establish gateway #52, game-mod extraction, or host compatibility.

## `negotiated-composition-v1-mcp` profile

ADR 0021 defines the opt-in composition profile selected with
`STS2_RUNTIME_PROFILE=negotiated-composition-v1`. It advertises the surviving Runtime-v3
gameplay/map and game-information operations after unique-name, revision, producer-support,
scope, and limit negotiation. `sts2.capabilities` is an MCP-local read-only discovery call;
unsupported operations remain visible as bounded unavailable reasons and are not forwarded.
Lifecycle events invalidate tracked snapshots and require catalog refresh before forwarding;
snapshot references must be registered by the current session, unknown references fail closed, and
only explicitly admitted calls change tracking state. Standalone profiles keep forwarding an
unregistered live reference that no lifecycle event invalidated.
The source/component and copied-artifact checks do not establish producer discovery, gateway
readiness, host extraction, live snapshot freshness, provider execution, or end-to-end acceptance.

## `save-profile-v1-mcp` profile

ADR 0025 defines the additive save-profile consumer selected with
`STS2_RUNTIME_PROFILE=save-profile-v1`. It exposes bounded list/current/status reads plus explicit
select and create-disposable mutations. Calls map only to the five gateway-owned fixed routes, with
empty bodies for reads/create and exactly `profile_id` plus a two-field baseline for selection.
Instance, session, lease, epoch, profile, baseline, operation, response, and user-data identities are
typed and bounded; unknown fields, foreign targets, stale baselines, and unsupported revisions fail
closed.

The executable is capability-gated by `STS2_SAVE_PROFILE_CAPABILITY`: absent/`unsupported` advertises
no save-profile tools, `read` advertises only passive reads, and `read-write` advertises all five.
The status/receipt tool reconciles the original operation identity and never repeats a mutation.
Gateway owns allocation, authorization, leases, ledger state, and forwarding; game-mod owns save-slot
meaning and host effects. Gateway PR #53 is the source/component dependency. Launch-profile wiring,
game-mod readback, host settlement, and integrated readiness remain unverified.
