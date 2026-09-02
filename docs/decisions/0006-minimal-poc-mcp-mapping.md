# ADR 0006: Minimal POC MCP mapping

- Status: Accepted for the deterministic POC
- Date: 2026-09-02

## Context

The MCP target is the thin client-facing boundary in a fake six-target path. It must expose only the
requested state read and typed action while preserving the protocol artifact's lineage and leaving
game legality and lifecycle authority downstream.

## Decision

Consume a checked-in copy of `sts2-protocol/poc-v1` and advertise exactly two tools: `get_state` and
`submit_action`. Map them to fixed gateway paths, GET for state and POST for action. Require explicit
instance/session context, generation, the `use_budget` action ID, and units in the action tool. Build
the full action-request envelope from the copied artifact metadata and return bounded downstream body
text so stable core error identity is preserved.

Use the existing custom JSON and fake gateway seams. No network, credential, game file, protocol
implementation dependency, or arbitrary downstream path is added.

## Consequences

The catalog and mapping are reviewable and deterministic for the requested fake path. The tests do not
prove MCP client interoperability, gateway readiness, host behavior, action legality, or effect
settlement; those remain unverified at this boundary.
