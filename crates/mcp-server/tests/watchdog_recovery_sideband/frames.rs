// SPDX-License-Identifier: MIT

//! Recovery-mux protocol state machine for the synthetic host.
//!
//! The closed host-lease-control and operation frames are built here, including
//! the HMAC-SHA256 acknowledgement proof the real gateway validates. This is
//! deterministic synthetic test code, not a game host, and is not evidence of
//! native host behavior.

use super::codec::{
    hex, proof_for_frame, sha256, utc_timestamp_millis, utc_timestamp_now, uuid_v4,
};
use super::{CONTRACT, LEASE_CONTRACT, LEASE_KEY_HEX, LEASE_SCHEMA, SCHEMA};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Default)]
pub struct HostState {
    ops: BTreeMap<String, Value>,
    fence_id: Option<String>,
    lease_expires: Option<String>,
}

pub fn handle(state: &mut HostState, request: &Value) -> (u16, Value) {
    let kind = request["kind"].as_str().unwrap_or_default();
    let correlation = request["correlation_id"].as_str().unwrap_or_default();
    let actor = request["actor"]["principal_id"]
        .as_str()
        .unwrap_or_default();
    match kind {
        "host_fence_request" => {
            let boot = &request["payload"]["boot"];
            let fence_id = uuid_v4();
            state.fence_id = Some(fence_id.clone());
            let fence = json!({
                "host_fence_id": fence_id,
                "deployment_id": boot["deployment_id"],
                "instance_id": boot["instance_id"],
                "instance_incarnation": boot["instance_incarnation"],
                "boot_id": boot["boot_id"],
                "authority_generation": boot["authority_generation"],
                "fence_generation": 1,
                "created_at": utc_timestamp_now(),
            });
            (
                200,
                recovery_frame(
                    "host_fence_response",
                    correlation,
                    "host_fence",
                    json!({"result": result("FENCE_ACCEPTED"), "fence": fence}),
                    actor,
                ),
            )
        }
        "lease_install_request" | "lease_renew_request" | "lease_revoke_request" => {
            let payload = &request["payload"];
            let grant = &payload["grant"];
            let lease = &grant["lease"];
            if kind == "lease_install_request" {
                state.lease_expires = lease["expires_at"].as_str().map(str::to_owned);
            }
            let status = match kind {
                "lease_renew_request" => "RENEWED",
                "lease_revoke_request" => "REVOKED",
                _ => "INSTALLED",
            };
            let ack = json!({
                "result": result(status),
                "installation_id": payload["installation_id"],
                "grant_digest": payload["grant_digest"],
                "boot_id": grant["boot"]["boot_id"],
                "instance_incarnation": grant["boot"]["instance_incarnation"],
                "host_fence_id": grant["fence"]["host_fence_id"],
                "fence_generation": grant["fence"]["fence_generation"],
                "lease_id": lease["lease_id"],
                "lease_epoch": lease["lease_epoch"],
                "host_install_generation": 1,
                "recorded_at": utc_timestamp_now(),
                "renew_sequence": if kind == "lease_renew_request" {
                    payload["renew_sequence"].clone()
                } else {
                    Value::Null
                },
                "expires_at": if kind == "lease_revoke_request" {
                    Value::Null
                } else {
                    lease["expires_at"].clone()
                },
            });
            let mut response = json!({
                "contract": LEASE_CONTRACT,
                "schema_digest": LEASE_SCHEMA,
                "message_id": uuid_v4(),
                "correlation_id": correlation,
                "sent_at": utc_timestamp_now(),
                "actor": {"principal_id": actor, "role": "host"},
                "auth": {
                    "principal_id": actor,
                    "capability": request["auth"]["capability"],
                    "proof": "",
                },
                "kind": kind.replace("_request", "_response"),
                "payload": {"ack": ack},
            });
            let domain = match kind {
                "lease_renew_request" => "host-lease-control/v1/lease-renew-ack",
                "lease_revoke_request" => "host-lease-control/v1/lease-revoke-ack",
                _ => "host-lease-control/v1/lease-install-ack",
            };
            let proof = proof_for_frame(&response, domain, &lease_key());
            response["auth"]["proof"] = Value::String(proof);
            (200, response)
        }
        "operation_intent_request" => {
            let operation = request["payload"]["operation"].clone();
            let created = utc_timestamp_now();
            if let Some(id) = operation["operation_id"].as_str() {
                state.ops.insert(id.to_owned(), operation.clone());
            }
            let full = full_operation(
                &operation,
                "INTENT_RECORDED",
                Value::Null,
                Value::Null,
                &created,
            );
            (
                200,
                recovery_frame(
                    "operation_intent_response",
                    correlation,
                    "operation_submit",
                    json!({"result": result("INTENT_RECORDED"), "operation": full}),
                    actor,
                ),
            )
        }
        "operation_dispatch_request" => {
            let id = request["payload"]["operation"]["operation_id"]
                .as_str()
                .unwrap_or_default();
            let Some(operation) = state.ops.get(id).cloned() else {
                return (
                    404,
                    recovery_frame(
                        "operation_dispatch_response",
                        correlation,
                        "operation_submit",
                        json!({"result": result("NOT_FOUND"), "operation": Value::Null}),
                        actor,
                    ),
                );
            };
            let created = utc_timestamp_now();
            let ticket = ticket_for(state, &operation, &created);
            let witness = witness_for(state, &operation);
            let full = full_operation(&operation, "SETTLED", ticket, witness, &created);
            (
                200,
                recovery_frame(
                    "operation_dispatch_response",
                    correlation,
                    "operation_submit",
                    json!({"result": result("SETTLED"), "operation": full}),
                    actor,
                ),
            )
        }
        "operation_lookup_request" => {
            let id = request["payload"]["operation"]["operation_id"]
                .as_str()
                .unwrap_or_default();
            let Some(operation) = state.ops.get(id).cloned() else {
                return (
                    404,
                    recovery_frame(
                        "operation_lookup_response",
                        correlation,
                        "recovery_read",
                        json!({
                            "result": result("NOT_FOUND"),
                            "operation": Value::Null,
                            "mutation_authorized": false,
                        }),
                        actor,
                    ),
                );
            };
            let created = utc_timestamp_now();
            let ticket = ticket_for(state, &operation, &created);
            let witness = witness_for(state, &operation);
            let full = full_operation(&operation, "SETTLED", ticket, witness, &created);
            (
                200,
                recovery_frame(
                    "operation_lookup_response",
                    correlation,
                    "recovery_read",
                    json!({
                        "result": result("SETTLED"),
                        "operation": full,
                        "mutation_authorized": false,
                    }),
                    actor,
                ),
            )
        }
        "operation_reconcile_request" => {
            let id = request["payload"]["operation"]["operation_id"]
                .as_str()
                .unwrap_or_default();
            let Some(operation) = state.ops.get(id).cloned() else {
                return (
                    404,
                    recovery_frame(
                        "operation_reconcile_response",
                        correlation,
                        "recovery_reconcile",
                        json!({
                            "result": result("NOT_FOUND"),
                            "operation": Value::Null,
                            "witness": Value::Null,
                        }),
                        actor,
                    ),
                );
            };
            let created = utc_timestamp_now();
            let ticket = ticket_for(state, &operation, &created);
            let witness = witness_for(state, &operation);
            let full = full_operation(&operation, "SETTLED", ticket, witness.clone(), &created);
            (
                200,
                recovery_frame(
                    "operation_reconcile_response",
                    correlation,
                    "recovery_reconcile",
                    json!({"result": result("SETTLED"), "operation": full, "witness": witness}),
                    actor,
                ),
            )
        }
        _ => (400, json!({"error_code": "unhandled_kind"})),
    }
}

