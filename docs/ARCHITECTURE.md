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
it constructs the complete copied `poc-v1` action-request envelope. Gateway response bodies are
returned as bounded MCP text, preserving downstream status/error identity without reimplementing
game legality.
