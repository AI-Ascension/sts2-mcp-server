# ADR 0008: Runtime-v2 MCP gameplay-operation mapping

- Status: Accepted for the deterministic source/fake seam; live settlement remains unverified
- Date: 2026-09-02

## Context

Runtime-v1 already owns a bounded `get_state`/`submit_action` catalog and a host-probe projection.
Runtime-v2 is a separately versioned gameplay-operation contract. It requires explicit fencing and a
stable operation identity because a timeout or disconnect cannot establish whether an admitted
mutation ran.

## Decision

Add a separate `runtime-v2-mcp` catalog with exactly two tools:

- `submit_action` maps one bounded request to the fixed action route. It accepts only `end_turn`, no
  action arguments, and requires `operation_id`, instance/session/lease identity, lease epoch, and
  expected generation.
- `reconcile_action` maps one bounded request to the fixed reconciliation route. It requires the same
  `operation_id` and does not dispatch a mutation.

The mapping constructs the complete Runtime-v2 envelope using the copied artifact metadata. Gateway
responses are accepted only when their metadata, identity, kind, operation, bounded observation, and
status shape validate. The full valid envelope is retained, including exact `error_code` origin.
Timeout or disconnect uncertainty is represented as `unknown` and is surfaced as an MCP tool error;
the adapter never retries a mutation. `settled` is accepted only with a fresh post-action observation
whose generation advances beyond the request and a matching `turn_end_settled` witness. Acknowledged
admission and state reads do not imply settlement.

Runtime-v1 catalog and behavior remain unchanged. Idempotency storage, lease authority, host state,
game rules, and settlement authority remain downstream responsibilities.

## Evidence and limits

The exact `sts2-protocol/runtime-v2` artifact is copied under `protocol-artifact/runtime-v2`, with
source and conformance bytes retained locally for checksum verification. Deterministic fake-gateway
tests cover all five statuses, duplicate replay/conflict, stale fences, unknown fields, and
reconciliation. No live STS2 gameplay, host files, saves, providers, or runtime settlement are
authorized or evidenced by this decision.
