# Workflows

## Change lifecycle

Design or issue → focused change → local validation → pull request → required read-only checks →
authorized merge → separately authorized release and post-release verification.

Green CI is not merge approval, a merge is not a release, and a published artifact is not deployment or
runtime compatibility.

## Automation

`ci.yml` runs formatting, Clippy, MCP/policy Rust tests, and strict repository policy for the initialized
workspace. `policy.yml` independently runs the policy tool and strict check. Both use `pull_request` and pushes to
`main`, top-level `contents: read`, explicit timeouts, cancellation only for superseded pull requests,
and an immutable checkout action commit. Neither workflow uses secrets, trusted self-hosted runners,
write tokens, or `pull_request_target`.

The workflows remain intentionally small and under the repository budget. Product, gateway, security,
compatibility, and release jobs are added only when their command and evidence artifact exist. A job must
not report success for a missing product surface.

## Trust and artifacts

Fork pull requests receive read-only checks without access to host assemblies, saves, providers,
credentials, or trusted networks. Cache only reproducible build inputs. Future diagnostics and artifacts
must have bounded size, deterministic names, source revision metadata, and sanitized contents.

Workflow action references use full 40-character commit SHAs. Shell steps must expose failures; no
unconditional success, blanket retry, or `continue-on-error` is permitted for required evidence.

## Review and release

Workflow changes state their event, permission, runner, ref, cache, artifact, and secret effects. Public
MCP or gateway behavior changes include contract fixtures, compatibility notes, security impact, and an
ADR where ownership or dependency direction changes. Release and deployment actions require explicit
authorization and follow [`RELEASING.md`](../RELEASING.md).
