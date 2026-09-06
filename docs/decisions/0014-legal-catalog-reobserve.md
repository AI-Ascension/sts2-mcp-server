# Legal catalog reobserve mapping

Status: source implementation; coordinated harness validation pending.

The host can advance between observation and the separate legal-action request. Its
HTTP refusal has exactly `correlation_id`, `error_code`, and `recovery`. The gateway
already preserves the reviewed correlated refusal. MCP previously reduced it to a
generic invalid-envelope error, losing the instruction to obtain a fresh observation.

For legal-action responses only, MCP preserves that compact body as an error result
when correlation matches, recovery is `reobserve`, and status/code are exactly 409 with
`stale_generation`, or 503 with `host_not_configured` or `host_observation_unavailable`.
The body is bounded to 1,024 bytes and rejects extra fields. Other routes, success
statuses, unknown codes, and mismatched correlation keep the existing fail-closed path.

This is a read refusal, never a legal-action catalog or a mutation receipt. The harness
owns bounded reobservation and must obtain matching fresh state and actions before
calling a provider or dispatching. MCP does not retry requests or infer host readiness.

Recording-gateway tests cover each accepted status/code and reject wrong routes,
status, correlation, codes, and extra fields without exposing the supplied private text.
These tests are component evidence; live stale-read recovery remains separate proof.
