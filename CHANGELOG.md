# Changelog

All notable changes are recorded here. The project follows Semantic Versioning once a release contract
exists.

## Unreleased

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
