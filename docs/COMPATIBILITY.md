# Compatibility

## Compatibility dimensions

Compatibility is tracked separately for the MCP wire/profile contract, the gateway API and mapping
contract, the Rust toolchain/runtime, operating system and architecture, configuration, and package
contents. This adapter does not claim compatibility with a game host merely because another component
does.

## Current baseline

The target contains the local no-I/O package, a runtime MCP process, and fake-gateway tests. The
`runtime-v1-mcp` profile is accepted for this bounded sprint and consumes the protocol owner's
release-like runtime artifact copy. A controlled component lane exercises the real MCP process and
gateway adapter with a synthetic downstream, and an authorized host lane exercises the exact
packaged downstream. Provider behavior and public release remain `unverified`.

| Subject | Current identity | Evidence |
| --- | --- | --- |
| Target package | sts2-mcp-server 0.0.0; local revision wave2-local-v0 | source-derived; not an external compatibility claim |
| Repository policy | target-local Rust repo-policy, version 0.0.0 | source-derived after local checks |
| Rust toolchain | `1.97.1`, edition 2024 | declared; compilation is a local gate |
| MCP protocol revision | `2025-06-18` for the POC handshake | statically pinned; live transport compatibility remains unverified |
| POC protocol artifact | `sts2-protocol/poc-v1`; schema digest `242b8f9233e915a55ea8d2e72ca476c1258169a67e62de72ee5aed848a6a0a19` | copied artifact/schema/checksum tests; protocol head `cad3c85d` |
| Gateway API revision | not frozen | unverified; consume an approved versioned description |
| Game host/loader | outside this target | not applicable to adapter source; owned by game-mod |
| Harness/model/provider | outside this target | not applicable to adapter authority |

## Versioning rules

An internal refactor with identical observable behavior is a patch-level implementation change. An
additive tool, field, or route is minor only after capability advertisement, mapping, fixtures, and
compatibility review. A renamed, removed, or semantically changed MCP or gateway contract is breaking or
must use a separately versioned profile/path with a migration window.

Do not infer the MCP revision from the game version, or the gateway version from the MCP server version.
Keep MCP session, gateway session, game instance, MCP request, gateway request, and host/action IDs in
separate namespaces with explicit mappings and restart/reuse rules.

## Evidence levels

- `build-only`: target-local source and governance tooling compile and pass deterministic checks;
- `transport`: a pinned MCP client exercises a disposable server transport;
- `gateway`: a disposable gateway double proves allowlists, auth, mapping, errors, and lifecycle signals;
- `host`: game-mod proves its own host boundary; this target does not establish it;
- `end-to-end`: the complete client-to-game path proves effect settlement and fresh observation.

Build-only and controlled component/gateway levels are available for this sprint's bounded lane. A
parse, build, handshake, or tool acknowledgement must not be reported as downstream host readiness
or game-effect compatibility. The real component trace does not promote the managed mod or game
host to a supported compatibility row.

## Runtime profile row

| MCP profile | Gateway path | Current evidence | Result |
| --- | --- | --- | --- |
| `runtime-v1-mcp` | Fixed single-instance runtime adapter | Mapping/artifact tests, component TCP lane, and authorized exact-host trace | Bounded adapter path confirmed for STS2 v0.107.1 Windows x86-64; gameplay and broader compatibility unverified |
| `runtime-v2-mcp` | `GET /v2/instances/{id}/state`, `POST /v2/instances/{id}/action`, `GET /v2/instances/{id}/operations/{operation_id}` | Copied-artifact checksum, deterministic mapping/projection tests, profile and identity unit tests | Source/fake seam confirmed; live gateway, host settlement, gameplay mutation, and end-to-end compatibility unverified |

The profile is compatible only with the exact `runtime-v1` schema digest and allowlisted response
shapes. It makes no provider, game-rule, gameplay mutation, or release-support claim.

Runtime-v2 consumes the exact handed-off schema digest
`f7963b19c8ed5bbdc02c08e83c7a2e16c4771ed5eb798b29a8208d7a917a86c2`. Its MCP mapping is a thin
adapter: it does not own idempotency, lease authority, host state, or settlement inference. A gateway
timeout or disconnect is an `unknown` operation outcome and requires reconciliation with the same
`operation_id`; it is never automatically resubmitted.
Malformed operation receipts likewise retain an `unknown` outcome until reconciliation. Failed
state reads report an ordinary tool error with no synthetic operation or observation. The MCP
operation-ID schema excludes `/`, which the current fixed reconciliation path cannot represent;
other shared identity fields retain their existing syntax. Existing slash-containing operation IDs
cannot be reconciled through this adapter and require owner-side investigation, not resubmission.
The process defaults to `runtime-v1`; `STS2_RUNTIME_PROFILE=runtime-v2` selects Runtime-v2 and any
other value fails closed. Runtime-v2 supplied instance/session/lease/epoch fields must match the
configured gateway identity before forwarding; Runtime-v1 retains its compatibility injection path.

Runtime-v2 separates the tool argument `mcp_session_id` from the gateway envelope `session_id`.
`STS2_MCP_SESSION_ID` configures the expected MCP session; when absent it defaults to
`STS2_SESSION_ID` for existing same-session setups. `STS2_SESSION_ID` always supplies the gateway
session. Foreign MCP sessions fail before forwarding, while MCP correlation headers retain the MCP
identity and gateway authority headers retain the gateway identity. Pure unbound library constructors
keep their historical same-session mapping; executable configuration uses the explicitly bound API.

The executable gateway address must be a numeric loopback socket address with a nonzero port
(for example `127.0.0.1:15525` or `[::1]:15525`); DNS names and remote endpoints are unsupported.
Connect, request writes, and response reads share a five-second total exchange deadline, with connect
additionally limited to two seconds. Responses require HTTP/1.1, a final 200–599 status, one decimal
Content-Length, and `application/json` (optionally UTF-8 charset); duplicate headers, transfer/content
encodings, and headers exceeding 8 KiB including the terminator fail closed. Bodies remain capped at
64 KiB. These are source and loopback-test guarantees, not downstream readiness evidence.

The Runtime-v2 mapping carries the caller's instance/session/lease/epoch as explicit authority
headers even for bodyless reconciliation. Before connecting, the executable rejects mismatches with
its configured gateway authority instead of silently replacing them. Runtime-v1 keeps its documented
configured-identity injection, but Runtime-v1 and Runtime-v2 response envelopes must match the actual
configured identity, request correlation, and route-specific result kind before projection.
