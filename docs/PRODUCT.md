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

The sixth accepted target, `sts2-protocol`, may own only genuinely shared language- and transport-neutral
contracts. MCP-specific catalogs and gateway-specific routing remain local to their boundary owners.

## Wave 2 initialization status

The initialized crate contains a bounded no-I/O frame decoder/encoder seam, a local capability/tool
catalog with one read-only preparation entry, a gateway adapter trait, and an in-memory fake-gateway test
suite. It does not open a listener, contact a gateway, access a game, call a provider, or implement the
final product profile. Before any public contract is accepted, document its owner, consumers, version,
provenance, mapping, security impact, deterministic fixtures, and conformance oracle. Missing runtime or
downstream evidence is `unverified`, not implied by these tests.
