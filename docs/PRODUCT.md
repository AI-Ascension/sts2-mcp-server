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

The additive Runtime-v2 profile exposes only `submit_action` and `reconcile_action`. It admits exactly
the argument-free `end_turn` action and requires a bounded operation identity plus explicit lease and
generation fences. `reconcile_action` carries the same `operation_id` to resolve an uncertain prior
submission; it is not a retry path.

The MCP layer preserves the complete versioned envelope and exact downstream status/error origin.
Timeout or disconnect uncertainty maps to `unknown`, not a generic successful or retryable result.
Only an explicit `settled` result with a fresh post-action observation and `turn_end_settled` witness
is surfaced as settled. The deterministic fake/source seam is confirmed; live host settlement,
gameplay mutation, and end-to-end compatibility are unverified.
