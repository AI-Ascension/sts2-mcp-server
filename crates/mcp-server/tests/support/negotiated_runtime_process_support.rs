// SPDX-License-Identifier: MIT
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::net::{TcpListener, TcpStream};
use std::time::Duration;

use serde_json::{Value, json};

pub(crate) fn serve_gateway(listener: TcpListener) -> Result<(), String> {
    for step in 0..5 {
        let (mut stream, _) = super::accept_bounded(&listener)?;
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .map_err(|error| error.to_string())?;
        let request = super::read_request(&mut stream)?;
        let response = match step {
            0 => {
                super::assert_startup_identity(&request, "game-information-binding-discovery");
                assert_eq!(request.method, "POST");
                assert_eq!(
                    request.path,
                    "/v1/instances/instance-1/game-information/lookup-binding"
                );
                assert_eq!(
                    serde_json::from_slice::<Value>(&request.body).unwrap(),
                    json!({
                        "operation":"discovery",
                        "project_id":"proj-1",
                        "run_id":"run-42",
                        "episode_id":"episode-7",
                        "agent_id":"agent-3",
                        "authority_epoch":7
                    })
                );
                lookup_discovery_response()
            }
            1 => {
                super::assert_startup_identity(&request, "negotiated-capabilities-startup");
                assert_eq!(request.method, "GET");
                assert_eq!(
                    request.path,
                    "/v1/instances/instance-1/negotiated-capabilities"
                );
                assert!(request.body.is_empty());
                negotiated_snapshot()
            }
            2 => {
                super::assert_tool_identity(&request, "capabilities-call");
                assert_eq!(request.method, "GET");
                assert_eq!(
                    request.path,
                    "/v1/instances/instance-1/game-information/capabilities"
                );
                assert!(request.body.is_empty());
                let mut response: Value = serde_json::from_str(include_str!(
                    "../../../../protocol-artifact/game-information-query-v1/golden/capabilities-response.json"
                ))
                .unwrap();
                response["correlation_id"] = json!("capabilities-call");
                response
            }
            3 => {
                super::assert_tool_identity(&request, "state-call");
                assert_eq!(request.method, "GET");
                assert_eq!(request.path, "/v3/instances/instance-1/state");
                let body: Value = serde_json::from_slice(&request.body).unwrap();
                assert_eq!(body["kind"], "state_request");
                assert_eq!(body["instance_id"], "instance-1");
                assert_eq!(body["session_id"], "session-1");
                assert_eq!(body["lease_id"], "lease-1");
                assert_eq!(body["lease_epoch"], 1);
                assert_eq!(body["correlation_id"], "state-call");
                let mut response: Value = serde_json::from_str(include_str!(
                    "../../../../protocol-artifact/runtime-v3-gameplay/golden/state-response.json"
                ))
                .unwrap();
                response["correlation_id"] = json!("state-call");
                response["instance_id"] = json!("instance-1");
                response["session_id"] = json!("session-1");
                response["lease_id"] = json!("lease-1");
                response
            }
            4 => {
                super::assert_tool_identity(&request, "binding-call");
                assert_eq!(request.method, "POST");
                assert_eq!(
                    request.path,
                    "/v1/instances/instance-1/game-information/lookup-binding"
                );
                assert_eq!(
                    serde_json::from_slice::<Value>(&request.body).unwrap(),
                    json!({
                        "operation":"observe",
                        "project_id":"proj-1",
                        "run_id":"run-42",
                        "episode_id":"episode-7",
                        "agent_id":"agent-3",
                        "authority_epoch":7
                    })
                );
                let mut response: Value = serde_json::from_str(include_str!(
                    "../../../../protocol-artifact/game-information-lookup-binding-v1/golden/observation-response.json"
                ))
                .unwrap();
                response["correlation_id"] = json!("binding-call");
                response
            }
            _ => unreachable!(),
        };
        super::write_json_response(&mut stream, &response)?;
    }
    Ok(())
}

