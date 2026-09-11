# Testing and evidence

## Test layers

| Layer | Purpose | Current state |
| --- | --- | --- |
| Unit | policy parsing, diagnostics, framing, validation, and pure mapping decisions | checker and MCP unit tests are present |
| Protocol | exact MCP serialization and copied POC mapping | two-tool local fixtures are present |
| Component | bounded transport, mapping, auth, timeout, cancellation, fake gateway | fake-gateway mapping and real stdio notification tests are present; cancellation effects are not established |
| Integration | real disposable process/socket composition | documented Runtime-v1 component evidence is separate from local unit tests; new runs require authorization |
| Host | game-mod/host load and effect behavior | owned by other targets |
| Release smoke | exact package bytes in a clean environment | not started |

## Foundation commands

Run from this target root:

```bash
cargo metadata --locked --offline --no-deps --format-version 1
for profile in poc-v1 runtime-v1 runtime-v2 runtime-v3-gameplay runtime-v4-expert runtime-v4-expert-action runtime-v4-expert-rest-action runtime-map-v1 coop-synchronization-v1 coop-receipt-query-v1 seeded-run-v1 coop-native-v1; do
  (cd "protocol-artifact/$profile" && sha256sum --check SHA256SUMS)
done
cargo test --locked --offline --package sts2-mcp-server --test artifact
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --all-features --locked
cargo run --locked --package repo-policy -- --strict
```

These commands validate the local MCP seam, copied artifact identity/checksums/schema fixtures, exactly two tool descriptors,
fixed GET/POST fake-gateway mappings, policy tool, and repository structure. They do not establish a
live MCP transport, gateway readiness, authentication, game compatibility, model/provider behavior,
or end-to-end action settlement.

## Future product tests

Before implementation, freeze the MCP revision, profile, gateway API description, route allowlist, error
mapping, identifier ledger, and bounded resource limits. Then add readable golden fixtures for every
advertised method, capability, tool, argument, result, error, and mapping. Use a deterministic fake
gateway for malformed input, unknown tool/route, session and instance mismatch, auth failure, oversized
content, downstream errors, retryability, timeout, cancellation before/after forwarding, reconnect, and
shutdown.

Every accepted operation must resolve to success, explicit rejection, or explicit cancellation. A client
timeout or acknowledgement is not evidence that the downstream game effect completed. Host or end-to-end
claims require exact versions, hashes, disposable data, setup, requests, observations, cleanup, and the
evidence level.

## Evidence language

Use `confirmed` only for controlled reproduced behavior, `source-derived` for exact source/config
facts, `inferred` for reasoned consequences, `proposed` for new decisions, and `unverified` when an
external precondition or runtime lane is absent. A skipped check remains visible and is not a pass.

## Runtime profile checks

`runtime_v1_shape.rs` exercises every required response field, closed root/nested objects, state
null fields, accepted/rejected result combinations, and matching witness/envelope generations.
Its three MIT response fixtures are copied from `sts2-protocol` commit
`40bdfc30cedcc11eea001ad28f4a6e58c788f98a`, `artifacts/runtime-v1/golden/` (schema digest
`a76086d7a68668fd4cff53999369d2b450b0d6623827393882f458f2aa1f93eb`). Their exact legacy MCP
projections remain unchanged. These are synthetic protocol-owner goldens, not host-effect evidence.

`runtime_v2_uncertainty.rs` checks that invalid action receipts remain unknown without retry or
untrusted payload leakage; state-read failures report missing observation rather than inventing an
operation. Operation IDs containing `/`, and the bare segments `.` and `..`, are rejected before
submission, because they cannot be looked up as one plain segment of the current gateway
reconciliation route.

`runtime_v2_sessions.rs` checks distinct configured MCP/gateway sessions for state, submission and
reconciliation, rejects foreign MCP sessions before forwarding, and exercises the actual executable
against a disposable loopback HTTP peer. It verifies both outbound header namespaces and the gateway
envelope, not only library construction. No host process or provider participates.

`json_notifications.rs` checks raw Unicode and surrogate-pair round trips, duplicate-key and
leading-zero rejection, and a 64-value nesting limit before recursive parsing. It also exercises the
actual binary's stdin/stdout to prove notifications produce no output, not even a blank line. The
integer-only boundary remains intentional; this is not general-purpose JSON-number support.
Notifications do not dispatch request-only tools. The synchronous transport still does not interrupt
an in-flight gateway call on cancellation; notification silence is not evidence of cancellation.

`runtime_mapping.rs` confirms the two runtime tool calls, fixed routes, complete action envelope,
effect-witness projection, and structured stale-generation preservation. `runtime_artifact.rs`
confirms the copied manifest and schema bytes. The runtime binary has bounded stdin frame and HTTP
adapter paths and builds with the pinned Rust toolchain.

A controlled component lane may run this process against the real gateway and a synthetic downstream;
that confirms MCP/gateway transport and mapping only. It does not prove the managed mod, Godot
main-thread execution, STS2 host compatibility, a disposable game profile, or gameplay mutation.

