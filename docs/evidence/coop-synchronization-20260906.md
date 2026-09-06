# Executable co-op synchronization evidence, 2026-09-06

Confirmed scope: read-only coordinator-reported synchronization across the actual MCP and
gateway executables. This is not native multiplayer gameplay, independent peer authentication,
or a host mutation test. No game, model provider, profile, or deployed Train service was used.

## Admission and implementation

Protocol ADR 0013 replaces the unadmitted six-family prototype with the complete consumed
`coop-synchronization-v1` artifact. The old source remains in the PR's Git ancestry.
Gateway `runtime_support/service_coop.rs` serializes the full response from `coop_reports.rs`;
MCP's explicit executable profile routes through `mapping_coop_synchronization.rs` and
`projection_coop_synchronization.rs`. Both use schema digest
`d410858cabbd38612345120c2196423130c7b21d788fd2b0d775cd82887087ec`.

The gateway reports the configured roster's recent coordinator reports, with thirty-second
expiry, monotonic convergence and exact lease fencing. The response always labels its
source `gateway_peer_reports`. MCP exposes no report, action, vote or shared-effect tool.
No synchronized result is used by the gateway as a gameplay forwarding authorization.

## Verification

The following passed in separate owner worktrees and Cargo target directories:

- Protocol: 56 tests; gateway: 125 tests; MCP: 114 default tests, zero failures.
- Format, warnings-denied Clippy, locked/offline workspace/all-target/all-feature tests,
  metadata, strict repository policy, and whitespace checks in all three repositories.
- All five artifact inventories per repository. The new artifact has eight exact entries,
  including source schema and conformance copies; all 27 vectors pass their declared oracles.
- Existing POC and Runtime-v1/v2/v3 artifacts remain byte-identical to current main.

MCP's cross-executable test is explicitly ignored by the single-repository default suite
because it requires the reviewed gateway binary. It was separately invoked and passed:

```sh
STS2_COOP_GATEWAY_BINARY=/path/to/reviewed/sts2-gateway-runtime \
  cargo test --locked --offline --package sts2-mcp-server \
  --test coop_gateway_runtime -- --ignored --nocapture
```

The test starts both real executables with cleared environments and disposable credentials.
It allocates the real gateway lease, initializes MCP, checks the single-tool catalog, and
reads through startup missing state, partial reports, generation-4 convergence, generation
disagreement, explicit disconnect, and generation-5 recovery. The MCP reader credential is
separate from the coordinator control credential. Read-only report submission returns 403;
stale epoch, unknown peer and regressing reports return 409. After lease release, MCP's read
returns a tool error. A listening downstream trap receives zero game connections. Both
owned child processes are joined during test cleanup.

| Tested executable | SHA-256 |
| --- | --- |
| Gateway debug executable | `f6fb4ae99728c2b170bfb69f73f62065d61fac0809ae4a1ef661770c0b38efba` |
| MCP debug executable | `9d96adfc501f8fea225c51e4823147c1d3dfec0487ab0322f66d659948399171` |

Exact submitted heads and CI outcomes belong to the coordinated protocol, gateway and MCP
PR records. A merge requires all owner checks green; this component evidence does not imply
deployment, release publication, or completion of the preserved actuation prototype.
