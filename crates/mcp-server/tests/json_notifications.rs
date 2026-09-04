// SPDX-License-Identifier: MIT

use std::io::Write;
use std::process::{Command, Stdio};
use sts2_mcp_server::{
    GatewayAdapter, GatewayError, GatewayRequest, GatewayResponse, JsonValue, McpServer, parse_json,
};

#[derive(Default)]
struct NoGateway {
    calls: usize,
}
impl GatewayAdapter for NoGateway {
    fn forward(&mut self, _: GatewayRequest) -> Result<GatewayResponse, GatewayError> {
        self.calls += 1;
        Err(GatewayError::Unavailable)
    }
}

#[test]
fn unicode_strings_round_trip_and_surrogate_pairs_decode() {
    let value = JsonValue::string("café 日本 🎮");
    assert_eq!(parse_json(&value.to_json()), Ok(value));
    assert_eq!(parse_json(r#""\ud83c\udfae""#), Ok(JsonValue::string("🎮")));
    for invalid in [r#""\ud800""#, r#""\udc00""#, r#""\ud800\u0041""#] {
        assert!(parse_json(invalid).is_err());
    }
}

#[test]
fn ambiguous_numbers_duplicate_fields_and_deep_frames_fail_closed() {
    for invalid in ["01", "-01", r#"{"id":1,"id":2}"#, r#"{"id":1,"\u0069d":2}"#] {
        assert!(parse_json(invalid).is_err(), "{invalid}");
    }
    assert!(parse_json("0").is_ok());
    assert!(parse_json("-0").is_ok());
    assert!(parse_json(&format!("{}0{}", "[".repeat(63), "]".repeat(63))).is_ok());
    assert!(parse_json(&format!("{}0{}", "[".repeat(8000), "]".repeat(8000))).is_err());
}

#[test]
fn notifications_are_silent_and_malformed_messages_still_error() {
    let mut server = McpServer::new(NoGateway::default());
    for notification in [
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
        r#"{"jsonrpc":"2.0","method":"notifications/cancelled","params":{"requestId":1}}"#,
        r#"{"jsonrpc":"2.0","method":"tools/call","params":{"name":"submit_action","arguments":{}}}"#,
    ] {
        assert_eq!(server.handle_message(notification), None);
    }
    assert_eq!(server.gateway().calls, 0);
    assert!(
        server
            .handle_frame(r#"{"jsonrpc":"2.0","method":3}"#)
            .contains("-32600")
    );
    assert!(
        server
            .handle_frame(r#"{"jsonrpc":"2.0","method":"tools/list","id":null}"#)
            .contains("-32600")
    );
    assert!(
        server
            .handle_frame(r#"{"jsonrpc":"2.0","method":"tools/list","id":1}"#)
            .contains("result")
    );
}

#[test]
fn executable_writes_no_notification_response_or_blank_line()
-> Result<(), Box<dyn std::error::Error>> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_sts2-mcp-server"))
        .env_clear()
        .env("STS2_GATEWAY_TOKEN", "test-only")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;
    child.stdin.take().ok_or("missing stdin")?.write_all(
        b"{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n{\"jsonrpc\":\"2.0\",\"method\":\"tools/list\",\"id\":1}\n"
    )?;
    let output = child.wait_with_output()?;
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout)?;
    assert_eq!(text.lines().count(), 1);
    assert!(text.contains("result"));
    Ok(())
}
