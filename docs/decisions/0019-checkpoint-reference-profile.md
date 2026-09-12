# ADR 0019: public checkpoint reference MCP profile

Accepted source/component contract; native producer remains unverified.

`checkpoint-reference-v1` selects the standalone `sts2.checkpoint_reference` read tool.
It requires instance_id, mcp_session_id, caller_id, lease_id and lease_epoch. The configured
gateway session is distinct from MCP session. The executable admits explicit authority against
configuration before injecting credentials and forwards only bodyless GET
`/v1/instances/{instance_id}/checkpoint-reference`. Legacy catalogs remain unchanged.

Gateway owns authorization and current lease authority. The response is the gateway ADR 0023
closed eight-member `ascension.checkpoint_reference_response.v1` envelope. MCP checks authority
and correlation and projects only its digest-free `exact-checkpoint-reference-v1` reference.
Unknown fields, malformed handles, inconsistent assurance, foreign authority and responses above
8192 bytes fail with redacted tool errors. No reference is fabricated for unavailable producers.
No capture/restore mutation, journal, cache, filesystem access or hidden payload tool is added.

Cancellation uses the existing read transport behavior without retry or settlement claims.
Deterministic tests exercise the real catalog/mapping and executable admission/response binding,
including wrong caller and lease, hidden fields, unsupported versions and unavailable transport.
