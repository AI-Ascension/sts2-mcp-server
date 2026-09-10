# Native co-op component consumer

Status: implemented as a source and component integration. The `coop-native-v1` profile is
selected with `STS2_RUNTIME_PROFILE=coop-native-v1` and consumes the accepted component artifact
at schema digest
`2f3bc99e53080fa11b39592b64fb0ab964a16f568719a2622d0b2caf766ab629`.

The catalog exposes seven typed tools: observation, legal catalog, local action, shared vote,
rejoin, recovery, and effect projection. Observation uses the bodyless
`GET /v1/instances/{id}/coop/native/observation` route. Legal catalog uses
`POST /v1/instances/{id}/coop/native/legal-catalog` with the closed
`legal_catalog_request` envelope. Mutations and recovery use their corresponding fixed POST
routes. No arbitrary downstream path is accepted.

MCP validates the closed twenty-member envelope, exact protocol metadata and configured session
identity, producer provenance, nested observations, legal catalog entries, operation-bound effects,
and receipts. `unknown` and rejected results remain tool errors; the adapter never retries or
infers settlement. The artifact copy and deterministic gateway-double checks establish component
behavior only.

This lane does not add gateway route registration, host access, provider execution, or lifecycle
control. The current gateway and harness heads must implement and independently verify their named
consumer contracts before this profile can be used across the runtime. Live native multiplayer
remains `unverified` until a disposable two-peer host-backed session proves identity, legal
catalog, settled action and vote, checksum agreement, disconnect/rejoin recovery, and convergence.
