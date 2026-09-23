// SPDX-License-Identifier: MIT

//! Support for the `watchdog-recovery-v1` executable gate.
//!
//! The synthetic loopback producer lives in the `host` submodule and the
//! dependency-free wire codecs in `codec`. Together they let the real
//! `sts2-mcp-server` sideband be exercised against a settled recovery record
//! while no game process exists at all, so a read that replayed an effect would
//! be visible as a second dispatch control frame.

mod codec;
mod frames;
mod host;

pub use codec::uuid_v4;
pub use host::{HostRequest, RecoveryHost};

use codec::{base64, hex, sha256, utc_timestamp_now};
use serde_json::{Map, Value, json};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{Receiver, channel};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

pub type TestResult<T> = Result<T, Box<dyn std::error::Error>>;

pub const CONTRACT: &str = "watchdog-recovery-v1";
pub const SCHEMA: &str = "fb934d3157485aaf6e13e6ebbb213ec8a14c7fc6f5eeebc06b7a22c1f0009217";
pub const LEASE_CONTRACT: &str = "watchdog-host-lease-control-v1";
pub const LEASE_SCHEMA: &str = "e22faf0f7d3cd313a007b65e52058b3c255153d5778dd8124055c283adf977f9";
pub const V3_SCHEMA: &str = "8e99cea36b7ede97532348fd8efe302ca79260895265a7bf14ddf7e006d8ff63";
pub const ZERO_DIGEST: &str = "0000000000000000000000000000000000000000000000000000000000000000";

pub const DEPLOYMENT: &str = "00000000-0000-4000-8000-000000000001";
pub const INSTANCE: &str = "00000000-0000-4000-8000-000000000002";
pub const INCARNATION: &str = "00000000-0000-4000-8000-000000000003";
pub const CALLER: &str = "00000000-0000-4000-8000-00000000000a";
pub const SESSION: &str = "00000000-0000-4000-8000-00000000000b";
pub const MCP_SESSION: &str = "00000000-0000-4000-8000-00000000000c";
pub const LEASE: &str = "00000000-0000-4000-8000-00000000000d";
pub const LEASE_EPOCH: u64 = 1;
pub const LEASE_KEY_HEX: &str = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";
pub const BOOTSTRAP_SECRET_B64: &str = "EBESExQVFhcYGRobHB0eHyAhIiMkJSYnKCkqKywtLi8=";
pub const RECOVERY_READ_SECRET: &str = "recovery-read-secret-value";
pub const GATEWAY_TOKEN: &str = "recovery-gateway-token";
pub const RECOVERY_TOKEN: &str = "recovery-sideband-token";
pub const MOD_TOKEN: &str = "synthetic-mod-token";
pub const RECOVERY_CONTROL_PATH: &str = "/api/v1/runtime/recovery";
pub const MAX_HTTP_BYTES: usize = 512 * 1024;

const ACTION_BYTES: &[u8] = br#"{"action":{"kind":"end_turn"},"action_id":"combat.end-turn"}"#;

pub fn recovery_request(kind: &str, capability: &str, payload: Value) -> Value {
    json!({
        "contract": CONTRACT,
        "schema_digest": SCHEMA,
        "message_id": uuid_v4(),
        "correlation_id": uuid_v4(),
        "sent_at": utc_timestamp_now(),
        "actor": {"principal_id": CALLER, "role": "harness"},
        "auth": {"principal_id": CALLER, "capability": capability, "proof": Value::Null},
        "kind": kind,
        "payload": payload,
    })
}

