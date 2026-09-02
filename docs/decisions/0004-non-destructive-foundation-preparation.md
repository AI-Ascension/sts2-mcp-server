# ADR 0004: Non-destructive foundation preparation

- Status: Accepted for Wave 1 preparation; MCP crate reservation superseded by ADR 0005
- Date: 2026-09-02

## Context

The target began as an uninitialized scaffold containing responsibility directories and a README. Wave 1
needs reproducible governance before product implementation, while the aggregate workspace, reference
trees, game files, and user state remain outside this target's write scope.

## Decision

Add only target-local governance, documentation, workflows, configuration, lockfile, and Rust policy
tooling during Wave 1. Keep `schemas/mcp`, `conformance`, and root `tests` reserved until an approved
product contract supplies a real consumer and non-empty source/test seam. The later Wave 2 MCP
initialization is governed by ADR 0005. Do not initialize Git, commit, publish, deploy, install, launch,
call providers, access proprietary game files, or mutate profiles/saves as part of these preparation
waves.

## Consequences

Foundation checks can run locally and in read-only CI without implying product readiness. The target's
status remains preparation/build-only and runtime-unverified. The next owner must initialize product code
only after the protocol profile, gateway mapping, and conformance requirements are accepted.
