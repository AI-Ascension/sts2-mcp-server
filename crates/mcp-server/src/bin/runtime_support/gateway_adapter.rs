// SPDX-License-Identifier: MIT

use super::{RuntimeGatewayAdapter, binding, exchange};
use sts2_mcp_server::{
    COOP_NATIVE_PROTOCOL_VERSION, COOP_NATIVE_SCHEMA_DIGEST, COOP_RECEIPT_QUERY_PROTOCOL_VERSION,
    ExactRestorePhase, GAME_INFORMATION_PROTOCOL_VERSION, GAME_INFORMATION_SCHEMA_DIGEST,
    GatewayAdapter, GatewayError, GatewayRequest, GatewayResponse, JsonValue,
    RUNTIME_MAP_V1_PROTOCOL_VERSION, RUNTIME_V2_PROTOCOL_VERSION,
    RUNTIME_V3_GAMEPLAY_PROTOCOL_VERSION, RUNTIME_V4_EXPERT_ACTION_PROTOCOL_VERSION,
    RUNTIME_V4_EXPERT_REST_ACTION_PROTOCOL_VERSION, SEEDED_RUN_PROTOCOL_VERSION,
    SEEDED_RUN_SCHEMA_DIGEST, validate_exact_restore_response,
};

#[path = "gateway_adapter_negotiated.rs"]
mod negotiated;

const MAX_BODY_BYTES: usize = 16 * 1024;

impl RuntimeGatewayAdapter {
    pub(super) fn body(&self, request: &GatewayRequest) -> Result<Vec<u8>, GatewayError> {
        let Some(value) = &request.body else {
            return Ok(Vec::new());
        };
        let JsonValue::Object(mut object) = value.clone() else {
            return Err(GatewayError::Rejected);
        };
        let is_runtime_v2 = protocol_is(&object, RUNTIME_V2_PROTOCOL_VERSION);
        let is_runtime_v3 = protocol_is(&object, RUNTIME_V3_GAMEPLAY_PROTOCOL_VERSION);
        let is_runtime_v4_action = protocol_is(&object, RUNTIME_V4_EXPERT_ACTION_PROTOCOL_VERSION);
        let is_runtime_v4_rest_action =
            protocol_is(&object, RUNTIME_V4_EXPERT_REST_ACTION_PROTOCOL_VERSION);
        let is_runtime_map = protocol_is(&object, RUNTIME_MAP_V1_PROTOCOL_VERSION);
        let is_coop_receipt_query = protocol_is(&object, COOP_RECEIPT_QUERY_PROTOCOL_VERSION);
        let is_coop_native = protocol_is(&object, COOP_NATIVE_PROTOCOL_VERSION);
        let is_seeded_run = protocol_is(&object, SEEDED_RUN_PROTOCOL_VERSION);
        let is_game_information = protocol_is(&object, GAME_INFORMATION_PROTOCOL_VERSION);
        let is_save_profile = binding::is_save_profile_route(request);
        let is_exact_restore = binding::is_exact_restore_route(request);
        let is_game_information_binding =
            binding::is_game_information_binding_route(&self.config, request);
        if !is_save_profile
            && !is_exact_restore
            && !is_game_information_binding
            && (is_runtime_v2
                || is_runtime_v3
                || is_runtime_v4_action
                || is_runtime_v4_rest_action
                || is_runtime_map
                || is_coop_receipt_query
                || is_coop_native
                || is_seeded_run)
        {
            self.validate_profile_identity(&object)?;
            self.validate_profile_digest(&object, is_seeded_run, is_coop_native)?;
        } else if !is_save_profile && !is_exact_restore && is_game_information {
            if object.get("schema_digest")
                != Some(&JsonValue::string(GAME_INFORMATION_SCHEMA_DIGEST))
                || object.get("kind") != Some(&JsonValue::string("query_request"))
            {
                return Err(GatewayError::Rejected);
            }
        } else if is_game_information_binding {
            negotiated::validate_binding_body(&object)?;
        } else if !is_save_profile && !is_exact_restore {
            self.inject_profile_identity(&mut object);
        }
        let encoded = if is_coop_receipt_query {
            sts2_mcp_server::canonical_coop_receipt_query(&JsonValue::Object(object))
                .ok_or(GatewayError::Rejected)?
        } else {
            JsonValue::Object(object).to_json()
        };
        if encoded.len() > MAX_BODY_BYTES {
            return Err(GatewayError::Rejected);
        }
        Ok(encoded.into_bytes())
    }

    fn validate_profile_identity(
        &self,
        object: &std::collections::BTreeMap<String, JsonValue>,
    ) -> Result<(), GatewayError> {
        if object.get("instance_id") != Some(&JsonValue::string(self.config.instance_id.as_str()))
            || object.get("session_id") != Some(&JsonValue::string(self.config.session_id.as_str()))
            || object.get("lease_id") != Some(&JsonValue::string(self.config.lease_id.as_str()))
            || object.get("lease_epoch") != Some(&JsonValue::Number(self.config.lease_epoch))
        {
            return Err(GatewayError::Rejected);
        }
        Ok(())
    }

