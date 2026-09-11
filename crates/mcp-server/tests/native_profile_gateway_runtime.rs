// SPDX-License-Identifier: MIT

//! Executable boundary gate for the native profile.  The loopback producer is
//! deterministic synthetic test code, not a game host or evidence of host behavior.

use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{Receiver, channel};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

#[path = "native_profile_gateway_runtime/support.rs"]
mod support;

use support::SyntheticProducer;

const GATEWAY_TOKEN: &str = "native-profile-gateway-token";
const PEER_TOKEN: &str = "native-profile-private-peer-token";
const PEER_ID: &str = "peer:host1";
const INSTANCE_ID: &str = "instance-1";
const SESSION_ID: &str = "session-1";
const LEASE_ID: &str = "lease-1";
const MCP_SESSION_ID: &str = "mcp-session-native-test";
const LEASE_EPOCH: u64 = 7;
const MAX_HTTP_BYTES: usize = 32 * 1024;
type TestResult<T> = Result<T, Box<dyn std::error::Error>>;

struct OwnedChild(Child);

impl Drop for OwnedChild {
    fn drop(&mut self) {
        if self.0.try_wait().ok().flatten().is_none() {
            let _ = self.0.kill();
        }
        let _ = self.0.wait();
    }
}

struct Mcp {
    child: OwnedChild,
    input: ChildStdin,
    lines: Receiver<Result<String, String>>,
    reader: Option<JoinHandle<()>>,
    id: u64,
}