pub(crate) fn serve_invalid_snapshot(listener: TcpListener, mutation: &str) -> Result<(), String> {
    let (mut discovery, _) = super::accept_bounded(&listener)?;
    let request = super::read_request(&mut discovery)?;
    super::assert_startup_identity(&request, "game-information-binding-discovery");
    assert_eq!(request.method, "POST");
    assert_eq!(
        request.path,
        "/v1/instances/instance-1/game-information/lookup-binding"
    );
    super::write_json_response(&mut discovery, &lookup_discovery_response())?;

    let (mut snapshot_request, _) = super::accept_bounded(&listener)?;
    let request = super::read_request(&mut snapshot_request)?;
    super::assert_startup_identity(&request, "negotiated-capabilities-startup");
    assert_eq!(request.method, "GET");
    assert_eq!(
        request.path,
        "/v1/instances/instance-1/negotiated-capabilities"
    );
    let mut snapshot = negotiated_snapshot();
    match mutation {
        "stale-lease" => snapshot["lease_epoch"] = json!(2),
        "schema" => snapshot["schema_version"] = json!("unrecognized-contract"),
        "identity" => snapshot["caller_id"] = json!("foreign-caller"),
        "producer-run" => snapshot["producer"]["run_id"] = json!("foreign-run"),
        "witness" => {
            snapshot["lookup_binding_witness"]["binding_id"] =
                json!("0000000000000000000000000000000000000000000000000000000000000000")
        }
        "duplicate-offer" => {
            let duplicate = snapshot["offers"][0].clone();
            snapshot["offers"].as_array_mut().unwrap().push(duplicate);
        }
        "recovery" => {
            snapshot["runtime_v3_baseline_witness"]["recovery_kinds"] =
                json!(["reobserve", "reconcile", "release_lease"])
        }
        "oversized" => {
            let body = format!("{}{}", snapshot, " ".repeat(16 * 1024 + 1));
            return super::write_raw_json_response(&mut snapshot_request, &body);
        }
        _ => return Err(String::from("unknown startup mutation")),
    }
    super::write_json_response(&mut snapshot_request, &snapshot)
}

pub(crate) fn serve_wire_limit_case(listener: TcpListener, case: &str) -> Result<(), String> {
    let mut snapshot_request = serve_startup(&listener)?;
    let mut snapshot = negotiated_snapshot();
    let state = snapshot["offers"]
        .as_array_mut()
        .and_then(|offers| {
            offers
                .iter_mut()
                .find(|offer| offer["operation"] == "runtime_v3.state")
        })
        .ok_or("runtime_v3.state offer missing")?;
    match case {
        "request" => state["wire_limits"]["max_request_bytes"] = json!(1),
        "response" => state["wire_limits"]["max_response_bytes"] = json!(1),
        "content" => state["content_limits"]["max_content_bytes"] = json!(1),
        _ => return Err(String::from("unknown wire-limit case")),
    }
    super::write_json_response(&mut snapshot_request, &snapshot)?;
    drop(snapshot_request);

    match case {
        "request" => {
            listener
                .set_nonblocking(true)
                .map_err(|error| error.to_string())?;
            let deadline = std::time::Instant::now() + Duration::from_secs(3);
            loop {
                match listener.accept() {
                    Ok(_) => return Err(String::from("request-over-limit reached Gateway")),
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        if std::time::Instant::now() >= deadline {
                            return Ok(());
                        }
                        std::thread::sleep(Duration::from_millis(10));
                    }
                    Err(error) => return Err(error.to_string()),
                }
            }
        }
        "response" => {
            let (mut stream, _) = super::accept_bounded(&listener)?;
            let request = super::read_request(&mut stream)?;
            super::assert_tool_identity(&request, "state-call");
            assert_eq!(request.method, "GET");
            assert_eq!(request.path, "/v3/instances/instance-1/state");
            let response = state_response("state-call");
            super::write_json_response(&mut stream, &response)
        }
        "content" => {
            let (mut state_stream, _) = super::accept_bounded(&listener)?;
            let state_request = super::read_request(&mut state_stream)?;
            super::assert_tool_identity(&state_request, "state-call");
            assert_eq!(state_request.method, "GET");
            assert_eq!(state_request.path, "/v3/instances/instance-1/state");
            super::write_json_response(&mut state_stream, &state_response("state-call"))?;

            let (mut capabilities_stream, _) = super::accept_bounded(&listener)?;
            let capabilities_request = super::read_request(&mut capabilities_stream)?;
            super::assert_tool_identity(&capabilities_request, "capabilities-call");
            assert_eq!(capabilities_request.method, "GET");
            assert_eq!(
                capabilities_request.path,
                "/v1/instances/instance-1/game-information/capabilities"
            );
            let mut response: Value = serde_json::from_str(include_str!(
                "../../../../protocol-artifact/game-information-query-v1/golden/capabilities-response.json"
            ))
            .unwrap();
            response["correlation_id"] = json!("capabilities-call");
            super::write_json_response(&mut capabilities_stream, &response)
        }
        _ => unreachable!(),
    }
}

