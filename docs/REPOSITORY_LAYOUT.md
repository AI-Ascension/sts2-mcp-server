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

Shared naming and exception rules are normative in the aggregate
[`NAMING_CONVENTIONS.md`](../../planning/naming_conventions/NAMING_CONVENTIONS.md), with machine
readable ownership in [`naming-registry.yaml`](../../planning/naming_conventions/naming-registry.yaml).
The MCP adapter owns its mapping names but preserves standard JSON-RPC and MCP member spellings.
