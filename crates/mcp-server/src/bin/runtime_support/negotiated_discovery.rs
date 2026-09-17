// SPDX-License-Identifier: MIT

use sts2_mcp_server::{JsonValue, parse_json};

pub(super) const ENVIRONMENT_KEY: &str = "STS2_LOOKUP_BINDING_DISCOVERY_REQUEST_JSON";
pub(super) const MAX_REQUEST_BYTES: usize = 2048;
pub(super) const CORRELATION_ID: &str = "game-information-binding-discovery";
const MAX_SAFE_INTEGER: i64 = 9_007_199_254_740_991;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct DiscoveryRequest {
    pub(super) project_id: String,
    pub(super) run_id: String,
    pub(super) episode_id: String,
    pub(super) agent_id: String,
    pub(super) authority_epoch: i64,
}

impl DiscoveryRequest {
    pub(super) fn gateway_body(&self) -> JsonValue {
        JsonValue::object([
            (String::from("operation"), JsonValue::string("discovery")),
            (
                String::from("project_id"),
                JsonValue::string(self.project_id.as_str()),
            ),
            (
                String::from("run_id"),
                JsonValue::string(self.run_id.as_str()),
            ),
            (
                String::from("episode_id"),
                JsonValue::string(self.episode_id.as_str()),
            ),
            (
                String::from("agent_id"),
                JsonValue::string(self.agent_id.as_str()),
            ),
            (
                String::from("authority_epoch"),
                JsonValue::Number(self.authority_epoch),
            ),
        ])
    }
}

pub(super) fn from_environment() -> Result<Option<DiscoveryRequest>, String> {
    match std::env::var(ENVIRONMENT_KEY) {
        Ok(value) => parse(&value).map(Some),
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(std::env::VarError::NotUnicode(_)) => {
            Err(format!("{ENVIRONMENT_KEY} is not valid UTF-8"))
        }
    }
}

fn parse(input: &str) -> Result<DiscoveryRequest, String> {
    if input.len() > MAX_REQUEST_BYTES {
        return Err(format!("{ENVIRONMENT_KEY} exceeds 2048 bytes"));
    }
    let value = parse_json(input).map_err(|_| format!("{ENVIRONMENT_KEY} is invalid JSON"))?;
    let object = exact_object(
        &value,
        &["operation", "scope", "authority_epoch", "correlation_id"],
    )?;
    if string(object, "operation")? != "discovery"
        || string(object, "correlation_id")? != CORRELATION_ID
    {
        return Err(format!(
            "{ENVIRONMENT_KEY} is not a fixed discovery request"
        ));
    }
    let scope = object
        .get("scope")
        .ok_or_else(|| format!("{ENVIRONMENT_KEY} scope is missing"))?;
    let scope = exact_object(scope, &["project_id", "run_id", "episode_id", "agent_id"])?;
    let authority_epoch = match object.get("authority_epoch") {
        Some(JsonValue::Number(value)) if (1..=MAX_SAFE_INTEGER).contains(value) => *value,
        _ => {
            return Err(format!(
                "{ENVIRONMENT_KEY} authority_epoch is outside its bound"
            ));
        }
    };
    Ok(DiscoveryRequest {
        project_id: identity(scope, "project_id")?,
        run_id: identity(scope, "run_id")?,
        episode_id: identity(scope, "episode_id")?,
        agent_id: identity(scope, "agent_id")?,
        authority_epoch,
    })
}

fn exact_object<'a>(
    value: &'a JsonValue,
    fields: &[&str],
) -> Result<&'a std::collections::BTreeMap<String, JsonValue>, String> {
    let object = value
        .as_object()
        .ok_or_else(|| format!("{ENVIRONMENT_KEY} object is malformed"))?;
    if object.len() != fields.len() || fields.iter().any(|field| !object.contains_key(*field)) {
        return Err(format!("{ENVIRONMENT_KEY} has unknown or missing fields"));
    }
    Ok(object)
}

fn string<'a>(
    object: &'a std::collections::BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<&'a str, String> {
    object
        .get(key)
        .and_then(JsonValue::as_string)
        .ok_or_else(|| format!("{ENVIRONMENT_KEY} {key} is not a string"))
}

fn identity(
    object: &std::collections::BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<String, String> {
    let value = string(object, key)?;
    if value.is_empty()
        || value.len() > 128
        || value.contains("..")
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-_./:".contains(&byte))
    {
        return Err(format!("{ENVIRONMENT_KEY} {key} is unsafe or oversized"));
    }
    Ok(value.to_owned())
}

#[cfg(test)]
#[path = "negotiated_discovery_tests.rs"]
mod tests;