fn full_operation(
    operation: &Value,
    state_name: &str,
    ticket: Value,
    witness: Value,
    created: &str,
) -> Value {
    json!({
        "operation_id": operation["operation_id"],
        "state": state_name,
        "payload_digest": operation["payload_digest"],
        "original_context": operation["original_context"],
        "expected_boundary": operation["expected_boundary"],
        "action": operation["action"],
        "ticket": ticket,
        "witness": witness,
        "uncertainty_reason": Value::Null,
        "created_at": created,
        "updated_at": utc_timestamp_now(),
    })
}

fn ticket_for(state: &HostState, operation: &Value, created: &str) -> Value {
    // A host admission ticket may never outlive the authenticated lease the
    // operation was admitted under: the gateway rejects it as a contract
    // mismatch otherwise. Clamp to the lease deadline captured on install.
    let expires = state
        .lease_expires
        .clone()
        .filter(|expires| expires.as_str() > created)
        .unwrap_or_else(|| {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|duration| duration.as_millis() as u64)
                .unwrap_or(0);
            utc_timestamp_millis(now.saturating_add(15_000))
        });
    json!({
        "ticket_id": uuid_v4(),
        "operation_id": operation["operation_id"],
        "payload_digest": operation["payload_digest"],
        "boot_id": operation["original_context"]["boot_id"],
        "instance_incarnation": operation["original_context"]["instance_incarnation"],
        "lease_epoch": operation["original_context"]["lease_epoch"],
        "host_fence_id": state.fence_id,
        "state": "SETTLED",
        "issued_at": created,
        "expires_at": expires,
    })
}

fn witness_for(state: &HostState, operation: &Value) -> Value {
    let payload_digest = operation["payload_digest"].as_str().unwrap_or_default();
    let generation = operation["expected_boundary"]["generation"]
        .as_u64()
        .unwrap_or(0)
        .saturating_add(1);
    json!({
        "witness_id": uuid_v4(),
        "operation_id": operation["operation_id"],
        "payload_digest": operation["payload_digest"],
        "boot_id": operation["original_context"]["boot_id"],
        "instance_incarnation": operation["original_context"]["instance_incarnation"],
        "host_fence_id": state.fence_id,
        "source": "host_game_thread",
        "state_id": "00000000-0000-4000-8000-0000000000aa",
        "generation": generation,
        "effect_digest": hex(&sha256(payload_digest.as_bytes())),
        "observed_at": utc_timestamp_now(),
    })
}

fn result(status: &str) -> Value {
    json!({"status": status, "retryable": false, "retry_after_seconds": Value::Null})
}

fn recovery_frame(
    kind: &str,
    correlation: &str,
    capability: &str,
    payload: Value,
    actor: &str,
) -> Value {
    json!({
        "contract": CONTRACT,
        "schema_digest": SCHEMA,
        "message_id": uuid_v4(),
        "correlation_id": correlation,
        "sent_at": utc_timestamp_now(),
        "actor": {"principal_id": actor, "role": "host"},
        "auth": {"principal_id": actor, "capability": capability, "proof": Value::Null},
        "kind": kind,
        "payload": payload,
    })
}

fn lease_key() -> [u8; 32] {
    let mut key = [0_u8; 32];
    for (index, byte) in key.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&LEASE_KEY_HEX[index * 2..index * 2 + 2], 16).unwrap_or(0);
    }
    key
}
