# Releasing

No public MCP artifact has been published. The current `0.0.0` workspace version
identifies preparation tooling, not a distributable MCP server. Current `main` contains the bounded
MCP process and versioned profiles for the POC, Runtime-v1, Runtime-v2, Runtime-v3, Runtime-v4
expert binding, and read-only co-op synchronization; each maps only fixed, authenticated gateway
routes and rejects unknown profiles, fields, identities, and response shapes.

For each release, a release owner must define the MCP profile, gateway contract revision, supported
Rust/runtime/platform combinations, package allowlist, and compatibility evidence.
The release must be built from an approved immutable revision with a committed lockfile and checksums.

Before any publication, verify:

- repository policy, formatting, lint, tests, protocol/gateway conformance, and security checks pass;
- every advertised MCP tool and gateway route has an owner-local fixture and versioned mapping;
- no proprietary host files, saves, credentials, private paths, or generated workspace state is packaged;
- compatibility evidence distinguishes build, transport, gateway, host, and end-to-end runtime levels;
- the changelog, compatibility record, notices, and package manifest describe the exact bytes; and
- maintainer approval and any protected release environment requirements are satisfied.

The current records establish deterministic framing/mapping and bounded component/runtime lanes. The
Runtime-v1 host trace and the [Windows](https://github.com/AI-Ascension/sts2-harness/blob/main/docs/evidence/seeded-astra-campaign-20260906.md)
and [Linux](https://github.com/AI-Ascension/sts2-harness/blob/main/docs/evidence/linux-seeded-campaign-20260906.md)
Runtime-v3 campaign records apply only to their named STS2 v0.107.1 fixtures and matching downstream
binaries; those records preserve provider-backed campaigns and settled downstream actions for those
exact runs. Runtime-v2 remains a source/fake seam, Runtime-v4 remains source/component binding
evidence, and the co-op profile is read-only coordinator reporting. Native Runtime-v4 host
legality/effects under a final current-head release, native multiplayer actuation, cross-consumer
integration for that release, deployment, a distributable package, and compatibility beyond the
recorded fixtures remain unverified. The gateway API revision is also not yet frozen as a release
contract.

Preparation, tagging, publication, deployment, and post-release verification are separate authority
events. Never treat a passing local check as permission to perform them.
