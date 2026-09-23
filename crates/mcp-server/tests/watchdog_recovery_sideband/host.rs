// SPDX-License-Identifier: MIT

//! Synthetic recovery-mux host listener for the `watchdog-recovery-v1` gate.
//!
//! The loopback producer is deterministic synthetic test code, not a game host
//! and not evidence of native host behavior. It terminates the real gateway's
//! recovery mux (`POST /api/v1/runtime/recovery`) so the real `sts2-mcp-server`
//! sideband can be exercised against a settled recovery record while no game
//! process exists. The frame state machine lives in the `frames` submodule.

use super::frames::{HostState, handle};
use super::{MAX_HTTP_BYTES, MOD_TOKEN, TestResult};
use serde_json::Value;
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, channel};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

/// One observed recovery-mux request, in arrival order.
#[derive(Clone, Debug)]
pub struct HostRequest {
    pub path: String,
    pub kind: String,
}

struct HostControl {
    fail_next_dispatch: AtomicBool,
}

/// Synthetic loopback host terminating the gateway's recovery mux.
pub struct RecoveryHost {
    pub address: SocketAddr,
    records: Receiver<HostRequest>,
    control: Arc<HostControl>,
    shutdown: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl RecoveryHost {
    pub fn start() -> TestResult<Self> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        listener.set_nonblocking(true)?;
        let address = listener.local_addr()?;
        let (sender, records) = channel();
        let control = Arc::new(HostControl {
            fail_next_dispatch: AtomicBool::new(false),
        });
        let shutdown = Arc::new(AtomicBool::new(false));
        let worker_control = Arc::clone(&control);
        let worker_shutdown = Arc::clone(&shutdown);
        let worker = thread::spawn(move || {
            let mut state = HostState::default();
            while !worker_shutdown.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        let _ = serve(stream, &mut state, &worker_control, &sender);
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(2));
                    }
                    Err(error) => {
                        let _ = sender.send(HostRequest {
                            path: String::from("<accept error>"),
                            kind: error.to_string(),
                        });
                        break;
                    }
                }
            }
        });
        Ok(Self {
            address,
            records,
            control,
            shutdown,
            worker: Some(worker),
        })
    }

    /// Makes the very next dispatch close without a response, so the gateway
    /// observes a lost host answer rather than a fabricated outcome.
    pub fn fail_next_dispatch(&self) {
        self.control
            .fail_next_dispatch
            .store(true, Ordering::SeqCst);
    }

    /// Collects every request recorded so far without blocking.
    pub fn observed(&self) -> Vec<HostRequest> {
        let mut requests = Vec::new();
        while let Ok(request) = self.records.try_recv() {
            requests.push(request);
        }
        requests
    }

    /// Waits out a short quiet window and returns any request that arrived.
    pub fn quiet(&mut self, window: Duration) -> Vec<HostRequest> {
        let mut requests = Vec::new();
        let deadline = Instant::now() + window;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                break;
            }
            match self
                .records
                .recv_timeout(remaining.min(Duration::from_millis(25)))
            {
                Ok(request) => requests.push(request),
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
        requests
    }
}

impl Drop for RecoveryHost {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::SeqCst);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn serve(
    mut stream: TcpStream,
    state: &mut HostState,
    control: &HostControl,
    sender: &std::sync::mpsc::Sender<HostRequest>,
) -> std::io::Result<()> {
    let Some((path, headers, body)) = read_http_request(&mut stream)? else {
        return Ok(());
    };
    let request: Value = match serde_json::from_slice(&body) {
        Ok(value) => value,
        Err(_) => {
            let _ = sender.send(HostRequest {
                path,
                kind: String::from("<unparseable>"),
            });
            return Ok(());
        }
    };
    let kind = request["kind"].as_str().unwrap_or_default().to_owned();
    let _ = sender.send(HostRequest {
        path: path.clone(),
        kind: kind.clone(),
    });
    if headers.get("authorization").map(String::as_str) != Some(&format!("Bearer {MOD_TOKEN}")) {
        return write_http_response(&mut stream, 401, b"{}");
    }
    if kind == "operation_dispatch_request"
        && control.fail_next_dispatch.swap(false, Ordering::SeqCst)
    {
        // Drop the connection without a response: the gateway must surface the
        // lost answer instead of inventing a settled record.
        return Ok(());
    }
    let (status, response) = handle(state, &request);
    write_http_response(&mut stream, status, &serde_json::to_vec(&response)?)
}

type HttpRequestParts = (String, BTreeMap<String, String>, Vec<u8>);

fn read_http_request(stream: &mut TcpStream) -> std::io::Result<Option<HttpRequestParts>> {
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    let mut head = Vec::new();
    while !head.ends_with(b"\r\n\r\n") {
        if head.len() >= MAX_HTTP_BYTES {
            return Err(std::io::Error::other("synthetic headers exceed bound"));
        }
        let mut byte = [0_u8; 1];
        match stream.read(&mut byte) {
            Ok(0) => return Ok(None),
            Ok(_) => head.push(byte[0]),
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => return Ok(None),
            Err(error) => return Err(error),
        }
    }
    let text = std::str::from_utf8(&head)
        .map_err(std::io::Error::other)?
        .to_owned();
    let path = text
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .ok_or_else(|| std::io::Error::other("synthetic path absent"))?
        .to_owned();
    let mut headers = BTreeMap::new();
    for line in text.lines().skip(1).filter(|line| !line.is_empty()) {
        if let Some((name, value)) = line.split_once(':') {
            headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_owned());
        }
    }
    let length = headers
        .get("content-length")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);
    if head.len().saturating_add(length) > MAX_HTTP_BYTES {
        return Err(std::io::Error::other("synthetic request exceeds bound"));
    }
    let mut body = vec![0_u8; length];
    stream.read_exact(&mut body)?;
    Ok(Some((path, headers, body)))
}

fn write_http_response(stream: &mut TcpStream, status: u16, body: &[u8]) -> std::io::Result<()> {
    write!(
        stream,
        "HTTP/1.1 {status} OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    )?;
    stream.write_all(body)
}
