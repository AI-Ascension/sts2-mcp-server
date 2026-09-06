# ADR 0015: executable read-only co-op synchronization

- Status: Accepted for coordinated integration; exact-head CI required before merge
- Date: 2026-09-06

Complete the proposal's read-only behavior using protocol ADR 0013's consumed
`coop-synchronization-v1` response rather than importing its unused action/vote/effect
prototype. The former proposal is preserved in the original PR history. It is neither an
admitted actuation contract nor a claim of multiplayer gameplay support.

Select the profile explicitly with `STS2_RUNTIME_PROFILE=coop-synchronization-v1`.
Expose exactly `sts2.coop_synchronization`, accepting the explicit instance, MCP session,
lease, and epoch. The current generation is read from the gateway; callers need no guessed
generation to observe or recover. The fixed GET route has no body and can never become
an arbitrary proxy or peer-report mutation tool. Other profiles' catalogs and limits stay
unchanged. The retired prototype profile and wire digest are refused.

The adapter requires supplied authority to match configured gateway identity before any
network call. MCP session and gateway session remain separately bound. The executable
injects its caller and bearer credentials only after admission and validates the entire
response's canonical metadata, scope, correlation, peer set, generation, and synchronization
relations. Duplicate or unknown JSON members and payload extensions fail closed.

Every projected response retains `source: gateway_peer_reports`. Synchronization describes
recent reports submitted to the gateway by its authorized coordinator, not independently
authenticated game peers. The tool never grants mutation authority. The gateway alone owns
roster, freshness, fencing, and the separate control-scoped report ingestion route.

Required proof includes executable profile selection, foreign/missing identity rejection
before TCP, full-schema positive/negative vectors, separate MCP/gateway sessions, all status
transitions with the real gateway executable, and unchanged frozen/runtime profile gates.
