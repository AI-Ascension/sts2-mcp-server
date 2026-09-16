# ADR 0026: exact-restore MCP profile

- Status: Accepted for the bounded MCP consumer source/component seam
- Date: 2026-09-16
- Owner: `sts2-mcp-server`
- Contracts: `sts2-protocol/exact-restore-v1` and the MCP-owned
  `sts2-exact-restore-gateway-v1` wrapper

## Context

The exact-restore protocol defines a staged transfer and one host-effect commit, but it does not
define MCP tools, transport authentication, Gateway routes, or native restore support. The MCP
consumer needs a narrow profile that preserves that division of ownership and cannot turn a lost
commit response into another effect attempt.

## Decision

The opt-in `exact-restore-v1` profile advertises exactly five tools:

| Tool | Fixed route |
| --- | --- |
| `sts2.exact_restore.begin` | `POST /v1/exact-restore/begin` |
| `sts2.exact_restore.put_chunk` | `POST /v1/exact-restore/chunk` |
| `sts2.exact_restore.finish_blob` | `POST /v1/exact-restore/finish` |
| `sts2.exact_restore.commit` | `POST /v1/exact-restore/commit` |
| `sts2.exact_restore.lookup` | `POST /v1/exact-restore/lookup` |

Every argument is a complete closed consumer wrapper around one neutral protocol frame. The MCP
adapter verifies the pinned neutral and wrapper schemas, direction, configured caller, configured
instance/session/lease/epoch, wrapper and frame identifiers, route/phase, operation identity,
request digest, response receipt, and transfer limits. It forwards the wrapper unchanged and never
builds arbitrary routes or accepts headers, credentials, paths, or commands from the MCP caller.

The executable uses the trusted `STS2_RECOVERY_TOKEN` and fixed
`x-sts2-recovery-capability: exact_restore` header. Both complete wrapper and neutral frame are
limited to 16,384 bytes. Chunks are at most 8,192 decoded bytes; the closure is limited to 64
references, 16 MiB per manifest or distinct blob, and 64 MiB aggregate. Base64 is checked for
canonical padding and pad bits before decoding.

Commit is never automatically retried. Once an ambiguous commit request may have reached Gateway,
the same process rejects a second commit for that operation and allows lookup. The durable
operation/receipt ledger remains a Gateway/native-owner responsibility; the in-process fence is an
additional transport guard, not a durability claim.

## Compatibility and evidence

The profile is additive and does not change existing catalogs or bounds. The neutral artifact is
pinned to protocol main commit `5d5a368ef8a89fd1cb356b04dbf9d8a056adbf05`, schema digest
`2289d888c33eac46873408303c4423eab762e3f7bd6132ae8ae88d0d3b1858e4`. The wrapper schema is
consumer-owned and pinned at digest
`0b181dc30524c8b14dea73e490da55538f2d57fe87bf58ed9fe33223406a7d89`.

Artifact, validation, serialized MCP mapping, and loopback HTTP tests establish source/component
behavior at the MCP boundary. They do not prove Gateway persistence, the availability of a native
restore adapter, host state restoration, or deployment and release support. Those claims remain
unverified until their owning targets provide independent evidence.
