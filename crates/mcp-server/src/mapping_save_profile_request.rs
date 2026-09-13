// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use crate::catalog::SAVE_PROFILE_MAX_BODY_BYTES;
use crate::gateway::{Correlation, GatewayAdapter, GatewayMethod, GatewayRequest};
use crate::json::JsonValue;
use crate::protocol::RequestId;
use crate::server::McpServer;

use super::{CallKind, Context};

pub(super) const COMMON_ARGUMENTS: [&str; 4] =
    ["instance_id", "mcp_session_id", "lease_id", "lease_epoch"];
pub(super) const SELECT_ARGUMENTS: [&str; 6] = [
    "instance_id",
    "mcp_session_id",
    "lease_id",
    "lease_epoch",
    "profile_id",
    "baseline",
];
pub(super) const STATUS_ARGUMENTS: [&str; 5] = [
    "instance_id",
    "mcp_session_id",
    "lease_id",
    "lease_epoch",
    "operation_id",
];

pub(super) fn context<G: GatewayAdapter>(
    server: &McpServer<G>,
    arguments: &BTreeMap<String, JsonValue>,
    correlation_id: String,
    request_id: RequestId,
    kind: CallKind,
) -> Result<Context, &'static str> {
    let instance_id = identity(arguments, "instance_id")?;
    let mcp_session_id = identity(arguments, "mcp_session_id")?;
    let lease_id = identity(arguments, "lease_id")?;
    if !super::super::safe_segment(instance_id)
        || !safe_authority_identity(mcp_session_id)
        || !safe_authority_identity(lease_id)
        || server
            .mcp_session_id()
            .is_some_and(|expected| expected != mcp_session_id)
    {
        return Err("save-profile identity or MCP session is unsafe or not bound");
    }
    let gateway_session_id = server
        .gateway_session_id()
        .unwrap_or(mcp_session_id)
        .to_owned();
    if !super::super::safe_header_value(&gateway_session_id) {
        return Err("gateway session identity is unsafe or oversized");
    }
    let lease_epoch = epoch(arguments)?;
    let operation_id = if kind == CallKind::Status {
        let value = identity(arguments, "operation_id")?;
        if !safe_operation_id(value) {
            return Err("operation_id is unsafe or cannot be used as one route segment");
        }
        value.to_owned()
    } else {
        correlation_id.clone()
    };
    let requested_profile = if kind == CallKind::Select {
        Some(profile_id(arguments)?.to_owned())
    } else {
        None
    };
    Ok(Context {
        instance_id: instance_id.to_owned(),
        mcp_session_id: mcp_session_id.to_owned(),
        gateway_session_id,
        lease_id: lease_id.to_owned(),
        lease_epoch,
        correlation_id,
        operation_id,
        requested_profile,
        kind,
        request_id,
    })
}

pub(super) fn gateway_request(
    context: &Context,
    arguments: &BTreeMap<String, JsonValue>,
) -> Result<GatewayRequest, &'static str> {
    let mut headers = super::super::headers(&context.mcp_session_id, &context.correlation_id);
    headers.extend([
        (
            String::from("x-sts2-instance-id"),
            context.instance_id.clone(),
        ),
        (
            String::from("x-sts2-session-id"),
            context.gateway_session_id.clone(),
        ),
        (String::from("x-sts2-lease-id"), context.lease_id.clone()),
        (
            String::from("x-sts2-lease-epoch"),
            context.lease_epoch.to_string(),
        ),
    ]);
    let (method, path, body) = match context.kind {
        CallKind::List => (
            GatewayMethod::Get,
            format!("/v1/instances/{}/save-profiles", context.instance_id),
            None,
        ),
        CallKind::Current => (
            GatewayMethod::Get,
            format!("/v1/instances/{}/save-profile/current", context.instance_id),
            None,
        ),
        CallKind::Status => (
            GatewayMethod::Get,
            format!(
                "/v1/instances/{}/save-profile/operations/{}",
                context.instance_id, context.operation_id
            ),
            None,
        ),
        CallKind::Select => {
            let profile_id = profile_id(arguments)?;
            let baseline = baseline(arguments)?;
            let body = JsonValue::object([
                (String::from("baseline"), baseline),
                (String::from("profile_id"), JsonValue::string(profile_id)),
            ]);
            if body.to_json().len() > SAVE_PROFILE_MAX_BODY_BYTES {
                return Err("save-profile selection exceeds the request bound");
            }
            (
                GatewayMethod::Post,
                select_path(&context.instance_id),
                Some(body),
            )
        }
        CallKind::CreateDisposable => (
            GatewayMethod::Post,
            create_path(&context.instance_id),
            Some(JsonValue::object([])),
        ),
    };
    Ok(GatewayRequest {
        method,
        path,
        headers,
        body,
        correlation: Correlation {
            mcp_session_id: context.mcp_session_id.clone(),
            mcp_request_id: context.request_id.clone(),
        },
    })
}

fn identity<'a>(
    arguments: &'a BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<&'a str, &'static str> {
    arguments
        .get(key)
        .and_then(JsonValue::as_string)
        .filter(|value| !value.is_empty())
        .ok_or("save-profile identity is missing")
}

fn epoch(arguments: &BTreeMap<String, JsonValue>) -> Result<i64, &'static str> {
    match arguments.get("lease_epoch") {
        Some(JsonValue::Number(value)) if (0..=9_007_199_254_740_991).contains(value) => Ok(*value),
        _ => Err("lease_epoch is outside the save-profile bound"),
    }
}

fn profile_id(arguments: &BTreeMap<String, JsonValue>) -> Result<&str, &'static str> {
    let value = identity(arguments, "profile_id")?;
    if !valid_profile_id(value) {
        return Err("profile_id is unsafe or oversized");
    }
    Ok(value)
}

fn baseline(arguments: &BTreeMap<String, JsonValue>) -> Result<JsonValue, &'static str> {
    let Some(JsonValue::Object(object)) = arguments.get("baseline") else {
        return Err("baseline must be an object");
    };
    if object.len() != 2 || !object.contains_key("identity") || !object.contains_key("digest") {
        return Err("baseline requires exactly identity and digest");
    }
    let identity = object
        .get("identity")
        .and_then(JsonValue::as_string)
        .filter(|value| safe_authority_identity(value))
        .ok_or("baseline identity is unsafe or oversized")?;
    let digest = object
        .get("digest")
        .and_then(JsonValue::as_string)
        .filter(|value| is_digest(value))
        .ok_or("baseline digest must be 64 lowercase hexadecimal bytes")?;
    Ok(JsonValue::object([
        (String::from("digest"), JsonValue::string(digest)),
        (String::from("identity"), JsonValue::string(identity)),
    ]))
}

fn valid_profile_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && !value.contains("..")
        && !value.contains("://")
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
}

fn is_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

pub(super) fn safe_authority_identity(value: &str) -> bool {
    safe_gateway_identity(value)
}

pub(super) fn safe_gateway_identity(value: &str) -> bool {
    super::super::safe_header_value(value)
        && !value.contains('/')
        && !value.contains("..")
        && !value.contains("://")
}

fn safe_operation_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && !value.contains('/')
        && !value.contains("..")
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
}

fn select_path(instance_id: &str) -> String {
    format!("/v1/instances/{instance_id}/save-profile/select")
}

fn create_path(instance_id: &str) -> String {
    format!("/v1/instances/{instance_id}/save-profile/create-disposable")
}
