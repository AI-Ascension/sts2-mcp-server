// SPDX-License-Identifier: MIT
#![allow(clippy::unwrap_used)] // Fail immediately if the fixture frame cannot be built.

use super::*;
use crate::gateway::GatewayError;
use crate::recovery_frame::{RECOVERY_SCHEMA_DIGEST, utc_timestamp_now, uuid_v4};
use crate::{GatewayResponse, McpServer, ToolCatalog};

const CALLER: &str = "00000000-0000-4000-8000-00000000000a";
const MCP_SESSION: &str = "mcp-session-1";

/// How the fixture answers: either a fixed body, or a complete lookup frame
/// whose correlation echoes the request the mapping layer just built.
#[derive(Clone)]
enum Answer {
    Fixed(GatewayResponse),
    EchoLookup { status: u16, state: &'static str },
}

#[derive(Clone)]
struct RecordingGateway {
    answer: Answer,
    request: Option<GatewayRequest>,
    principal: Option<String>,
}

impl GatewayAdapter for RecordingGateway {
    fn forward(&mut self, request: GatewayRequest) -> Result<GatewayResponse, GatewayError> {
        let correlation = request
            .body
            .as_ref()
            .and_then(JsonValue::as_object)
            .and_then(|frame| frame.get("correlation_id"))
            .and_then(JsonValue::as_string)
            .unwrap_or_default()
            .to_owned();
        self.request = Some(request);
        Ok(match self.answer {
            Answer::Fixed(ref response) => response.clone(),
            Answer::EchoLookup { status, state } => GatewayResponse {
                status,
                body: response_frame(&correlation, state, status == 503),
            },
        })
    }

