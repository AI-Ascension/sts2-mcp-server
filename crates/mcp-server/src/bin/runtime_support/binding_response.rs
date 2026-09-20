// SPDX-License-Identifier: MIT

use super::RuntimeConfig;
use sts2_mcp_server::{CONTENT_MANIFEST_PROTOCOL_VERSION, CONTENT_MANIFEST_SCHEMA_DIGEST};
use sts2_mcp_server::{
    GAME_INFORMATION_PROTOCOL_VERSION, GAME_INFORMATION_SCHEMA_DIGEST, GatewayError, JsonValue,
    SAVE_PROFILE_CONTRACT,
};

pub(crate) fn validate(
    config: &RuntimeConfig,
    body: &JsonValue,
    correlation: &str,
    kind: &str,
) -> Result<(), GatewayError> {
    if matches!(
        kind,
        "capabilities_response" | "content_manifest_response" | "game_information_response"
    ) {
        return game_information(config, body, correlation, kind);
    }
    if kind == "save_profile_response" {
        return save_profile(body, config, correlation);
    }
    let JsonValue::Object(object) = body else {
        return Err(GatewayError::MalformedResponse);
    };
    for (name, expected) in [
        ("instance_id", config.instance_id.as_str()),
        ("session_id", config.session_id.as_str()),
        ("lease_id", config.lease_id.as_str()),
        ("correlation_id", correlation),
        ("kind", kind),
    ] {
        if name == "kind" && kind == "checkpoint_reference_response" {
            super::checkpoint_reference::response(config, body)?;
            continue;
        }
        if object.get(name) != Some(&JsonValue::string(expected)) {
            return Err(GatewayError::MalformedResponse);
        }
    }
    if object.get("lease_epoch") != Some(&JsonValue::Number(config.lease_epoch)) {
        return Err(GatewayError::MalformedResponse);
    }
    Ok(())
}

fn save_profile(
    body: &JsonValue,
    config: &RuntimeConfig,
    correlation: &str,
) -> Result<(), GatewayError> {
    let JsonValue::Object(object) = body else {
        return Err(GatewayError::MalformedResponse);
    };
    if let Some(contract) = object.get("contract") {
        if contract != &JsonValue::string(SAVE_PROFILE_CONTRACT) {
            return Err(GatewayError::MalformedResponse);
        }
    } else if !matches!(object.get("error_code"), Some(JsonValue::String(_))) {
        return Err(GatewayError::MalformedResponse);
    }
    for (field, expected) in [
        ("instance_id", config.instance_id.as_str()),
        ("caller_id", config.caller_id.as_str()),
        ("session_id", config.session_id.as_str()),
        ("mcp_session_id", config.mcp_session_id.as_str()),
        ("lease_id", config.lease_id.as_str()),
        ("correlation_id", correlation),
    ] {
        if let Some(value) = object.get(field)
            && value != &JsonValue::string(expected)
        {
            return Err(GatewayError::MalformedResponse);
        }
    }
    if let Some(epoch) = object.get("lease_epoch")
        && epoch != &JsonValue::Number(config.lease_epoch)
    {
        return Err(GatewayError::MalformedResponse);
    }
    Ok(())
}

fn game_information(
    config: &RuntimeConfig,
    body: &JsonValue,
    correlation: &str,
    expected: &str,
) -> Result<(), GatewayError> {
    let JsonValue::Object(object) = body else {
        return Err(GatewayError::MalformedResponse);
    };
    // Each game-information read kind has its own pinned profile and digest, so the
    // expected identity is selected by the kind the request path already implied.
    let (protocol_version, schema_digest) = match expected {
        "content_manifest_response" => (
            CONTENT_MANIFEST_PROTOCOL_VERSION,
            CONTENT_MANIFEST_SCHEMA_DIGEST,
        ),
        _ => (
            GAME_INFORMATION_PROTOCOL_VERSION,
            GAME_INFORMATION_SCHEMA_DIGEST,
        ),
    };
    if object.get("protocol_version") != Some(&JsonValue::string(protocol_version))
        || object.get("schema_digest") != Some(&JsonValue::string(schema_digest))
        || object.get("correlation_id") != Some(&JsonValue::string(correlation))
    {
        return Err(GatewayError::MalformedResponse);
    }
    let Some(JsonValue::String(response_kind)) = object.get("kind") else {
        return Err(GatewayError::MalformedResponse);
    };
    if (expected == "capabilities_response"
        && !matches!(
            response_kind.as_str(),
            "capabilities_response" | "error_response"
        ))
        || (expected == "content_manifest_response"
            && !matches!(
                response_kind.as_str(),
                "content_manifest_response" | "error_response"
            ))
        || (expected == "game_information_response"
            && !matches!(response_kind.as_str(), "query_response" | "error_response"))
    {
        return Err(GatewayError::MalformedResponse);
    }
    if expected == "game_information_response" && response_kind == "query_response" {
        let Some(JsonValue::Object(query)) = object.get("query") else {
            return Err(GatewayError::MalformedResponse);
        };
        let Some(JsonValue::Object(binding)) = query.get("binding") else {
            return Err(GatewayError::MalformedResponse);
        };
        if binding.get("mode") == Some(&JsonValue::string("live")) {
            let Some(JsonValue::Object(instance)) = binding.get("instance_ref") else {
                return Err(GatewayError::MalformedResponse);
            };
            if instance.get("instance_id") != Some(&JsonValue::string(config.instance_id.as_str()))
            {
                return Err(GatewayError::MalformedResponse);
            }
        }
    }
    Ok(())
}
