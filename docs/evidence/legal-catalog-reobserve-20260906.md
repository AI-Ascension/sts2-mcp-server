# Native legal-catalog recovery mapping

Date: 2026-09-06. Evidence: confirmed native read-only mapping; coordinator recovery is separately
covered by harness component tests. This is not a full campaign completion record.

A normal Linux campaign controller stopped after 380 settled operations with a legal-action read
failure. Its last operation was settled; no uncertain action was retried. The game remained open.
A separate read-only check used the same authorized instance and lease context through real
gateway and MCP processes. It performed:

1. A fresh observation.
2. A legal-action read with an intentionally stale state identity.
3. Verification of `isError: true` and exactly the correlated compact body:
   `correlation_id: "3"`, `error_code: "stale_generation"`, `recovery: "reobserve"`.
4. `sts2.reobserve`, then a legal-action read whose state identity and generation matched.
5. Confirmed lease release and owned process cleanup.

The check made zero provider calls and zero game dispatches. It did not alter the game state.
The harness subsequently continued separately from the preserved settled boundary; that
continuation is not an uninterrupted fresh campaign.

The earlier in-memory mapper tests did not cover the HTTP adapter's generic status classifier.
That classifier discarded the compact refusal before mapping. The adapter now admits it only
on the exact v3 legal-action GET route, using the same shape/status/correlation validator as the
mapper. Socket tests exercise both layers and reject unrelated errors, wrong correlation,
wrong route and additional private fields.

## Artifact identity

| Artifact | SHA-256 |
|---|---|
| MCP executable used by native check | `1e157d2c421719cf9a73034724e465e68c231b03315113c73fa67a58294d447e` |
| Gateway executable used by native check | `66c66172b0bfed303d5a44efb6fdadaeb49277e126c089ce28b2f4288bc02539` |
| Sanitized check summary | `bdfd57a2ea2ee77c123706d8adf77fdf429337d442056732d171b89db64b7606` |

Raw local check identifier: `1788703975`. Credentials and raw game observations are excluded.
The summary records the exact compact refusal, matched fresh catalog, zero provider calls,
zero dispatches and confirmed release.

Full locked offline workspace tests, Clippy with warnings denied, formatting, strict repository
policy and binary build passed for this implementation. These checks do not establish automatic
recovery during a normally played full campaign or victory observation.
