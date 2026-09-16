// SPDX-License-Identifier: MIT
#![allow(clippy::unwrap_used)] // Disposable loopback peer fails the test immediately.

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener};
use std::thread;

use sts2_mcp_server::{
    Correlation, GatewayAdapter, GatewayMethod, GatewayRequest, JsonValue, RequestId, parse_json,
};

use super::{RuntimeConfig, RuntimeGatewayAdapter};
use serde_json::Value;

const FRAMES: &str =
    include_str!("../../../../../protocol-artifact/exact-restore-v1/golden/frames.json");

fn frames() -> Vec<Value> {
    serde_json::from_str::<Value>(FRAMES).unwrap()["frames"]
        .as_array()
        .unwrap()
        .clone()
}

fn wrapper(frame: Value, request: bool) -> JsonValue {
    let message_id = frame["message_id"].as_str().unwrap();
    let correlation_id = frame["correlation_id"].as_str().unwrap();
    let role = if request { "harness" } else { "gateway" };
    JsonValue::object([
        (
            String::from("contract"),
            JsonValue::string("sts2-exact-restore-gateway-v1"),
        ),
        (
            String::from("schema_digest"),
            JsonValue::string(sts2_mcp_server::EXACT_RESTORE_GATEWAY_SCHEMA_DIGEST),
        ),
        (String::from("message_id"), JsonValue::string(message_id)),
        (
            String::from("correlation_id"),
            JsonValue::string(correlation_id),
        ),
        (
            String::from("actor"),
            JsonValue::object([
                (String::from("principal_id"), JsonValue::string("harness")),
                (String::from("role"), JsonValue::string(role)),
            ]),
        ),
        (
            String::from("auth"),
            JsonValue::object([
                (String::from("principal_id"), JsonValue::string("harness")),
                (
                    String::from("capability"),
                    JsonValue::string("exact_restore"),
                ),
                (String::from("proof"), JsonValue::Null),
            ]),
        ),
        (
            String::from("kind"),
            JsonValue::string(if request {
                "exact_restore_request"
            } else {
                "exact_restore_response"
            }),
        ),
        (
            String::from("payload"),
            JsonValue::object([(
                String::from("frame"),
                parse_json(&frame.to_string()).unwrap(),
            )]),
        ),
    ])
}

fn request(index: usize, route: &str) -> GatewayRequest {
    let ids = frames();
    GatewayRequest {
        method: GatewayMethod::Post,
        path: format!("/v1/exact-restore/{route}"),
        headers: BTreeMap::from([
            (
                String::from("x-mcp-session-id"),
                String::from("mcp-session-1"),
            ),
            (String::from("x-mcp-correlation-id"), format!("mcp-{index}")),
            (
                String::from("x-sts2-instance-id"),
                String::from("02ab8278-c166-4557-bd6d-8f7575484a55"),
            ),
            (
                String::from("x-sts2-session-id"),
                String::from("session-example"),
            ),
            (
                String::from("x-sts2-lease-id"),
                String::from("c0f5a147-1ba1-4a1c-9a25-1b46935403ef"),
            ),
            (String::from("x-sts2-lease-epoch"), String::from("8")),
        ]),
        body: Some(wrapper(ids[index].clone(), true)),
        correlation: Correlation {
            mcp_session_id: String::from("mcp-session-1"),
            mcp_request_id: RequestId::String(format!("mcp-{index}")),
        },
    }
}

fn read_http_request(stream: &mut std::net::TcpStream) -> (String, String, String, Vec<u8>) {
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 2048];
    let header_end = loop {
        let count = stream.read(&mut buffer).unwrap();
        assert_ne!(count, 0);
        bytes.extend_from_slice(&buffer[..count]);
        if let Some(offset) = bytes.windows(4).position(|part| part == b"\r\n\r\n") {
            break offset;
        }
    };
    let header = String::from_utf8(bytes[..header_end].to_vec()).unwrap();
    let mut lines = header.lines();
    let mut first = lines.next().unwrap().split_whitespace();
    let method = first.next().unwrap();
    let path = first.next().unwrap();
    let content_length: usize = lines
        .find_map(|line| {
            line.split_once(':')
                .filter(|(name, _)| name.eq_ignore_ascii_case("content-length"))
                .map(|(_, value)| value.trim().parse().unwrap())
        })
        .unwrap();
    let body_start = header_end + 4;
    while bytes.len() - body_start < content_length {
        let count = stream.read(&mut buffer).unwrap();
        assert_ne!(count, 0);
        bytes.extend_from_slice(&buffer[..count]);
    }
    (
        method.to_owned(),
        path.to_owned(),
        header,
        bytes[body_start..body_start + content_length].to_vec(),
    )
}

fn config(address: SocketAddr) -> RuntimeConfig {
    RuntimeConfig {
        gateway_address: address,
        gateway_token: String::new(),
        recovery_token: Some(String::from("recovery-secret")),
        exact_restore_profile: true,
        instance_id: String::from("02ab8278-c166-4557-bd6d-8f7575484a55"),
        caller_id: String::from("harness"),
        session_id: String::from("session-example"),
        mcp_session_id: String::from("mcp-session-1"),
        lease_id: String::from("c0f5a147-1ba1-4a1c-9a25-1b46935403ef"),
        lease_epoch: 8,
        coop_native_peer_binding: None,
    }
}

