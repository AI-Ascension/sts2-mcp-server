# ADR 0002: Narrow scope for the accepted sixth protocol target

- Status: Accepted by the current build-completion plan
- Date: 2026-09-02

## Context

The current build-completion plan includes `sts2-protocol` as the sixth accepted target. Earlier layout
planning treated it as a decision-stage candidate, so the distinction must remain explicit. A shared
repository is useful only if it owns contracts that are genuinely reused across independent consumers;
it must not become a dumping ground for MCP, gateway, host, or harness behavior.

## Decision

`sts2-protocol` may own only approved language- and transport-neutral shared contracts, with a named owner,
at least two independent consumers, explicit versioning, provenance/license records, and an
implementation-neutral conformance oracle. It must not own game rules, host objects, gateway lifecycle or
routing, MCP framing/tool catalogs, model/provider behavior, or harness artifacts.

This MCP target remains the owner of MCP wire/tool schemas and mappings. It may consume a released or
otherwise approved shared contract, but it must not define boundary-specific content in the protocol
target or assume a shared type is normative without an explicit mapping and version.

## Consequences

The six-target build order can initialize a protocol repository without widening this adapter's authority.
Every shared field, state, identifier, error, or timing contract has one source of truth and an explicit
consumer mapping. A proposed contract without an owner, consumer, version, provenance, or test oracle is
blocked rather than invented.