/// Sends one HTTP request and returns the status plus the parsed JSON body.
pub fn http_request(
    address: SocketAddr,
    method: &str,
    path: &str,
    headers: &[(&str, &str)],
    body: Option<&[u8]>,
) -> TestResult<(u16, Value)> {
    let body = body.unwrap_or_default();
    let mut request = format!("{method} {path} HTTP/1.1\r\nHost: {address}\r\n");
    for (name, value) in headers {
        request.push_str(&format!("{name}: {value}\r\n"));
    }
    request.push_str(&format!(
        "Content-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    ));
    let mut stream = TcpStream::connect_timeout(&address, Duration::from_secs(2))?;
    stream.set_read_timeout(Some(Duration::from_secs(10)))?;
    stream.write_all(request.as_bytes())?;
    stream.write_all(body)?;
    let mut response = Vec::new();
    stream
        .take(MAX_HTTP_BYTES as u64 + 1)
        .read_to_end(&mut response)?;
    if response.len() > MAX_HTTP_BYTES {
        return Err("gateway response exceeds test bound".into());
    }
    let text = String::from_utf8_lossy(&response);
    let (head, payload) = text
        .split_once("\r\n\r\n")
        .ok_or("gateway framing absent")?;
    let status: u16 = head
        .split_whitespace()
        .nth(1)
        .ok_or("gateway status absent")?
        .parse()?;
    let body = serde_json::from_str(payload).unwrap_or(Value::Null);
    Ok((status, body))
}

/// Sends one recovery frame with the capability header the route demands.
pub fn recovery_post(
    address: SocketAddr,
    path: &str,
    capability: &str,
    frame: &Value,
) -> TestResult<(u16, Value)> {
    let body = serde_json::to_vec(frame)?;
    http_request(
        address,
        "POST",
        path,
        &[
            ("Authorization", &format!("Bearer {RECOVERY_TOKEN}")),
            ("Content-Type", "application/json"),
            ("x-sts2-recovery-capability", capability),
        ],
        Some(&body),
    )
}
pub struct OwnedChild(pub Child);

impl Drop for OwnedChild {
    fn drop(&mut self) {
        if self.0.try_wait().ok().flatten().is_none() {
            let _ = self.0.kill();
        }
        let _ = self.0.wait();
    }
}

pub fn spawn_gateway(
    binary: &str,
    address: SocketAddr,
    producer: SocketAddr,
    store_path: &str,
) -> TestResult<OwnedChild> {
    let mut command = Command::new(binary);
    command.env_clear();
    for (name, value) in gateway_environment(address, producer, store_path) {
        command.env(name, value);
    }
    Ok(OwnedChild(
        command
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?,
    ))
}

fn gateway_environment(
    address: SocketAddr,
    producer: SocketAddr,
    store_path: &str,
) -> Vec<(String, String)> {
    let home = std::env::var("HOME").unwrap_or_else(|_| String::from("/tmp"));
    let path = std::env::var("PATH").unwrap_or_else(|_| String::from("/usr/bin:/bin"));
    [
        ("HOME", home),
        ("PATH", path),
        ("STS2_GATEWAY_ADDR", address.to_string()),
        ("STS2_MOD_ADDR", producer.to_string()),
        ("STS2_MOD_TOKEN", MOD_TOKEN.to_owned()),
        ("STS2_GATEWAY_TOKEN", GATEWAY_TOKEN.to_owned()),
        ("STS2_RECOVERY_TOKEN", RECOVERY_TOKEN.to_owned()),
        ("STS2_INSTANCE_ID", INSTANCE.to_owned()),
        ("STS2_CALLER_ID", CALLER.to_owned()),
        ("STS2_SESSION_ID", SESSION.to_owned()),
        ("STS2_MCP_SESSION_ID", MCP_SESSION.to_owned()),
        ("STS2_LEASE_ID", LEASE.to_owned()),
        ("STS2_LEASE_EPOCH", LEASE_EPOCH.to_string()),
        ("STS2_DEPLOYMENT_ID", DEPLOYMENT.to_owned()),
        ("STS2_RECOVERY_STORE", store_path.to_owned()),
        ("STS2_RUNTIME_HOST_PRINCIPAL_ID", CALLER.to_owned()),
        ("STS2_RUNTIME_HOST_LEASE_KEY", LEASE_KEY_HEX.to_owned()),
        (
            "STS2_RUNTIME_BOOTSTRAP_SECRET",
            BOOTSTRAP_SECRET_B64.to_owned(),
        ),
        (
            "STS2_RUNTIME_RECOVERY_READ_SECRET",
            RECOVERY_READ_SECRET.to_owned(),
        ),
        ("STS2_RECOVERY_RELEASE_DIGEST", ZERO_DIGEST.to_owned()),
        ("STS2_RECOVERY_CONFIG_DIGEST", ZERO_DIGEST.to_owned()),
        ("STS2_RECOVERY_PROFILE_DIGEST", ZERO_DIGEST.to_owned()),
        (
            "STS2_RECOVERY_RUNTIME_V3_SCHEMA_DIGEST",
            V3_SCHEMA.to_owned(),
        ),
    ]
    .into_iter()
    .map(|(name, value)| (String::from(name), value))
    .collect()
}

pub fn await_gateway(gateway: &mut OwnedChild, address: SocketAddr) -> TestResult<()> {
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        if let Some(status) = gateway.0.try_wait()? {
            return Err(format!("gateway exited early: {status}").into());
        }
        if let Ok((status, _)) = http_request(
            address,
            "GET",
            "/health/live",
            &[("Authorization", &format!("Bearer {GATEWAY_TOKEN}"))],
            None,
        ) && status == 200
        {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(25));
    }
    Err("gateway did not become live".into())
}

pub struct Mcp {
    child: OwnedChild,
    input: std::process::ChildStdin,
    lines: Receiver<Result<String, String>>,
    reader: Option<JoinHandle<()>>,
    id: u64,
}

impl Mcp {
    pub fn start(address: SocketAddr) -> TestResult<Self> {
        let mut command = Command::new(env!("CARGO_BIN_EXE_sts2-mcp-server"));
        command.env_clear();
        let home = std::env::var("HOME").unwrap_or_else(|_| String::from("/tmp"));
        for (name, value) in [
            ("HOME", home),
            ("STS2_GATEWAY_ADDR", address.to_string()),
            ("STS2_RECOVERY_TOKEN", RECOVERY_TOKEN.to_owned()),
            ("STS2_RUNTIME_PROFILE", String::from("watchdog-recovery-v1")),
            ("STS2_CALLER_ID", CALLER.to_owned()),
            ("STS2_INSTANCE_ID", INSTANCE.to_owned()),
            ("STS2_SESSION_ID", SESSION.to_owned()),
            ("STS2_MCP_SESSION_ID", MCP_SESSION.to_owned()),
            ("STS2_LEASE_ID", LEASE.to_owned()),
            ("STS2_LEASE_EPOCH", LEASE_EPOCH.to_string()),
        ] {
            command.env(name, value);
        }
        let mut child = OwnedChild(
            command
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()?,
        );
        let input = child.0.stdin.take().ok_or("MCP stdin absent")?;
        let output = child.0.stdout.take().ok_or("MCP stdout absent")?;
        let (sender, lines) = channel();
        let reader = thread::spawn(move || {
            let mut reader = std::io::BufReader::new(output);
            loop {
                let mut line = String::new();
                match std::io::BufRead::read_line(&mut reader, &mut line) {
                    Ok(0) => break,
                    Ok(_) if line.len() <= 1 << 20 => {
                        if sender.send(Ok(line)).is_err() {
                            break;
                        }
                    }
                    _ => {
                        let _ = sender.send(Err(String::from("invalid or oversized MCP output")));
                        break;
                    }
                }
            }
        });
        Ok(Self {
            child,
            input,
            lines,
            reader: Some(reader),
            id: 0,
        })
    }

    pub fn request(&mut self, name: &str, arguments: Value) -> TestResult<Value> {
        self.id += 1;
        writeln!(
            self.input,
            "{}",
            json!({"jsonrpc":"2.0","id":self.id,"method":name,"params":arguments})
        )?;
        self.input.flush()?;
        let line = self.lines.recv_timeout(Duration::from_secs(10))??;
        let response: Value = serde_json::from_str(&line)?;
        assert_eq!(response["id"], self.id);
        Ok(response)
    }

    pub fn tools_call(&mut self, name: &str, arguments: Value) -> TestResult<Value> {
        self.request("tools/call", json!({"name": name, "arguments": arguments}))
    }
}

impl Drop for Mcp {
    fn drop(&mut self) {
        let _ = self.child.0.kill();
        let _ = self.child.0.wait();
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
    }
}

/// The closed recovery tool arguments for this sideband.
pub fn recovery_arguments(payload: Value) -> Value {
    json!({"mcp_session_id": MCP_SESSION, "payload": payload})
}

/// The seven-field operation reference the sideband forwards verbatim.
pub fn operation_ref(operation_id: &str, payload_digest: &str, context: &Value) -> Value {
    json!({
        "operation_id": operation_id,
        "payload_digest": payload_digest,
        "original_context": context,
    })
}

/// The seven-field original context a lease provides.
pub fn original_context(lease: &Value) -> Value {
    json!({
        "deployment_id": lease["deployment_id"],
        "instance_id": lease["instance_id"],
        "instance_incarnation": lease["instance_incarnation"],
        "boot_id": lease["boot_id"],
        "authority_generation": lease["authority_generation"],
        "lease_id": lease["lease_id"],
        "lease_epoch": lease["lease_epoch"],
    })
}

/// The digest of the fixed synthetic action this gate dispatches.
pub fn action_digest() -> String {
    hex(&sha256(ACTION_BYTES))
}

/// The canonical operation payload the gateway records at intent time.
pub fn operation_intent(operation_id: &str, context: &Value) -> Value {
    let digest = action_digest();
    json!({
        "operation_id": operation_id,
        "payload_digest": digest,
        "original_context": context,
        "expected_boundary": {
            "state_id": "00000000-0000-4000-8000-0000000000aa",
            "generation": 0,
            "catalog_digest": "c".repeat(64),
        },
        "action": {
            "schema_digest": V3_SCHEMA,
            "canonical_json_b64": base64(ACTION_BYTES),
            "payload_digest": digest,
        },
    })
}

/// The map of a recovery response frame's payload.
pub fn response_payload(frame: &Value) -> Map<String, Value> {
    frame["payload"].as_object().cloned().unwrap_or_default()
}