The executable HTTP unit tests use ephemeral loopback listeners, not a live gateway or game. They
cover slow-drip headers/bodies against one total deadline, an unread request writer, expired deadlines,
strict JSON content type and Content-Length framing, duplicate headers, unsupported encodings,
truncation, header-size limits, and outbound header injection. Configuration tests reject DNS names,
non-loopback addresses, port zero, and control/non-ASCII token bytes.

The executable binding tests reject missing/mismatched bodyless authority before any connection and
reject foreign response instance/session/lease/epoch, wrong correlation, or wrong route-specific kind.
The Runtime-v2 mapping regression verifies that reconciliation preserves all four authority headers
without adding a mutation body.

The executable binding tests reject missing or foreign MCP-session headers and correlation before
connecting, while accepting distinct configured MCP and gateway sessions. A disposable loopback
HTTP 403 peer verifies typed scope-denial classification without forwarding private denial details.
These are source/component checks and establish no host settlement or provider evidence.

Runtime-v2 HTTP 429 guidance preserves only bounded `error_code`, `retryable: true`, and
`retry_after_ms` between zero and 60,000 milliseconds. Invalid guidance fails closed and private
fields are omitted. The adapter never automatically redispatches; synthetic tests cover valid and
out-of-range delays. Gateway support for this guidance is an independent consumer integration gate.

A pure configuration-selection test covers the standalone `mcp-session-1` default, explicit distinct
and same-session overrides, empty values, and invalid-Unicode configuration without mutating shared
process environment. This verifies default selection, not cross-process readiness.

## Runtime-v3 checks

The Runtime-v3 tests assert the exact six advertised tools, fixed semantic paths, bounded action
shapes, generation/identity fencing, redaction, stale handling, and no-fallback timeout behavior.
These are source/test checks, not live provider or host evidence.

The review regressions additionally bind a legal-action catalog to its requested generation and
state, bind settlement witnesses to the returned state and original dispatch generation, and retain
an operation's existing settlement when waiting after a newer observation. Wait and recovery reads
do not reinterpret their current observation generation as the original mutation generation.

`runtime_v3_gameplay_artifact.rs` verifies the full copied package checksum inventory and validates
all four producer goldens. The mapping regression suite schema-validates every advertised tool's
outbound envelope and local uncertainty response, including all four recovery kinds. The artifact
is pinned to the producer SHA recorded in COMPATIBILITY.md, not inferred from matching filenames.

`recovery_ownership.rs` pins that `release_lease` and `stop_episode` map only to
`POST /v3/instances/{id}/recover` with a null operation identity, exactly one gateway request, and a
typed `-32007` result on scope denial. `profile_frame_bounds.rs` proves the MCP frame limit is per
profile: 16 KiB for poc/runtime-v1/runtime-v2 and 256 KiB for Runtime-v3, with one byte more rejected
before any gateway access; the executable HTTP tests pin the 64 KiB and 128 KiB response-body budgets.

Runtime-v3 regression tests construct a schema-valid
oversized settlement receipt and verify one dispatch produces structured uncertainty retaining the
operation identity. These checks use gateway doubles and do not establish host settlement.

## Co-op synchronization verification

`coop_synchronization` consumes all 27 shared vectors and verifies exact metadata, closed
objects, duplicate/integer handling, read-only mapping, configured sessions, and complete
response projection. Executable unit tests cover explicit profile selection and refusal of
missing/foreign bodyless-request authority before TCP. All eight artifact checksum entries
must pass. Existing profiles retain their own catalogs and bounds.

The separate `coop_gateway_runtime` test launches the actual gateway and MCP executables,
uses distinct control/read credentials, and verifies convergence, disagreement, disconnect,
recovery, stale lease refusal, and no downstream game connection. It is explicitly ignored
in single-repository CI because the gateway binary is external; run it as a coordinated gate:

```sh
STS2_COOP_GATEWAY_BINARY=/path/to/reviewed/sts2-gateway-runtime \
  cargo test --locked --offline --package sts2-mcp-server --test coop_gateway_runtime -- --ignored
```

Use separate Cargo target directories for the two worktrees. This test supplies disposable
coordinator reports; it proves executable coordination transport, not native multiplayer.

## Native co-op component checks

The `mapping_coop_native` tests verify the exact seven-tool `coop-native-v1-mcp` catalog, fixed
observation/catalog/action/vote/rejoin/recover routes, response-only effect projection, configured
MCP and gateway identity, copied-artifact metadata, and bounded native envelopes. The projection
tests reject unknown members, operation and route identity drift, HTTP status mismatches, duplicate
catalog IDs, foreign voters, and receipt/effect/observation generation, digest, authority, or
checkpoint inconsistencies. They also retain native HTTP 409 response envelopes for projection and
reject recovery request echoes and route-kind drift. `runtime_support::binding` tests verify that the executable admits only
the six exact native v1 paths, requires the native protocol/schema identity on body-bearing calls,
and preserves explicit gateway authority.

These checks are source/component and synthetic gateway-boundary evidence. They do not start a game,
invoke a model or provider, prove native host legality or settlement, or establish deployment,
release, or Workshop compatibility. The host-backed acceptance campaign must exercise at least two
native instances and preserve the same operation identity through disconnect/rejoin recovery.

