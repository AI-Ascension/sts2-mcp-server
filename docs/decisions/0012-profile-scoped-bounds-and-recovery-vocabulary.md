# ADR 0012: profile-scoped byte limits and recovery vocabulary ownership

- Status: accepted by the repository review orchestration decision (2026-09-05)
- Date: 2026-09-05

## Context

PR #8 added the Runtime-v3 semantic profile and, to carry its larger envelopes, raised three global
limits for every profile: the MCP frame from 16 KiB to 256 KiB, the executable gateway response body
from 64 KiB to 128 KiB, and the projected tool content from 16 KiB to 128 KiB. The review recorded
this as an unscoped compatibility change for poc/runtime-v1/runtime-v2 consumers (finding P8-1). The
same profile exposes `release_lease` and `stop_episode` through `sts2.recover`, which name lifecycle
that the gateway (leases) and the harness (episodes) own (finding P8-5).

## Decision

Byte limits are profile properties. The catalog reports its frame limit through the public
`ToolCatalog::max_frame_bytes`; the library frame decoder and the executable stdin reader both use
it. The executable selects the gateway response body limit together with the catalog
(`RuntimeProfile`), and the projection keeps separate legacy and Runtime-v3 content limits. The
poc, runtime-v1, and runtime-v2 profiles return to their historical 16 KiB / 64 KiB / 16 KiB limits;
only `runtime-v3-gameplay-mcp` accepts 256 KiB / 128 KiB / 128 KiB. `MAX_FRAME_BYTES` remains the
public absolute ceiling. The 16 KiB gateway request body and 8 KiB header limits are unchanged.

`sts2.recover` keeps `release_lease` and `stop_episode` callable. The adapter exposes the recovery
vocabulary and checks its shape, but every kind is forwarded to the one fixed route
`POST /v3/instances/{id}/recover` with no other side effect; the gateway authorizes the request with
the `control` scope and decides whether the recovery happens. The adapter constructs no lifecycle
route, holds no lease or episode state, and reports a scope denial as the typed `-32007` error.

## Consequences and evidence

The only public API addition is `ToolCatalog::max_frame_bytes`; wire schemas, frozen artifacts,
routes, tool catalogs, and identity admission are unchanged. Consumers that relied on the interim,
unreleased global limits on a legacy profile must select Runtime-v3 or stay within the historical
limits. `tests/profile_frame_bounds.rs`, the executable HTTP and profile unit tests, and
`tests/runtime_v3_gameplay_regressions/recovery_ownership.rs` are deterministic source-level
evidence; live gateway authorization, lease release, and episode stop remain unverified here.
