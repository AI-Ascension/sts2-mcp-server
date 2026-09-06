// SPDX-License-Identifier: MIT

use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{Receiver, channel};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

pub const CONTROL_TOKEN: &str = "disposable-coop-coordinator-token";
pub const READ_TOKEN: &str = "disposable-coop-reader-token";
pub type TestResult<T> = Result<T, Box<dyn std::error::Error>>;

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
    downstream: SocketAddr,
) -> TestResult<OwnedChild> {
    let expiry = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs()
        + 120;
    let child = Command::new(binary)
        .env_clear()
        .env("STS2_GATEWAY_ADDR", address.to_string())
        .env("STS2_MOD_ADDR", downstream.to_string())
        .env("STS2_MOD_TOKEN", "disposable-unused-mod-token")
        .env("STS2_GATEWAY_TOKEN", CONTROL_TOKEN)
        .env("STS2_GATEWAY_TOKEN_SCOPE", "read,control")
        .env("STS2_GATEWAY_TOKEN_PREVIOUS", READ_TOKEN)
        .env("STS2_GATEWAY_TOKEN_PREVIOUS_SCOPE", "read")
        .env("STS2_GATEWAY_TOKEN_PREVIOUS_EXPIRES_AT", expiry.to_string())
        .env(
            "STS2_COOP_ROSTER",
            r#"[{"peer_id":"local-1","role":"local"},{"peer_id":"ally-1","role":"ally"}]"#,
        )
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    Ok(OwnedChild(child))
}

pub fn await_gateway(child: &mut OwnedChild, address: SocketAddr) -> TestResult<()> {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if let Some(status) = child.0.try_wait()? {
            return Err(format!("gateway exited: {status}").into());
        }
        if let Ok((409, _)) = http(
            address,
            "GET",
            "/v1/instances/instance-1/coop/synchronization",
            READ_TOKEN,
            None,
            None,
        ) {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(20));
    }
    Err("gateway did not expose its lease-fenced route".into())
}

pub fn http(
    address: SocketAddr,
    method: &str,
    path: &str,
    token: &str,
    body: Option<Value>,
    changed: Option<(&str, &str)>,
) -> TestResult<(u16, Value)> {
    let body = body.map(|value| value.to_string()).unwrap_or_default();
    let mut headers = BTreeMap::from([
        ("authorization", format!("Bearer {token}")),
        ("x-sts2-instance-id", "instance-1".to_owned()),
        ("x-sts2-caller-id", "harness".to_owned()),
        ("x-sts2-session-id", "session-1".to_owned()),
        ("x-mcp-session-id", "mcp-session-1".to_owned()),
        ("x-sts2-lease-id", "lease-1".to_owned()),
        ("x-sts2-lease-epoch", "1".to_owned()),
        ("x-sts2-correlation-id", "report-1".to_owned()),
        ("content-type", "application/json".to_owned()),
        ("host", address.to_string()),
        ("content-length", body.len().to_string()),
        ("connection", "close".to_owned()),
    ]);
    if let Some((name, value)) = changed {
        headers.insert(name, value.to_owned());
    }
    let mut request = format!("{method} {path} HTTP/1.1\r\n");
    for (name, value) in headers {
        request.push_str(&format!("{name}: {value}\r\n"));
    }
    request.push_str("\r\n");
    request.push_str(&body);
    let mut socket = TcpStream::connect_timeout(&address, Duration::from_secs(1))?;
    socket.set_read_timeout(Some(Duration::from_secs(2)))?;
    socket.set_write_timeout(Some(Duration::from_secs(2)))?;
    socket.write_all(request.as_bytes())?;
    let mut bytes = Vec::new();
    socket.take(32769).read_to_end(&mut bytes)?;
    if bytes.len() > 32768 {
        return Err("HTTP response exceeds test bound".into());
    }
    let text = std::str::from_utf8(&bytes)?;
    let (head, body) = text.split_once("\r\n\r\n").ok_or("HTTP framing absent")?;
    let status = head
        .split_whitespace()
        .nth(1)
        .ok_or("HTTP status absent")?
        .parse()?;
    Ok((status, serde_json::from_str(body)?))
}

pub fn report(
    address: SocketAddr,
    peer: &str,
    generation: u64,
    connected: bool,
) -> TestResult<(u16, Value)> {
    http(
        address,
        "POST",
        "/v1/instances/instance-1/coop/peer-report",
        CONTROL_TOKEN,
        Some(json!({"peer_id":peer,"generation":generation,"connected":connected})),
        None,
    )
}

pub struct Mcp {
    child: OwnedChild,
    input: ChildStdin,
    lines: Receiver<Result<String, String>>,
    reader: Option<JoinHandle<()>>,
    id: u64,
}

impl Mcp {
    pub fn start(address: SocketAddr) -> TestResult<Self> {
        let mut child = OwnedChild(
            Command::new(env!("CARGO_BIN_EXE_sts2-mcp-server"))
                .env_clear()
                .env("STS2_GATEWAY_ADDR", address.to_string())
                .env("STS2_GATEWAY_TOKEN", READ_TOKEN)
                .env("STS2_RUNTIME_PROFILE", "coop-synchronization-v1")
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

    pub fn request(&mut self, method: &str, params: Value) -> TestResult<Value> {
        self.id += 1;
        writeln!(
            self.input,
            "{}",
            json!({"jsonrpc":"2.0","id":self.id,"method":method,"params":params})
        )?;
        self.input.flush()?;
        let line = self.lines.recv_timeout(Duration::from_secs(5))??;
        let response: Value = serde_json::from_str(&line)?;
        if response["id"] != self.id {
            return Err("MCP response identity mismatched".into());
        }
        Ok(response)
    }

    pub fn synchronization(&mut self) -> TestResult<Value> {
        let response = self.request("tools/call", json!({"name":"sts2.coop_synchronization","arguments":{
            "instance_id":"instance-1","mcp_session_id":"mcp-session-1","lease_id":"lease-1","lease_epoch":1,
        }}))?;
        if response["result"]["isError"] != false {
            return Err(format!("co-op read failed: {response}").into());
        }
        let body = response["result"]["content"][0]["text"]
            .as_str()
            .ok_or("MCP result text absent")?;
        Ok(serde_json::from_str(body)?)
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