The ignored `native_profile_gateway_runtime` gate runs the MCP executable against an exact reviewed
gateway binary and a bounded synthetic loopback producer. It verifies that the closed envelope
retains `protocol_version`, `schema_digest`, and `expected_host_generation` while the downstream
producer receives none of the redundant `x-sts2-protocol-version`, `x-sts2-schema-digest`, or
`x-sts2-host-generation` headers. It also covers private-credential non-forwarding, unknown-action
recovery with a null actor, stale lease rejection, and peer-attribution rejection. Run it only with
the reviewed gateway binary:

```sh
STS2_COOP_GATEWAY_BINARY=/path/to/reviewed/sts2-gateway-runtime \
  cargo test --locked --package sts2-mcp-server --test native_profile_gateway_runtime \
  -- --ignored --exact native_profile_executable_gate_uses_private_binding_and_fences_recovery
```

The producer is synthetic test code, not a game host; passing this gate does not prove real native
host behavior, two-peer settlement, or gameplay.

## Runtime-map profile checks

`runtime_map_v1.rs` verifies the exact seven-tool catalog, `sts2.map_snapshot` argument schema,
configured MCP-session binding, bodyless GET path, explicit gateway authority, complete corrected
golden projection, and stale/foreign/unknown-field rejection. `runtime_map_v1_artifact.rs` checks
the copied artifact bytes associated with recorded protocol source commit
`b3d3034f32e68d70c9e681f906ee37d74db153c4`, schema digest
`ceab0d2dfc471d1ec36d12edaf4654b8c7fdced06548bf47265e11c63f98115b`, copied manifest, schema,
conformance case, three goldens, and every checksum entry; separate source reconciliation establishes
the commit provenance. The executable TCP adapter test covers
the same profile across the actual HTTP framing boundary.

Projection validation preserves overlapping coordinates and disconnected visible components while
rejecting duplicate graph IDs, unknown edge endpoints, cycles, stale generations, invalid visited
position/history references, duplicate bindings or action-option IDs, and over-limit content.
Profile frame/body/projected-content limits are 256 KiB; existing profiles retain their historical
limits. These checks are source/component and artifact-integrity evidence, not host map freshness,
visualizer, provider, or gameplay evidence.

## Seeded-run profile checks

`seeded_run_mapping.rs` verifies the exact two-tool `seeded-run-v1-mcp` catalog, fixed start and
bodyless reconciliation routes, bounded standard context, separate MCP/gateway authority, selected
context digest, and canonical seed/run-start witness projection. It also checks timeout uncertainty,
same-operation reconciliation, and rejection of unsupported fields, operation IDs, and artifact
metadata before forwarding. The copied artifact manifest, schema, conformance case, goldens, and every
`SHA256SUMS` entry are verified through `verify_seeded_run_artifact` and the profile loop above.

These are source/component and artifact-integrity checks. They do not start a native run, establish
host seed readback, prove profile/save isolation or gameplay, or provide deployment and release
evidence.

## Runtime-v4 expert REST-action checks

`runtime_v4_expert_rest_action.rs` covers the exact three-tool catalog, fixed state/action/reconcile
routes, typed Smith and Mend action shapes, operation identity binding, generation/state fencing,
selector admission, and completion witnesses. `runtime_v4_expert_rest_action_errors.rs` checks the
404/408/502/504 `unknown` and 499 `cancelled` mappings, forged same-operation action/generation/state
rejection, and the no-rebinding rule after transport failure. `runtime_v4_expert_rest_action_capacity.rs`
checks that 128 active or pending selector admissions are bounded before forwarding, the 129th is
rejected, terminal capacity is reclaimed, and a late progress GET after terminal completion and
selector-key eviction preserves the original receipt without reactivating the completed selector.

The copied candidate artifact verification checks the manifest status, schema digest
`bb3555fae28eb1f79d08a15e9884696a579e4c20836f5016509f17e0f4c36fbd`, source schema identity, all 16
goldens, 22 mutation fixtures, producer fixtures, and every `SHA256SUMS` entry. Smith and Mend
fixtures provide synthetic producer lifecycle evidence. The tests use gateway doubles and prove
source/component behavior; they do not prove native host legality, provider execution, deployment,
or release compatibility.

## Co-op receipt-query checks

The receipt-query mapping tests verify the exact `sts2.coop_receipt_query` catalog, configured MCP and
gateway sessions, fixed `POST /v1/instances/{id}/coop/receipt-query` route, explicit authority headers,
canonical compact UTF-8 request bytes, complete read-only receipt projection, and rejection of absent
or unsorted actor/participant identity before the gateway. The profile enforces a 16 KiB request and
response bound and performs no observe, reconcile, retry, queue, or mutation operation.

Artifact verification checks the proposed-unadmitted manifest, schema `$id`, digest
`3e3eaedb93926b26025abb09d8028491e2632896753688c1182c698fed7d3f7c`, conformance/fixture/golden
JSON, and every checksum entry. These are canonical-wire, artifact-integrity, and synthetic mapping
checks; no admitted consumer, native producer, host, provider, deployment, or release evidence is
implied.
