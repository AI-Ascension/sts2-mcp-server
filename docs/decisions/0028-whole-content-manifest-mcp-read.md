# ADR 0028: whole-catalog content-manifest MCP read

- Status: Accepted for the bounded whole-catalog read
- Date: 2026-09-20
- Owner: `sts2-mcp-server`
- Contracts: `sts2-protocol/game-information-content-manifest-v1` and the gateway-owned
  `GET /v1/instances/{id}/game-information/content-manifest` route

## Context

The game-information query profile answers bounded list, search, get, detail, and availability reads
from a catalog the gateway already holds, but it never tells a consumer what that catalog is. A
caller that needs the pinned content revision — build, packages, families, and the content,
localized-text, and inventory revisions — had no MCP tool for it, so a definition absent from a page
could not be told apart from one the authority never had. `sts2-protocol` defines the whole-manifest
envelope for exactly that question, and the gateway implements the fixed bodyless route in
`AI-Ascension/sts2-gateway#87` (merge `2d7f758b`), but the MCP adapter exposed no tool for it.

Two shortcuts are wrong here. Answering from the producer's capabilities read would return a
second, differently-scoped summary under a route whose bounds were never pinned for it. Relaying a
shortened catalog when the authority refuses to produce a complete one would turn a refusal into
plausible-looking data a consumer could cache and act on.

## Decision

`sts2.game_information_content_manifest` is the eighth tool of the existing
`game-information-query-v1-mcp` catalog, and the catalog revision is unchanged. One additive read
is a new member of a pinned catalog rather than a new profile, matching the precedent of
`sts2.game_information_binding` (`f49dbf9`).

The tool takes no selector. Its arguments are the four context arguments every other member takes —
`instance_id`, `mcp_session_id`, `lease_id`, `lease_epoch` — and it is forwarded as a bodyless
`GET /v1/instances/{id}/game-information/content-manifest` carrying the same identity headers and
`x-mcp-request-id` as the other reads. The gateway decides which content authority the request is
answered from; the adapter never sends a content manifest, locale, or scope of its own.

The projection states the pinned artifact's own identity instead of re-deriving it. The envelope
must be exactly the seven members `protocol_version`, `schema_digest`, `provenance`,
`correlation_id`, `kind`, `manifest`, and `error`; the provenance must name
`sts2-protocol/game-information-content-manifest-v1`, the schema source path, and the generator
`hand-authored`; and the digest must equal the pinned
`416a39769445e6e462c5d5b5504f29010c255e2116a73094e55c7268e47f2ba6`.

A `content_manifest_response` must carry all ten catalog members and no error arm. An
`error_response` must carry no catalog, and its code must be paired with a reason that code's own
vocabulary admits, so `access_denied` carrying a malformed-catalog reason is refused rather than
relayed as a denial. A refusal is relayed as the protocol's typed refusal — the caller sees
`access_denied`, not a malformed answer — and an accepted catalog is passed through verbatim,
because re-summarizing a catalog would destroy the revision evidence the caller asked for.

## Bounds

The fixed route frames 128 KiB, so a body beyond that is not a response the route can produce. This
hop admits 128 KiB and answers a larger body with `game_information_response_too_large` rather than
a shortened catalog, and the negotiated wire-limit path clamps the tool's own admitted bytes to the
same number so no configuration can buffer a body the route cannot carry. The profile's 16 MiB
serialization ceiling remains declared by the pinned manifest for the producer; it is not a bound
this hop relays under.

## Composition

The tool is admitted to negotiated composition as a `StaticReference` read under the revision
`game-information-content-manifest-v1-mcp`. The route carries no negotiated offer today, so a
composition without a gateway offer drops the tool instead of advertising a read the gateway never
admitted; that is asserted, not assumed.

## Evidence and limits

The pinned artifact is pinned by digest together with its whole checksum inventory. The protocol's
own case file and the eight vectors it declares are vendored byte-for-byte from `sts2-protocol` and
pinned by digest in the adapter, and each is driven through the projection: the complete catalog
and the typed refusal are relayed, all five declared owner-error mappings survive as their own
codes, and every invalid vector fails closed. The artifact's upstream `SHA256SUMS` covers only the
files beside it, so these copies are pinned here rather than by the artifact's own inventory; that
narrower upstream inventory is a contract-side gap, not something this hop can close by editing
the pin.

The following claims remain unverified and must not be inferred from this decision: producer
integration of the whole-manifest read in `sts2-game-mod` (the artifact's own manifest still
records it as `pending in sts2-game-mod`); live gateway readiness and selection for this route;
native host authority over the content catalog; and deployment or release support.