#[test]
fn five_fixed_routes_forward_authenticated_frames_and_validate_correlated_responses() {
    let routes = ["begin", "chunk", "finish", "commit", "lookup"];
    let request_indices = [0, 2, 3, 4, 6];
    let response_indices = [12, 14, 15, 16, 18];
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let response_frames = frames();
    let worker = thread::spawn(move || {
        for ((route, request_index), response_index) in routes
            .into_iter()
            .zip(request_indices)
            .zip(response_indices)
        {
            let (mut stream, _) = listener.accept().unwrap();
            let (method, path, header, request_body) = read_http_request(&mut stream);
            assert_eq!(method, "POST");
            assert_eq!(path, format!("/v1/exact-restore/{route}"));
            assert!(header.contains("Authorization: Bearer recovery-secret\r\n"));
            assert!(header.contains("x-sts2-recovery-capability: exact_restore\r\n"));
            assert!(header.contains(&format!(
                "x-sts2-correlation-id: {}\r\n",
                response_frames[request_index]["correlation_id"]
                    .as_str()
                    .unwrap()
            )));
            let request_json = serde_json::from_slice::<Value>(&request_body).unwrap();
            assert_eq!(
                request_json,
                serde_json::from_str::<Value>(
                    &wrapper(response_frames[request_index].clone(), true).to_json(),
                )
                .unwrap()
            );
            let response_body = wrapper(response_frames[response_index].clone(), false).to_json();
            stream
                .write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                        response_body.len(),
                        response_body
                    )
                    .as_bytes(),
                )
                .unwrap();
        }
    });

    let mut adapter = RuntimeGatewayAdapter::new(config(address), 16 * 1024);
    for (index, route) in routes.into_iter().enumerate() {
        let response = adapter
            .forward(request(request_indices[index], route))
            .unwrap();
        assert_eq!(response.status, 200);
        assert_eq!(
            serde_json::from_str::<Value>(&response.body.to_json()).unwrap()["kind"],
            serde_json::json!("exact_restore_response")
        );
    }
    worker.join().unwrap();
}

#[test]
fn commit_uncertainty_blocks_a_second_effect_request_until_lookup() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let response_frame = frames()[17].clone();
    let lookup_request = frames()[6].clone();
    let lookup_response = frames()[18].clone();
    let worker = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let (method, path, _, request_body) = read_http_request(&mut stream);
        assert_eq!(method, "POST");
        assert_eq!(path, "/v1/exact-restore/commit");
        let request_body = serde_json::from_slice::<Value>(&request_body).unwrap();
        assert_eq!(
            request_body["auth"]["capability"],
            serde_json::json!("exact_restore")
        );
        let response = wrapper(response_frame, false).to_json();
        stream
            .write_all(
                format!(
                    "HTTP/1.1 202 Accepted\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                    response.len(),
                    response
                )
                .as_bytes(),
            )
            .unwrap();

        let (mut lookup_stream, _) = listener.accept().unwrap();
        let (method, path, _, request_body) = read_http_request(&mut lookup_stream);
        assert_eq!(method, "POST");
        assert_eq!(path, "/v1/exact-restore/lookup");
        let request_json = serde_json::from_slice::<Value>(&request_body).unwrap();
        assert_eq!(
            request_json,
            serde_json::from_str::<Value>(&wrapper(lookup_request, true).to_json(),).unwrap()
        );
        let response = wrapper(lookup_response, false).to_json();
        lookup_stream
            .write_all(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                    response.len(),
                    response
                )
                .as_bytes(),
            )
            .unwrap();
    });
    let mut adapter = RuntimeGatewayAdapter::new(config(address), 16 * 1024);
    assert!(adapter.forward(request(5, "commit")).is_ok());
    assert_eq!(
        adapter.forward(request(5, "commit")),
        Err(sts2_mcp_server::GatewayError::Rejected)
    );
    let lookup_result = adapter.forward(request(6, "lookup"));
    assert!(lookup_result.is_ok(), "{lookup_result:?}");
    worker.join().unwrap();
}

#[test]
fn rejects_foreign_owner_and_schema_before_opening_gateway_socket() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    drop(listener);
    let mut adapter = RuntimeGatewayAdapter::new(config(address), 16 * 1024);
    let mut foreign = request(0, "begin");
    if let Some(JsonValue::Object(wrapper)) = &mut foreign.body
        && let Some(JsonValue::Object(payload)) = wrapper.get_mut("payload")
        && let Some(JsonValue::Object(frame)) = payload.get_mut("frame")
        && let Some(JsonValue::Object(body)) = frame.get_mut("payload")
        && let Some(JsonValue::Object(expected_owner)) = body.get_mut("expected_owner")
    {
        expected_owner.insert(String::from("lease_id"), JsonValue::string("foreign-lease"));
    }
    assert_eq!(
        adapter.forward(foreign),
        Err(sts2_mcp_server::GatewayError::Rejected)
    );

    let mut malformed = request(0, "begin");
    if let Some(JsonValue::Object(wrapper)) = &mut malformed.body {
        wrapper.insert(String::from("new_field"), JsonValue::Null);
    }
    assert_eq!(
        adapter.forward(malformed),
        Err(sts2_mcp_server::GatewayError::Rejected)
    );

    let mut wrong_correlation = request(0, "begin");
    wrong_correlation.headers.insert(
        String::from("x-mcp-correlation-id"),
        String::from("mcp-another-request"),
    );
    assert_eq!(
        adapter.forward(wrong_correlation),
        Err(sts2_mcp_server::GatewayError::Rejected)
    );
}
