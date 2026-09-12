// SPDX-License-Identifier: MIT
use super::{GatewayError, GatewayRequest, JsonValue, RuntimeConfig};

pub(super) fn admit(config: &RuntimeConfig, request: &GatewayRequest) -> Result<(), GatewayError> {
    if request.path.ends_with("/checkpoint-reference")
        && request.headers.get("x-sts2-caller-id").map(String::as_str)
            != Some(config.caller_id.as_str())
    {
        return Err(GatewayError::Rejected);
    }
    Ok(())
}

pub(super) fn response(config: &RuntimeConfig, body: &JsonValue) -> Result<(), GatewayError> {
    let JsonValue::Object(object) = body else {
        return Err(GatewayError::MalformedResponse);
    };
    if object.get("schema")
        != Some(&JsonValue::string(
            "ascension.checkpoint_reference_response.v1",
        ))
        || object.get("caller_id") != Some(&JsonValue::string(&config.caller_id))
    {
        return Err(GatewayError::MalformedResponse);
    }
    Ok(())
}

#[cfg(test)]
#[path = "binding_checkpoint_reference_tests.rs"]
mod tests;