    fn frame_principal(&self) -> Option<&str> {
        self.principal.as_deref()
    }
}

fn context() -> JsonValue {
    JsonValue::object([
        (
            "deployment_id".to_owned(),
            JsonValue::string(uuid_v4().unwrap()),
        ),
        (
            "instance_id".to_owned(),
            JsonValue::string(uuid_v4().unwrap()),
        ),
        (
            "instance_incarnation".to_owned(),
            JsonValue::string(uuid_v4().unwrap()),
        ),
        ("boot_id".to_owned(), JsonValue::string(uuid_v4().unwrap())),
        ("authority_generation".to_owned(), JsonValue::Number(1)),
        ("lease_id".to_owned(), JsonValue::string(uuid_v4().unwrap())),
        ("lease_epoch".to_owned(), JsonValue::Number(1)),
    ])
}

fn operation_ref() -> JsonValue {
    JsonValue::object([
        (
            "operation_id".to_owned(),
            JsonValue::string(uuid_v4().unwrap()),
        ),
        (
            "payload_digest".to_owned(),
            JsonValue::string("0".repeat(64)),
        ),
        ("original_context".to_owned(), context()),
    ])
}

fn lookup_payload() -> JsonValue {
    JsonValue::object([
        ("operation".to_owned(), operation_ref()),
        (
            "lookup_scope".to_owned(),
            JsonValue::string("historical_read"),
        ),
    ])
}

fn result(status: &str, retryable: bool, retry_after: JsonValue) -> JsonValue {
    JsonValue::object([
        ("status".to_owned(), JsonValue::string(status)),
        ("retryable".to_owned(), JsonValue::Bool(retryable)),
        ("retry_after_seconds".to_owned(), retry_after),
    ])
}

/// One complete gateway response frame, as the sideband route emits it.
fn response_frame(correlation_id: &str, status: &str, retryable: bool) -> JsonValue {
    JsonValue::object([
        (
            "contract".to_owned(),
            JsonValue::string(crate::recovery_frame::RECOVERY_CONTRACT),
        ),
        (
            "schema_digest".to_owned(),
            JsonValue::string(RECOVERY_SCHEMA_DIGEST),
        ),
        (
            "message_id".to_owned(),
            JsonValue::string(uuid_v4().unwrap()),
        ),
        (
            "correlation_id".to_owned(),
            JsonValue::string(correlation_id),
        ),
        ("sent_at".to_owned(), JsonValue::string(utc_timestamp_now())),
        (
            "actor".to_owned(),
            JsonValue::object([
                ("principal_id".to_owned(), JsonValue::string(CALLER)),
                ("role".to_owned(), JsonValue::string("gateway")),
            ]),
        ),
        (
            "auth".to_owned(),
            JsonValue::object([
                ("principal_id".to_owned(), JsonValue::string(CALLER)),
                ("capability".to_owned(), JsonValue::string("recovery_read")),
                ("proof".to_owned(), JsonValue::Null),
            ]),
        ),
        (
            "kind".to_owned(),
            JsonValue::string("operation_lookup_response"),
        ),
        (
            "payload".to_owned(),
            JsonValue::object([
                (
                    "result".to_owned(),
                    result(
                        status,
                        retryable,
                        if retryable {
                            JsonValue::Number(1)
                        } else {
                            JsonValue::Null
                        },
                    ),
                ),
                ("operation".to_owned(), operation_ref()),
                ("mutation_authorized".to_owned(), JsonValue::Bool(false)),
            ]),
        ),
    ])
}

/// The sideband must echo the correlation of the request it answers, so the
/// fixture is built from the frame the mapping layer actually produced.
fn server_with(answer: Answer, principal: Option<&str>) -> McpServer<RecordingGateway> {
    McpServer::with_catalog_and_sessions(
        RecordingGateway {
            answer,
            request: None,
            principal: principal.map(str::to_owned),
        },
        ToolCatalog::watchdog_recovery(),
        "session-1",
        MCP_SESSION,
    )
}

fn call_frame(name: &str, arguments: JsonValue) -> String {
    JsonValue::object([
        ("jsonrpc".to_owned(), JsonValue::string("2.0")),
        ("id".to_owned(), JsonValue::string("corr:1")),
        ("method".to_owned(), JsonValue::string("tools/call")),
        (
            "params".to_owned(),
            JsonValue::object([
                ("name".to_owned(), JsonValue::string(name)),
                ("arguments".to_owned(), arguments),
            ]),
        ),
    ])
    .to_json()
}

fn lookup_arguments() -> JsonValue {
    JsonValue::object([
        ("mcp_session_id".to_owned(), JsonValue::string(MCP_SESSION)),
        ("payload".to_owned(), lookup_payload()),
    ])
}

#[test]
fn forwards_the_built_frame_and_surfaces_the_response_verbatim() -> Result<(), String> {
    // An unresolved lookup answers 503 together with a complete frame, and the
    // frame must reach the caller unchanged rather than becoming an error.
    let mut server = server_with(
        Answer::EchoLookup {
            status: 503,
            state: "MAY_HAVE_BEEN_DISPATCHED",
        },
        Some(CALLER),
    );
    let output = server.handle_frame(&call_frame("watchdog.operation_lookup", lookup_arguments()));
    if !output.contains("\"isError\":false") {
        return Err(format!(
            "a complete unresolved frame must surface: {output}"
        ));
    }
    if !output.contains("MAY_HAVE_BEEN_DISPATCHED") || !output.contains("mutation_authorized") {
        return Err(format!(
            "the frame payload was not forwarded verbatim: {output}"
        ));
    }
    let request = server
        .gateway()
        .request
        .as_ref()
        .ok_or_else(|| String::from("gateway was not called"))?;
    if request.path != "/v1/recovery/operation/lookup" || request.method != GatewayMethod::Post {
        return Err(String::from("recovery route was not the lookup route"));
    }
    let body = request
        .body
        .clone()
        .ok_or_else(|| String::from("recovery frame was omitted"))?;
    let correlation = body
        .as_object()
        .and_then(|frame| frame.get("correlation_id"))
        .and_then(JsonValue::as_string)
        .ok_or_else(|| String::from("recovery frame has no correlation"))?
        .to_owned();
    if request
        .headers
        .get("x-sts2-correlation-id")
        .map(String::as_str)
        != Some(correlation.as_str())
    {
        return Err(String::from(
            "the transport correlation did not carry the frame correlation",
        ));
    }
    Ok(())
}

#[test]
fn a_body_that_is_not_a_recovery_frame_is_refused() -> Result<(), String> {
    let mut server = server_with(
        Answer::Fixed(GatewayResponse {
            status: 400,
            body: JsonValue::object([(
                "error_code".to_owned(),
                JsonValue::string(String::from("recovery_capability_forbidden")),
            )]),
        }),
        Some(CALLER),
    );
    let output = server.handle_frame(&call_frame("watchdog.operation_lookup", lookup_arguments()));
    if !output.contains("watchdog_recovery_frame_invalid") {
        return Err(format!("a short-circuit body must be refused: {output}"));
    }
    Ok(())
}

#[test]
fn a_missing_principal_never_reaches_the_gateway() -> Result<(), String> {
    let mut server = server_with(
        Answer::Fixed(GatewayResponse {
            status: 200,
            body: JsonValue::Null,
        }),
        None,
    );
    let output = server.handle_frame(&call_frame("watchdog.operation_lookup", lookup_arguments()));
    if !output.contains("watchdog_recovery_principal_unavailable") {
        return Err(format!("an unbound principal must be refused: {output}"));
    }
    if server.gateway().request.is_some() {
        return Err(String::from("a refused call reached the gateway"));
    }
    Ok(())
}

#[test]
fn advertised_but_unwired_tools_are_refused_without_a_forward() -> Result<(), String> {
    for name in [
        "watchdog.bootstrap",
        "watchdog.operation_intent",
        "watchdog.operation_dispatch",
    ] {
        let mut server = server_with(
            Answer::Fixed(GatewayResponse {
                status: 200,
                body: JsonValue::Null,
            }),
            Some(CALLER),
        );
        let output = server.handle_frame(&call_frame(name, lookup_arguments()));
        if !output.contains("watchdog_recovery_route_not_installed") {
            return Err(format!("{name} was not refused: {output}"));
        }
        if server.gateway().request.is_some() {
            return Err(format!("{name} reached the gateway"));
        }
    }
    Ok(())
}

#[test]
fn a_malformed_payload_is_refused_before_the_gateway() -> Result<(), String> {
    let mut payload = lookup_payload();
    if let JsonValue::Object(object) = &mut payload {
        object.insert(
            "lookup_scope".to_owned(),
            JsonValue::string("authoritative_write"),
        );
    }
    let mut server = server_with(
        Answer::Fixed(GatewayResponse {
            status: 200,
            body: JsonValue::Null,
        }),
        Some(CALLER),
    );
    let output = server.handle_frame(&call_frame(
        "watchdog.operation_lookup",
        JsonValue::object([
            ("mcp_session_id".to_owned(), JsonValue::string(MCP_SESSION)),
            ("payload".to_owned(), payload),
        ]),
    ));
    if !output.contains("historical read") {
        return Err(format!("a write scope must be refused: {output}"));
    }
    if server.gateway().request.is_some() {
        return Err(String::from("a malformed payload reached the gateway"));
    }
    Ok(())
}

#[test]
fn a_foreign_session_argument_is_refused() -> Result<(), String> {
    let mut server = server_with(
        Answer::Fixed(GatewayResponse {
            status: 200,
            body: JsonValue::Null,
        }),
        Some(CALLER),
    );
    let output = server.handle_frame(&call_frame(
        "watchdog.operation_lookup",
        JsonValue::object([
            (
                "mcp_session_id".to_owned(),
                JsonValue::string("mcp-session-other"),
            ),
            ("payload".to_owned(), lookup_payload()),
        ]),
    ));
    if !output.contains("not the active MCP session") {
        return Err(format!("a foreign session must be refused: {output}"));
    }
    Ok(())
}

#[test]
fn a_response_from_another_correlation_is_refused() -> Result<(), String> {
    let frame = response_frame(&uuid_v4().unwrap(), "SETTLED", false);
    let mut server = server_with(
        Answer::Fixed(GatewayResponse {
            status: 200,
            body: frame,
        }),
        Some(CALLER),
    );
    let output = server.handle_frame(&call_frame("watchdog.operation_lookup", lookup_arguments()));
    if !output.contains("does not echo the request") {
        return Err(format!("a foreign correlation must be refused: {output}"));
    }
    Ok(())
}

#[test]
fn the_catalog_advertises_the_closed_nine_tool_surface() {
    let catalog = ToolCatalog::watchdog_recovery();
    let names: Vec<&str> = catalog
        .tools()
        .iter()
        .map(|tool| tool.name.as_str())
        .collect();
    assert_eq!(
        names,
        [
            "watchdog.bootstrap",
            "watchdog.host_fence",
            "watchdog.lease_acquire",
            "watchdog.lease_renew",
            "watchdog.lease_revoke",
            "watchdog.operation_intent",
            "watchdog.operation_dispatch",
            "watchdog.operation_lookup",
            "watchdog.operation_reconcile",
        ]
    );
    assert_eq!(catalog.revision, crate::catalog::WATCHDOG_RECOVERY_PROFILE);
}
