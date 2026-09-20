# Legal catalog reobserve mapping

Status: implemented; coordinated harness component checks and native read-only mapping verified.

The HTTP adapter must preserve this refusal before generic status classification. Both layers
share one body validator, and the adapter independently requires the exact v3 legal-action GET
route. Socket tests exercise the real HTTP adapter through MCP mapping, including wrong route,
correlation, extra fields and unrelated error rejection.

The [native read-only check](../evidence/legal-catalog-reobserve-20260906.md) verified the compact
refusal, reobservation and matching fresh catalog through real gateway and MCP processes.

The host can advance between observation and the separate legal-action request. Its
HTTP refusal has exactly `correlation_id`, `error_code`, and `recovery`. The gateway
already preserves the reviewed correlated refusal. MCP previously reduced it to a
generic invalid-envelope error, losing the instruction to obtain a fresh observation.

For legal-action responses only, MCP preserves that compact body as an error result
when correlation matches, recovery is `reobserve`, and status/code are exactly 409 with
`stale_generation`, or 503 with `host_not_configured`, `host_observation_unavailable`, or a
refused-launch-contract code. A refused launch contract is a distinct failure from a lane that never
declared one, and it arrives as the game-mod's own refusal prefix `launch_contract_refused` either
alone or followed by `_` and one reason token of 1 to 64 ASCII alphanumerics, `_` or `-` — the exact
vocabulary the producer composes, so a string the producer cannot emit is refused here rather than
admitted as a neighbouring code (`AI-Ascension/sts2-gateway#85`). The body is bounded to 1,024 bytes
and rejects extra fields. Other routes, success statuses, unknown codes, and mismatched correlation
keep the existing fail-closed path.

This is a read refusal, never a legal-action catalog or a mutation receipt. The harness
owns bounded reobservation and must obtain matching fresh state and actions before
calling a provider or dispatching. MCP does not retry requests or infer host readiness.

Recording-gateway tests cover each accepted status/code and reject wrong routes,
status, correlation, codes, and extra fields without exposing the supplied private text.
These tests are component evidence; live stale-read recovery remains separate proof.
