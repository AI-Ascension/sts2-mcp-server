# Repository layout

## Current foundation tree

```text
.
├── crates/mcp-server/       target-owned MCP framing/catalog/mapping crate
├── protocol-artifact/poc-v1 offline release-like artifact copy used by the POC mapping
├── schemas/mcp/             reserved MCP schema location; no accepted schema yet
├── conformance/             reserved implementation-neutral fixture location
├── tests/                   reserved product/component test location
├── tools/repo-policy/       target-local Rust governance tool
├── docs/                    standards, product boundary, and decisions
├── .github/                 bounded read-only automation and dependency updates
└── Cargo.toml               workspace containing MCP seam and repo-policy
```

The schemas/mcp, conformance, and root tests directories remain reserved for owner-local contract
artifacts. The MCP crate is non-empty and has focused framing and POC mapping tests; no placeholder
crate was added elsewhere.

## Responsibility map

| Area | Owner | Allowed concern |
| --- | --- | --- |
| crates/mcp-server | this target | bounded MCP framing, catalog, and gateway mapping seam |
| `schemas/mcp` | this target | approved MCP wire/tool schemas |
| `conformance` | this target | MCP/mapping behavior once contracts exist |
| `tools/repo-policy` | this target | repository governance only |
| `sts2-protocol` | sixth target | shared language-/transport-neutral contracts only |
| `sts2-game-core` | core target | host-independent domain meaning |
| `sts2-game-mod` | mod target | host boundary and authoritative game HTTP |
| `sts2-gateway` | gateway target | lifecycle, leases, routing, auth, registry |
| `sts2-harness` | harness target | coordination, models/providers, trajectories, artifacts |

## Dependency and runtime rules

Runtime communication is client/harness → MCP server → gateway → isolated game-mod → host. Compile-time
dependencies must follow accepted contract ownership and must not bypass a process boundary. The MCP
server may consume versioned gateway-interface descriptions and accepted shared protocol contracts; it
must not depend on gateway registry internals, game-mod/host implementation, or harness crates.

Every future module gets one responsibility, one identified consumer, and a build/test purpose. Generated
output, proprietary files, saves, credentials, and machine-specific paths are not repository contents.

## Naming authority

Shared naming and exception rules are normative in the aggregate NAMING_CONVENTIONS.md, with machine
readable ownership in naming-registry.yaml.
The MCP adapter owns its mapping names but preserves standard JSON-RPC and MCP member spellings.

## Runtime-map additions

```text
protocol-artifact/runtime-map-v1/             copied manifest, schema, goldens, and checksums
schemas/runtime-map-v1.schema.json            source-path schema companion
conformance/cases/runtime-map-v1.json         implementation-neutral consumer case
crates/mcp-server/src/catalog_runtime_map.rs  additive seven-tool catalog
crates/mcp-server/src/mapping_runtime_map.rs  fixed route and argument mapping
crates/mcp-server/src/projection_runtime_map* complete graph/result validation
```

## Game-information query additions

```text
protocol-artifact/game-information-query-v1/  pinned manifest, schema, goldens, and checksums
schemas/game-information-query-v1.schema.json source-path schema companion
conformance/cases/game-information-query-v1.json implementation-neutral consumer case
crates/mcp-server/src/catalog_game_information.rs strict six-tool descriptors
crates/mcp-server/src/mapping_game_information*.rs typed fixed-route request/response seam
```

The protocol owner remains responsible for the neutral model and conformance meaning. MCP owns
framing, catalog selection, session binding, fixed gateway mapping, and safe projection; it does
not import game-mod or visualizer implementation.

## Save-profile additions

```text
crates/mcp-server/src/catalog_save_profile.rs             additive five-tool catalog and schemas
crates/mcp-server/src/mapping_save_profile.rs             fixed gateway route/body mapping
crates/mcp-server/src/mapping_save_profile_response*.rs   bounded receipt/error validation
crates/mcp-server/tests/save_profile_mapping.rs           fake-gateway and negative consumer conformance
docs/decisions/0025-save-profile-mcp-tools.md             owner/compatibility decision
```

`gateway-save-profile-v1` remains a gateway-local contract; no game-mod implementation, save path,
launch command, or copied host logic is stored here. The executable profile is capability-gated and
unsupported owners advertise no usable save-profile descriptors.

## Watchdog recovery sideband additions

```text
crates/mcp-server/src/recovery_frame*.rs          closed frame envelope, scalars, and validation
crates/mcp-server/src/catalog_watchdog_recovery.rs ordered nine-tool sideband catalog
crates/mcp-server/src/mapping_watchdog_recovery*.rs two wired fixed-route reads and their tests
crates/mcp-server/src/bin/runtime_support/binding_recovery*.rs executable route/header admission
docs/decisions/0027-watchdog-recovery-mcp-sideband.md      owner/compatibility decision
```

The durable boot and dispatch routes stay with the runtime owner; this sideband stores no lease,
fence, intent, or operation record, and it surfaces a gateway recovery frame verbatim rather than
projecting it.
