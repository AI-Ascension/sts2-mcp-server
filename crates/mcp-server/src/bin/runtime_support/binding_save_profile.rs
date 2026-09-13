// SPDX-License-Identifier: MIT

use sts2_mcp_server::{GatewayError, GatewayMethod, GatewayRequest, GatewayResponse, JsonValue};

pub(super) fn is_route(request: &GatewayRequest) -> bool {
    let Some(rest) = request.path.strip_prefix("/v1/instances/") else {
        return false;
    };
    let Some((instance, route)) = rest.split_once('/') else {
        return false;
    };
    !instance.is_empty()
        && (matches!(
            route,
            "save-profiles"
                | "save-profile/current"
                | "save-profile/select"
                | "save-profile/create-disposable"
        ) || route.starts_with("save-profile/operations/"))
}

pub(crate) fn classify(response: GatewayResponse) -> Result<GatewayResponse, GatewayError> {
    if (400..=599).contains(&response.status)
        && !matches!(response.status, 401 | 403 | 404)
        && super::is_save_profile_result(&response.body)
    {
        return Ok(response);
    }
    super::super::exchange::classify(response)
}

pub(super) fn response_kind(request: &GatewayRequest) -> bool {
    let Some(rest) = request.path.strip_prefix("/v1/instances/") else {
        return false;
    };
    let Some((_, route)) = rest.split_once('/') else {
        return false;
    };
    match (request.method, route) {
        (GatewayMethod::Get, "save-profiles" | "save-profile/current")
            if request.body.is_none() =>
        {
            true
        }
        (GatewayMethod::Post, "save-profile/select") => {
            request.body.as_ref().is_some_and(select_body_is_fixed)
        }
        (GatewayMethod::Post, "save-profile/create-disposable") => request
            .body
            .as_ref()
            .is_none_or(|body| matches!(body, JsonValue::Object(object) if object.is_empty())),
        (GatewayMethod::Get, route)
            if request.body.is_none()
                && route.starts_with("save-profile/operations/")
                && safe_operation_id(
                    route.strip_prefix("save-profile/operations/").unwrap_or(""),
                ) =>
        {
            true
        }
        _ => false,
    }
}

pub(super) fn headers_are_fixed(request: &GatewayRequest) -> bool {
    request.headers.keys().all(|name| {
        matches!(
            name.as_str(),
            "x-mcp-session-id"
                | "x-mcp-request-id"
                | "x-sts2-instance-id"
                | "x-sts2-session-id"
                | "x-sts2-lease-id"
                | "x-sts2-lease-epoch"
        )
    }) && request.headers.get("x-mcp-session-id") == Some(&request.correlation.mcp_session_id)
        && request.headers.get("x-mcp-request-id")
            == Some(&request.correlation.mcp_request_id.stable_text())
}

pub(super) fn body_is_fixed(request: &GatewayRequest) -> bool {
    let Some(rest) = request.path.strip_prefix("/v1/instances/") else {
        return false;
    };
    let Some((_, route)) = rest.split_once('/') else {
        return false;
    };
    match route {
        "save-profiles" | "save-profile/current" => request.body.is_none(),
        route if route.starts_with("save-profile/operations/") => request.body.is_none(),
        "save-profile/create-disposable" => request
            .body
            .as_ref()
            .is_none_or(|body| matches!(body, JsonValue::Object(object) if object.is_empty())),
        "save-profile/select" => request.body.as_ref().is_some_and(select_body_is_fixed),
        _ => false,
    }
}

pub(super) fn select_body_is_fixed(body: &JsonValue) -> bool {
    let JsonValue::Object(object) = body else {
        return false;
    };
    if object.len() != 2
        || !matches!(object.get("profile_id"), Some(JsonValue::String(profile)) if safe_profile_id(profile))
    {
        return false;
    }
    let Some(JsonValue::Object(baseline)) = object.get("baseline") else {
        return false;
    };
    baseline.len() == 2
        && matches!(
            (baseline.get("identity"), baseline.get("digest")),
            (
                Some(JsonValue::String(identity)),
                Some(JsonValue::String(digest))
            ) if safe_gateway_identity(identity) && is_digest(digest)
        )
}

fn safe_profile_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && !value.contains("..")
        && !value.contains("://")
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
}

fn safe_gateway_identity(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && !value.contains('/')
        && !value.contains("..")
        && !value.contains("://")
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
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

fn is_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
