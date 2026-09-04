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
operation-ID schema excludes `/`, which the current fixed reconciliation path cannot represent,
and the mapping also rejects the bare segments `.` and `..` before dispatch so no dot segment reaches
the route; other shared identity fields retain their existing syntax. Existing slash-containing operation IDs
cannot be reconciled through this adapter and require owner-side investigation, not resubmission.
The process defaults to `runtime-v1`; `STS2_RUNTIME_PROFILE=runtime-v2` selects Runtime-v2 and any
other value fails closed. Runtime-v2 supplied instance/session/lease/epoch fields must match the
configured gateway identity before forwarding; Runtime-v1 retains its compatibility injection path.

Runtime-v2 separates the tool argument `mcp_session_id` from the gateway envelope `session_id`.
`STS2_MCP_SESSION_ID` configures the expected MCP session; when absent it defaults to
`mcp-session-1`, matching the harness and gateway composition defaults. Existing same-session setups
must explicitly set `STS2_MCP_SESSION_ID` to their gateway session. `STS2_SESSION_ID` supplies the gateway
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

## Complete frozen artifact inventories

The copied POC and Runtime-v1 bundles include the canonical README, checksum inventory,
conformance cases, schema sources and golden vectors from protocol commit
`11e4252e39a77f0017b8e4f3720590e6162e8f53`. Existing schema, manifest and golden wire bytes
are unchanged; the packaging correction restores missing inventory entries. CI checks every
checksum without ignoring missing files. These are inert MIT contract data, not protocol
implementation dependencies or new host evidence. MCP continues to own framing, tool validation
and fixed gateway mapping; the game-mod and host retain authoritative game state and effects.

## Process session and denial mapping

Executable requests must carry the configured MCP session in both correlation metadata and the
`x-mcp-session-id` header. A gateway HTTP 403 response maps to the sanitized MCP scope error
`-32007`; it is a known authorization denial, not an unknown operation requiring reconciliation.
The copied Runtime-v2 bytes and generation/settlement validation remain unchanged.

[ADR 0010](decisions/0010-runtime-v2-process-session-and-scope-errors.md) records the public
`GatewayError::Forbidden` enum addition and its exhaustive-match compatibility consequence.

Runtime-v2 HTTP 429 guidance preserves only bounded `error_code`, `retryable: true`, and
`retry_after_ms` between zero and 60,000 milliseconds. Invalid guidance fails closed and private
fields are omitted. The adapter never automatically redispatches; synthetic tests cover valid and
out-of-range delays. Gateway support for this guidance is an independent consumer integration gate.

## Earlier Runtime-v3 gameplay proposal

`STS2_RUNTIME_PROFILE=runtime-v3-gameplay` selects the three-tool bounded `play_card` profile.
It consumes protocol PR #7's `c961bbde893f0422f80233d14ea9ae8b648ee9032136e5370aa5f6b949f6575e`
schema, copied from `11a7979f7368c78c10924337228991d16c9ec92a`. Its intended gateway dependency is
gateway PR #6. This is not compatible with the broader same-named protocol PR #8 / gateway PR #7 /
MCP PR #8 proposal. Reconcile the competing profiles before selecting an integrated merge order.

Reconciliation validates the stored action and matching witness under the requested identity and
operation fence, without inventing card index zero or a null target. Submission responses still
must echo the submitted action exactly. A failed reconciliation read returns a tool error preserving
uncertainty, not a fabricated operation receipt. Canonical artifact/schema/golden and fake-mapping
tests establish source-level contract agreement only; live gameplay remains unverified.