impl Mcp {
    fn start(address: SocketAddr) -> TestResult<Self> {
        let mut child = OwnedChild(
            Command::new(env!("CARGO_BIN_EXE_sts2-mcp-server"))
                .env_clear()
                .env("STS2_GATEWAY_ADDR", address.to_string())
                .env("STS2_GATEWAY_TOKEN", GATEWAY_TOKEN)
                .env("STS2_RUNTIME_PROFILE", "coop-native-v1")
                .env("STS2_COOP_NATIVE_PEER_TOKEN", PEER_TOKEN)
                .env("STS2_COOP_NATIVE_PEER_ID", PEER_ID)
                .env("STS2_INSTANCE_ID", INSTANCE_ID)
                .env("STS2_SESSION_ID", SESSION_ID)
                .env("STS2_MCP_SESSION_ID", MCP_SESSION_ID)
                .env("STS2_LEASE_ID", LEASE_ID)
                .env("STS2_LEASE_EPOCH", LEASE_EPOCH.to_string())
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()?,
        );
        let input = child.0.stdin.take().ok_or("MCP stdin absent")?;
        let output = child.0.stdout.take().ok_or("MCP stdout absent")?;
        let (sender, lines) = channel();
        let reader = thread::spawn(move || {
            let mut reader = BufReader::new(output);
            loop {
                let mut line = String::new();
                match reader.by_ref().take(65537).read_line(&mut line) {
                    Ok(0) => break,
                    Ok(_) if line.len() <= 65536 => {
                        if sender.send(Ok(line)).is_err() {
                            break;
                        }
                    }
                    _ => {
                        let _ = sender.send(Err("invalid or oversized MCP output".to_owned()));
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

    fn request(&mut self, name: &str, arguments: Value) -> TestResult<Value> {
        self.id += 1;
        writeln!(
            self.input,
            "{}",
            json!({"jsonrpc":"2.0","id":self.id,"method":"tools/call","params":{"name":name,"arguments":arguments}})
        )?;
        self.input.flush()?;
        let line = self.lines.recv_timeout(Duration::from_secs(5))??;
        let response: Value = serde_json::from_str(&line)?;
        assert_eq!(response["id"], self.id);
        Ok(response)
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

fn arguments() -> Value {
    json!({
        "instance_id": INSTANCE_ID,
        "mcp_session_id": MCP_SESSION_ID,
        "lease_id": LEASE_ID,
        "lease_epoch": LEASE_EPOCH,
    })
}

fn spawn_gateway(
    binary: &str,
    address: SocketAddr,
    producer: SocketAddr,
) -> TestResult<OwnedChild> {
    Ok(OwnedChild(
        Command::new(binary)
            .env_clear()
            .env("STS2_GATEWAY_ADDR", address.to_string())
            .env("STS2_MOD_ADDR", producer.to_string())
            .env("STS2_MOD_TOKEN", "synthetic-mod-token")
            .env("STS2_GATEWAY_TOKEN", GATEWAY_TOKEN)
            .env("STS2_GATEWAY_TOKEN_SCOPE", "read,control,mutate")
            .env("STS2_COOP_NATIVE_PEER_TOKEN", PEER_TOKEN)
            .env("STS2_COOP_NATIVE_PEER_ID", PEER_ID)
            .env("STS2_INSTANCE_ID", INSTANCE_ID)
            .env("STS2_SESSION_ID", SESSION_ID)
            .env("STS2_MCP_SESSION_ID", MCP_SESSION_ID)
            .env("STS2_LEASE_ID", LEASE_ID)
            .env("STS2_LEASE_EPOCH", LEASE_EPOCH.to_string())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?,
    ))
}

fn await_gateway(gateway: &mut OwnedChild, address: SocketAddr) -> TestResult<()> {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if let Some(status) = gateway.0.try_wait()? {
            return Err(format!("gateway exited: {status}").into());
        }
        if TcpStream::connect_timeout(&address, Duration::from_millis(50)).is_ok() {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(20));
    }
    Err("gateway did not bind loopback listener".into())
}

fn direct_gateway(
    address: SocketAddr,
    path: &str,
    peer_token: Option<&str>,
    lease_epoch: u64,
    body: Value,
) -> TestResult<u16> {
    let body = body.to_string();
    let mut headers = BTreeMap::from([
        ("authorization", format!("Bearer {GATEWAY_TOKEN}")),
        ("x-sts2-instance-id", INSTANCE_ID.to_owned()),
        ("x-sts2-caller-id", "harness".to_owned()),
        ("x-sts2-session-id", SESSION_ID.to_owned()),
        ("x-mcp-session-id", MCP_SESSION_ID.to_owned()),
        ("x-sts2-lease-id", LEASE_ID.to_owned()),
        ("x-sts2-lease-epoch", lease_epoch.to_string()),
        ("x-sts2-correlation-id", "native-direct".to_owned()),
        ("content-type", "application/json".to_owned()),
        ("host", address.to_string()),
        ("content-length", body.len().to_string()),
        ("connection", "close".to_owned()),
    ]);
    if let Some(peer_token) = peer_token {
        headers.insert("x-sts2-peer-token", peer_token.to_owned());
    }
    let mut text = format!("POST {path} HTTP/1.1\r\n");
    for (name, value) in headers {
        text.push_str(&format!("{name}: {value}\r\n"));
    }
    text.push_str("\r\n");
    text.push_str(&body);
    let mut stream = TcpStream::connect_timeout(&address, Duration::from_secs(1))?;
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    stream.write_all(text.as_bytes())?;
    let mut response = Vec::new();
    stream
        .take(MAX_HTTP_BYTES as u64 + 1)
        .read_to_end(&mut response)?;
    if response.len() > MAX_HTTP_BYTES {
        return Err("gateway response exceeds test bound".into());
    }
    let head = std::str::from_utf8(&response)?
        .split_once("\r\n\r\n")
        .ok_or("gateway response framing absent")?
        .0;
    Ok(head
        .split_whitespace()
        .nth(1)
        .ok_or("gateway status absent")?
        .parse()?)
}

#[test]
#[ignore = "requires STS2_COOP_GATEWAY_BINARY exact reviewed gateway; synthetic loopback producer only"]
fn native_profile_executable_gate_uses_private_binding_and_fences_recovery() -> TestResult<()> {
    let gateway_binary = std::env::var("STS2_COOP_GATEWAY_BINARY")?;
    let producer = SyntheticProducer::start()?;
    let reservation = TcpListener::bind("127.0.0.1:0")?;
    let gateway_address = reservation.local_addr()?;
    drop(reservation);
    let mut gateway = spawn_gateway(&gateway_binary, gateway_address, producer.address)?;
    await_gateway(&mut gateway, gateway_address)?;
    assert_eq!(
        direct_gateway(
            gateway_address,
            "/v1/sessions/allocate",
            None,
            LEASE_EPOCH,
            json!({
                "instance_id": INSTANCE_ID,
                "caller_id": "harness",
                "session_id": SESSION_ID,
            })
        )?,
        200
    );

    let mut mcp = Mcp::start(gateway_address)?;
    let mut action = arguments();
    action["operation_id"] = Value::String("op:native:action:unknown".to_owned());
    action["actor_peer"] = Value::String(PEER_ID.to_owned());
    action["expected_host_generation"] = json!(1);
    action["action"] = json!({"kind":"end_turn","action_id":"turn:1","target_peer":null});
    let unknown = mcp.request("sts2.coop_native_action", action)?;
    assert_eq!(unknown["result"]["isError"], true, "{unknown}");
    let unknown_payload: Value = serde_json::from_str(
        unknown["result"]["content"][0]["text"]
            .as_str()
            .ok_or("unknown action response text absent")?,
    )?;
    assert!(unknown_payload["status"] == "unknown", "{unknown_payload}");

    let mut recovery = arguments();
    recovery["operation_id"] = Value::String("op:native:action:unknown".to_owned());
    recovery["recovery"] = json!({"kind":"reconcile","rejoin_epoch":1});
    let settled = mcp.request("sts2.coop_native_recover", recovery)?;
    assert_eq!(settled["result"]["isError"], false, "{settled}");
    let settled_payload: Value = serde_json::from_str(
        settled["result"]["content"][0]["text"]
            .as_str()
            .ok_or("recovery response text absent")?,
    )?;
    assert!(settled_payload["status"] == "settled", "{settled_payload}");

    let mut wrong_actor = arguments();
    wrong_actor["actor_peer"] = Value::String("peer:client1".to_owned());
    wrong_actor["expected_host_generation"] = json!(1);
    let wrong = mcp.request("sts2.coop_native_legal_catalog", wrong_actor)?;
    assert_eq!(wrong["result"]["isError"], true, "{wrong}");

    let mismatch = mcp.request("sts2.coop_native_observation", arguments())?;
    assert_eq!(mismatch["result"]["isError"], true, "{mismatch}");
    assert!(mismatch.to_string().contains("rejected"), "{mismatch}");
    drop(mcp);
    producer.assert_records()?;

    let invalid_action = json!({
        "protocol_version":"coop-native-v1", "schema_digest":"2f3bc99e53080fa11b39592b64fb0ab964a16f568719a2622d0b2caf766ab629",
        "provenance":{"artifact":"sts2-protocol/coop-native-v1","source":"schemas/coop-native-v1.schema.json","generator":"hand-authored"},
        "correlation_id":"native-direct", "instance_id":INSTANCE_ID, "session_id":SESSION_ID, "lease_id":LEASE_ID, "lease_epoch":LEASE_EPOCH,
        "kind":"local_action_request", "operation_id":"op:native:direct", "actor_peer":PEER_ID, "expected_host_generation":1,
        "action":{"kind":"end_turn","action_id":"turn:1","target_peer":null}, "vote":null, "status":null, "observation":null, "effect":null, "recovery":null, "catalog":null, "receipt":null
    });
    assert_eq!(
        direct_gateway(
            gateway_address,
            &format!("/v1/instances/{INSTANCE_ID}/coop/native/action"),
            None,
            LEASE_EPOCH,
            invalid_action.clone()
        )?,
        401
    );
    assert_eq!(
        direct_gateway(
            gateway_address,
            &format!("/v1/instances/{INSTANCE_ID}/coop/native/action"),
            Some("wrong-native-profile-private-peer-token"),
            LEASE_EPOCH,
            invalid_action.clone()
        )?,
        401
    );
    assert_eq!(
        direct_gateway(
            gateway_address,
            &format!("/v1/instances/{INSTANCE_ID}/coop/native/action"),
            Some(PEER_TOKEN),
            LEASE_EPOCH + 1,
            invalid_action
        )?,
        409
    );

    let status = Command::new(env!("CARGO_BIN_EXE_sts2-mcp-server"))
        .env_clear()
        .env("STS2_GATEWAY_ADDR", gateway_address.to_string())
        .env("STS2_GATEWAY_TOKEN", GATEWAY_TOKEN)
        .env("STS2_RUNTIME_PROFILE", "coop-native-v1")
        .status()?;
    assert!(
        !status.success(),
        "native profile must reject missing binding configuration"
    );
    Ok(())
}