fn serve_startup(listener: &TcpListener) -> Result<TcpStream, String> {
    let (mut discovery, _) = super::accept_bounded(listener)?;
    let request = super::read_request(&mut discovery)?;
    super::assert_startup_identity(&request, "game-information-binding-discovery");
    assert_eq!(request.method, "POST");
    assert_eq!(
        request.path,
        "/v1/instances/instance-1/game-information/lookup-binding"
    );
    super::write_json_response(&mut discovery, &lookup_discovery_response())?;

    let (mut snapshot_request, _) = super::accept_bounded(listener)?;
    let request = super::read_request(&mut snapshot_request)?;
    super::assert_startup_identity(&request, "negotiated-capabilities-startup");
    assert_eq!(request.method, "GET");
    assert_eq!(
        request.path,
        "/v1/instances/instance-1/negotiated-capabilities"
    );
    Ok(snapshot_request)
}

fn state_response(correlation: &str) -> Value {
    let mut response: Value = serde_json::from_str(include_str!(
        "../../../../protocol-artifact/runtime-v3-gameplay/golden/state-response.json"
    ))
    .unwrap();
    response["correlation_id"] = json!(correlation);
    response["instance_id"] = json!("instance-1");
    response["session_id"] = json!("session-1");
    response["lease_id"] = json!("lease-1");
    response
}

pub(crate) fn lookup_discovery_response() -> Value {
    let mut response: Value = serde_json::from_str(include_str!(
        "../../../../protocol-artifact/game-information-lookup-binding-v1/golden/discovery-response.json"
    ))
    .unwrap();
    response["correlation_id"] = json!("game-information-binding-discovery");
    response
}

pub(crate) fn negotiated_snapshot() -> Value {
    let mut offers = Vec::new();
    for operation in [
        "game_information.capabilities",
        "game_information.lookup_binding.discovery",
        "game_information.lookup_binding.observe",
        "runtime_v3.state",
        "runtime_v3.legal_actions",
        "runtime_v3.wait",
        "runtime_v3.reobserve",
    ] {
        let (revision, request_bytes, response_bytes, content_bytes) =
            if operation.starts_with("game_information.lookup_binding.") {
                (
                    "game-information-lookup-binding-v1",
                    16_384,
                    262_144,
                    262_144,
                )
            } else if operation.starts_with("game_information.") {
                ("game-information-query-v1", 0, 8_192, 8_192)
            } else {
                ("runtime-v3-gameplay", 16_384, 131_072, 131_072)
            };
        offers.push(json!({
            "operation":operation,
            "revision":revision,
            "required_scope":"read",
            "scope":["read"],
            "wire_limits":{
                "max_request_bytes":request_bytes,
                "max_response_bytes":response_bytes
            },
            "content_limits":{
                "max_content_bytes":content_bytes,
                "max_page_items":1
            }
        }));
    }
    json!({
        "schema_version":"sts2-gateway-negotiated-capabilities-v1",
        "gateway_revision":"sts2-gateway-negotiated-capabilities-v1",
        "correlation_id":"negotiated-capabilities-startup",
        "instance_id":"instance-1",
        "caller_id":"harness",
        "session_id":"session-1",
        "mcp_session_id":"mcp-session-1",
        "lease_id":"lease-1",
        "lease_epoch":1,
        "caller_scopes":["read"],
        "producer":{
            "profile":"game-information-query-v1",
            "schema_digest":super::GAME_INFORMATION_SCHEMA,
            "content_manifest_id":"content-1",
            "run_id":"run-42"
        },
        "lookup_binding_witness":{
            "profile":"game-information-lookup-binding-v1",
            "schema_digest":super::LOOKUP_SCHEMA,
            "binding_id":super::BINDING_ID,
            "content_manifest_id":"content-1",
            "authority_epoch":7
        },
        "runtime_v3_baseline_witness":{
            "profile":"runtime-v3-gameplay",
            "schema_digest":super::RUNTIME_SCHEMA,
            "configured_state_probe":true,
            "recovery_kinds":["reobserve","reconcile"]
        },
        "offers":offers
    })
}