    fn validate_profile_digest(
        &self,
        object: &std::collections::BTreeMap<String, JsonValue>,
        is_seeded_run: bool,
        is_coop_native: bool,
    ) -> Result<(), GatewayError> {
        if is_seeded_run
            && object.get("schema_digest") != Some(&JsonValue::string(SEEDED_RUN_SCHEMA_DIGEST))
        {
            return Err(GatewayError::Rejected);
        }
        if is_coop_native
            && object.get("schema_digest") != Some(&JsonValue::string(COOP_NATIVE_SCHEMA_DIGEST))
        {
            return Err(GatewayError::Rejected);
        }
        Ok(())
    }

    fn inject_profile_identity(&self, object: &mut std::collections::BTreeMap<String, JsonValue>) {
        object.insert(
            String::from("instance_id"),
            JsonValue::string(self.config.instance_id.as_str()),
        );
        object.insert(
            String::from("session_id"),
            JsonValue::string(self.config.session_id.as_str()),
        );
        object.insert(
            String::from("lease_id"),
            JsonValue::string(self.config.lease_id.as_str()),
        );
        object.insert(
            String::from("lease_epoch"),
            JsonValue::Number(self.config.lease_epoch),
        );
    }
}

fn protocol_is(object: &std::collections::BTreeMap<String, JsonValue>, expected: &str) -> bool {
    matches!(
        object.get("protocol_version"),
        Some(JsonValue::String(value)) if value == expected
    )
}

impl GatewayAdapter for RuntimeGatewayAdapter {
    fn forward(&mut self, request: GatewayRequest) -> Result<GatewayResponse, GatewayError> {
        binding::admit(&self.config, &request)?;
        let restore = binding::exact_restore_binding(&self.config, &request)?;
        if let Some(binding) = &restore
            && binding.phase == ExactRestorePhase::Commit
            && self.lookup_only_operations.contains(&binding.operation_id)
        {
            return Err(GatewayError::Rejected);
        }
        let mut request = binding::attach_native_peer_token(&self.config, request)?;
        if let Some(binding) = &restore {
            request.headers.insert(
                String::from("x-sts2-correlation-id"),
                binding.correlation_id.clone(),
            );
        }
        let save_profile_route = binding::is_save_profile_route(&request);
        let response_kind = binding::response_kind(&self.config, &request);
        let expert_state_route = request.method == sts2_mcp_server::GatewayMethod::Get
            && request.path == format!("/v4/instances/{}/expert-state", self.config.instance_id);
        let correlation = request.correlation.mcp_request_id.stable_text();
        let wire_operation = negotiated::wire_operation(&self.config.instance_id, &request);
        let catalog_read = request.method == sts2_mcp_server::GatewayMethod::Get
            && request.path == format!("/v3/instances/{}/legal-actions", self.config.instance_id);
        let body = self.body(&request)?;
        let response_limit = if self.enforce_wire_limits {
            let operation = wire_operation.ok_or(GatewayError::Rejected)?;
            let limits = self
                .wire_limits
                .get(operation)
                .ok_or(GatewayError::Rejected)?;
            if body.len() > limits.max_request_bytes {
                return Err(GatewayError::Rejected);
            }
            limits.max_response_bytes
        } else {
            self.max_response_bytes
        };
        let response = match exchange::exchange(&self.config, request, body, response_limit) {
            Ok(response) => response,
            Err(error) => {
                self.retain_lookup_only_after_uncertain_commit(&restore);
                return Err(error);
            }
        };
        if let Some(binding) = &restore {
            let validated = match validate_exact_restore_response(
                &response.body,
                binding,
                &self.config.caller_id,
            ) {
                Ok(validated) => validated,
                Err(_) => {
                    self.retain_lookup_only_after_uncertain_commit(&restore);
                    return Err(GatewayError::MalformedResponse);
                }
            };
            if validated.lookup_only {
                self.lookup_only_operations
                    .insert(binding.operation_id.clone());
            }
            return Ok(response);
        }
        if catalog_read
            && sts2_mcp_server::catalog_reobserve_body(&response, &correlation).is_some()
        {
            return Ok(response);
        }
        if !expert_state_route
            && ((200..300).contains(&response.status) || binding::is_runtime_result(&response.body))
            && let Some(kind) = response_kind
        {
            binding::response(&self.config, &response.body, &correlation, kind)?;
        }
        if save_profile_route {
            binding::classify_save_profile(response)
        } else {
            exchange::classify(response)
        }
    }
}

impl RuntimeGatewayAdapter {
    fn retain_lookup_only_after_uncertain_commit(
        &mut self,
        binding: &Option<sts2_mcp_server::ExactRestoreRequestBinding>,
    ) {
        if let Some(binding) = binding
            && binding.phase == ExactRestorePhase::Commit
        {
            self.lookup_only_operations
                .insert(binding.operation_id.clone());
        }
    }
}
