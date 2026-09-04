// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use crate::json::JsonValue;
use crate::projection::RuntimeV3GameplayProjectionContext;

const MAX_GENERATION: i64 = 9_007_199_254_740_991;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct RuntimeV3GameplayContext {
    pub(super) projection: RuntimeV3GameplayProjectionContext,
    pub(super) state_id: Option<String>,
    pub(super) operation_id: Option<String>,
}

impl RuntimeV3GameplayContext {
    pub(super) fn parse(
        arguments: &BTreeMap<String, JsonValue>,
        correlation_id: &str,
        require_state_id: bool,
        require_operation_id: bool,
    ) -> Result<Self, &'static str> {
        let instance_id = string_argument(arguments, "instance_id")?;
        let session_id = string_argument(arguments, "mcp_session_id")?;
        let lease_id = string_argument(arguments, "lease_id")?;
        if !crate::mapping::safe_segment(instance_id)
            || !crate::mapping::safe_header_value(session_id)
            || !crate::mapping::safe_header_value(lease_id)
            || !crate::mapping::safe_header_value(correlation_id)
        {
            return Err("Runtime-v3 identity is unsafe or oversized");
        }
        let lease_epoch = bounded_argument(arguments, "lease_epoch")?;
        let generation = bounded_argument(arguments, "generation")?;
        let state_id = optional_string_argument(arguments, "state_id", require_state_id)?;
        let operation_id = optional_string_argument(arguments, "operation_id", require_operation_id)?;
        if state_id
            .as_deref()
            .is_some_and(|value| !crate::mapping::safe_header_value(value))
            || operation_id
                .as_deref()
                .is_some_and(|value| !crate::mapping::safe_header_value(value))
        {
            return Err("Runtime-v3 state or operation identity is unsafe or oversized");
        }
        Ok(Self {
            projection: RuntimeV3GameplayProjectionContext {
                correlation_id: String::from(correlation_id),
                instance_id: String::from(instance_id),
                session_id: String::from(session_id),
                lease_id: String::from(lease_id),
                lease_epoch,
                generation,
                operation_id: operation_id.clone(),
            },
            state_id,
            operation_id,
        })
    }

    pub(super) fn instance_id(&self) -> &str {
        self.projection.instance_id.as_str()
    }

    pub(super) fn session_id(&self) -> &str {
        self.projection.session_id.as_str()
    }

    pub(super) fn correlation_id(&self) -> &str {
        self.projection.correlation_id.as_str()
    }

    pub(super) fn generation(&self) -> i64 {
        self.projection.generation
    }
}

fn string_argument<'a>(
    arguments: &'a BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<&'a str, &'static str> {
    arguments
        .get(key)
        .and_then(JsonValue::as_string)
        .filter(|value| !value.is_empty())
        .ok_or("Runtime-v3 identity argument must be a non-empty string")
}

fn optional_string_argument(
    arguments: &BTreeMap<String, JsonValue>,
    key: &str,
    required: bool,
) -> Result<Option<String>, &'static str> {
    match arguments.get(key) {
        Some(JsonValue::String(value)) if !value.is_empty() => Ok(Some(value.clone())),
        Some(JsonValue::Null) if !required => Ok(None),
        Some(JsonValue::String(_)) => Err("Runtime-v3 identity argument must be non-empty"),
        None if !required => Ok(None),
        _ => Err("Runtime-v3 required identity argument is missing or invalid"),
    }
}

fn bounded_argument(
    arguments: &BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<i64, &'static str> {
    match arguments.get(key) {
        Some(JsonValue::Number(value)) if *value >= 0 && *value <= MAX_GENERATION => Ok(*value),
        _ => Err("Runtime-v3 generation or lease_epoch is outside the protocol bound"),
    }
}
