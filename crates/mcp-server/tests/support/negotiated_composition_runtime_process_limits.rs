// SPDX-License-Identifier: MIT
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::io::BufReader;
use std::net::TcpListener;
use std::process::{Child, ChildStdin, ChildStdout};

use serde_json::{Value, json};

use super::{LOOKUP_REQUEST, mcp_request, spawn_mcp, support, wait_child};

#[test]
fn shipped_stdio_request_over_wire_cap_refuses_before_gateway_io() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap().to_string();
    let gateway = std::thread::spawn(move || support::serve_wire_limit_case(listener, "request"));
    let mut child = spawn_mcp(address, LOOKUP_REQUEST);
    let mut stdin = child.0.stdin.take().unwrap();
    let mut stdout = BufReader::new(child.0.stdout.take().unwrap());
    initialize_process(&mut stdin, &mut stdout, &mut child.0);

    let response = mcp_request(&mut stdin, &mut stdout, &mut child.0, state_call());
    assert_eq!(response["result"]["isError"], true, "{response}");

    drop(stdin);
    wait_child(&mut child.0);
    gateway.join().unwrap().unwrap();
}

#[test]
fn shipped_stdio_response_over_wire_cap_is_rejected() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap().to_string();
    let gateway = std::thread::spawn(move || support::serve_wire_limit_case(listener, "response"));
    let mut child = spawn_mcp(address, LOOKUP_REQUEST);
    let mut stdin = child.0.stdin.take().unwrap();
    let mut stdout = BufReader::new(child.0.stdout.take().unwrap());
    initialize_process(&mut stdin, &mut stdout, &mut child.0);

    let response = mcp_request(&mut stdin, &mut stdout, &mut child.0, state_call());
    assert_eq!(response["result"]["isError"], true, "{response}");

    drop(stdin);
    wait_child(&mut child.0);
    gateway.join().unwrap().unwrap();
}

#[test]
fn shipped_stdio_content_cap_isolated_per_operation() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap().to_string();
    let gateway = std::thread::spawn(move || support::serve_wire_limit_case(listener, "content"));
    let mut child = spawn_mcp(address, LOOKUP_REQUEST);
    let mut stdin = child.0.stdin.take().unwrap();
    let mut stdout = BufReader::new(child.0.stdout.take().unwrap());
    initialize_process(&mut stdin, &mut stdout, &mut child.0);

    let state = mcp_request(&mut stdin, &mut stdout, &mut child.0, state_call());
    assert_eq!(state["result"]["isError"], true, "{state}");
    let capabilities = mcp_request(
        &mut stdin,
        &mut stdout,
        &mut child.0,
        json!({
            "jsonrpc":"2.0",
            "id":"capabilities-call",
            "method":"tools/call",
            "params":{
                "name":"sts2.game_information_capabilities",
                "arguments":{
                    "instance_id":"instance-1",
                    "mcp_session_id":"mcp-session-1",
                    "lease_id":"lease-1",
                    "lease_epoch":1
                }
            }
        }),
    );
    assert_eq!(capabilities["result"]["isError"], false, "{capabilities}");

    drop(stdin);
    wait_child(&mut child.0);
    gateway.join().unwrap().unwrap();
}

fn initialize_process(
    stdin: &mut ChildStdin,
    stdout: &mut BufReader<ChildStdout>,
    child: &mut Child,
) {
    let initialized = mcp_request(
        stdin,
        stdout,
        child,
        json!({
            "jsonrpc":"2.0",
            "id":"initialize",
            "method":"initialize",
            "params":{
                "protocolVersion":"2025-06-18",
                "capabilities":{},
                "clientInfo":{"name":"negotiated-startup-test","version":"1"}
            }
        }),
    );
    assert!(initialized["result"].is_object(), "{initialized}");
}

fn state_call() -> Value {
    json!({
        "jsonrpc":"2.0",
        "id":"state-call",
        "method":"tools/call",
        "params":{
            "name":"sts2.observe",
            "arguments":{
                "instance_id":"instance-1",
                "mcp_session_id":"mcp-session-1",
                "lease_id":"lease-1",
                "lease_epoch":1,
                "generation":0
            }
        }
    })
}
