// SPDX-License-Identifier: MIT
#![allow(clippy::unwrap_used)]

use super::super::tests::{config, request};
use super::super::*;
use sts2_mcp_server::GatewayAdapter;

#[test]
fn checkpoint_reference_executable_requires_full_authority_and_fixed_route() {
    let mut valid = request();
    valid.path = "/v1/instances/instance/checkpoint-reference".into();
    valid
        .headers
        .insert("x-sts2-caller-id".into(), "caller".into());
    assert_eq!(admit(&config(), &valid), Ok(()));
    for header in [
        "x-sts2-caller-id",
        "x-sts2-instance-id",
        "x-sts2-session-id",
        "x-sts2-lease-id",
        "x-sts2-lease-epoch",
        "x-mcp-session-id",
    ] {
        let mut changed = valid.clone();
        changed.headers.insert(header.into(), "foreign".into());
        assert_eq!(admit(&config(), &changed), Err(GatewayError::Rejected));
        changed.headers.remove(header);
        assert_eq!(admit(&config(), &changed), Err(GatewayError::Rejected));
    }
    let mut changed = valid.clone();
    changed.method = GatewayMethod::Post;
    assert_eq!(admit(&config(), &changed), Err(GatewayError::Rejected));
    let mut changed = valid.clone();
    changed.body = Some(JsonValue::Null);
    assert_eq!(admit(&config(), &changed), Err(GatewayError::Rejected));
    let mut changed = valid;
    changed.path.push_str("?path=/secret");
    assert_eq!(admit(&config(), &changed), Err(GatewayError::Rejected));
}

#[test]
fn checkpoint_reference_executable_binds_configured_caller_on_real_exchange() {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;
    use std::time::Duration;
    for caller in ["caller", "foreign"] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let mut config = config();
        config.gateway_address = listener.local_addr().unwrap();
        let worker = thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut bytes = [0; 8192];
            let count = socket.read(&mut bytes).unwrap();
            let request = String::from_utf8_lossy(&bytes[..count]);
            assert!(request.starts_with("GET /v1/instances/instance/checkpoint-reference "));
            assert!(request.contains("x-sts2-caller-id: caller"));
            let body = format!(
                r#"{{"schema":"ascension.checkpoint_reference_response.v1","instance_id":"instance","caller_id":"{caller}","session_id":"session","lease_id":"lease","lease_epoch":1,"correlation_id":"request","reference":null}}"#
            );
            socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",body.len()).as_bytes()).unwrap();
        });
        let mut request = request();
        request.path = "/v1/instances/instance/checkpoint-reference".into();
        request
            .headers
            .insert("x-sts2-caller-id".into(), "caller".into());
        let mut adapter = super::super::super::RuntimeGatewayAdapter::new(config, 8192);
        let result = adapter.forward(request);
        if caller == "caller" {
            assert!(result.is_ok());
        } else {
            assert_eq!(result, Err(GatewayError::MalformedResponse));
        }
        worker.join().unwrap();
    }
}
