# ADR 0025: bounded save-profile MCP tools

- Status: Proposed MCP consumer contract; gateway source/component contract is merged in gateway PR
  #53, while game-mod launch-profile integration remains an external gate
- Date: 2026-09-13
- Owner: `sts2-mcp-server`
- Downstream contract: `gateway-save-profile-v1` (gateway-local, not a shared protocol artifact)

## Context

Issue #50 needs agent-visible save-profile discovery and controlled selection without exposing a
filesystem path, process command, URL, arbitrary gateway route, or host object. Gateway PR #53
provides the fixed, authenticated, leased contract. The game-mod remains the owner of save-slot
meaning, baseline calculation, and host effects; this repository owns only MCP framing, descriptors,
validation, and mapping.

The launch-profile owner for the disposable allocation is not merged on this branch. The consumer
therefore treats its opaque descriptor and `gateway-launch-profile-v1` provenance as an external
contract and does not import game-mod code or infer a path from a profile ID.

## Decision

The additive `save-profile-v1-mcp` catalog contains exactly these tools:

| Tool | Permission | Gateway route | Body |
| --- | --- | --- | --- |
| `sts2.save_profile_list` | read | `GET /v1/instances/{id}/save-profiles` | empty |
| `sts2.save_profile_current` | read | `GET /v1/instances/{id}/save-profile/current` | empty |
| `sts2.save_profile_select` | mutate | `POST /v1/instances/{id}/save-profile/select` | exactly `profile_id` and `baseline` |
| `sts2.save_profile_create_disposable` | mutate | `POST /v1/instances/{id}/save-profile/create-disposable` | empty object |
| `sts2.save_profile_status` | read | `GET /v1/instances/{id}/save-profile/operations/{operation_id}` | empty |

The status tool is also the receipt/reconciliation surface. It accepts one path-safe operation
identity and never resubmits a mutation. Selection and disposable creation are not advertised as
discovery or read-only tools. No delete, reset, import, export, unlock-all, shell, path, URL, or
direct mod tool exists.

Every descriptor uses a closed object schema and requires `instance_id`, `mcp_session_id`, `lease_id`,
and a nonnegative, JavaScript-safe `lease_epoch`. Instance and operation path segments, session and
lease headers, profile identities, error codes, and guidance are bounded to 128 bytes with explicit
alphabets. Selection requires a two-field baseline (`identity` and a 64-byte lowercase digest)
before a gateway request is built. The request body is capped at 16 KiB.

The MCP session and gateway session remain independent. Mapping places the MCP namespace in
`x-mcp-session-id`/correlation metadata and the gateway authority in explicit `x-sts2-*` fields;
the executable adds its configured caller and correlation headers. A mutating operation uses the
MCP request identity as the gateway operation identity. An uncertain mutation returns `unknown`
with that identity and bounded reconciliation guidance. The status tool takes that same identity
explicitly, so a lost response cannot trigger a second mutation.

Responses must carry `gateway-save-profile-v1`, a matching operation identity, an admitted route
when the gateway ledger supplies one, and one of `accepted`, `settled`, `rejected`, `unknown`,
`blocked`, `cancelled`, `pending`, `created`, or `selected`. An optional `schema_revision` is
accepted only when it equals the gateway contract revision. Settled selection requires the requested
profile and an authoritative baseline; settled disposable creation requires a gateway-owned
user-data descriptor, matching provenance, and an authoritative baseline. Bare gateway errors are
accepted only for bounded `error_code`/retry guidance and are normalized without forwarding private
fields. Denied, stale, unavailable, accepted, settled, and unknown outcomes stay distinguishable.

The executable selects this profile only with `STS2_RUNTIME_PROFILE=save-profile-v1`. Capability
publication is fail-closed: absent `STS2_SAVE_PROFILE_CAPABILITY` (or its plural alias) means
unsupported and advertises no tools; `read` advertises list/current/status; `read-write` advertises
all five. The initialize response reports contract, profile revision, and read/mutate support.
Unknown capability values fail process configuration rather than widening authority.

## Compatibility and ownership

This is an additive MCP profile; all existing catalogs, pins, routes, and legacy bounds remain
unchanged. Gateway route, scope, lease, identity, body, response, and retention changes require a
new contract/profile review. Gateway owns authentication, lifecycle, leases, allocation identity,
ledger state, and fixed downstream forwarding. Game-mod owns save-slot enumeration/current/selection,
baseline semantics, launch-profile binding, and host effects. MCP clients and the harness consume the
bounded surface but do not become authorities.

## Deterministic evidence and external gate

`tests/save_profile_mapping.rs` proves the exact five descriptors, annotations, initialize capability
metadata, fixed method/path/body/header mapping, closed input validation, stale and contract errors,
read-only refusal, and unknown-operation reconciliation without mutation replay. Executable binding
tests prove the route allowlist, explicit authority requirement, foreign-target rejection, no legacy
body injection, and response contract fence.

These checks are source/component and synthetic fake-gateway evidence. The merged gateway PR #53,
the not-yet-merged game-mod launch-profile/save-slot owner, real gateway readiness, host settlement,
production persistence, cross-restart durability, provider participation, deployment, and release
compatibility remain `unverified`. No game, save, profile, or host process is used by this profile.
