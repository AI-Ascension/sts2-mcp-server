// SPDX-License-Identifier: MIT

use super::{
    INSTANCE_ID, LEASE_EPOCH, LEASE_ID, MAX_HTTP_BYTES, PEER_TOKEN, SESSION_ID, TestResult,
};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::mpsc::{Receiver, channel};
use std::thread::{self, JoinHandle};
use std::time::Duration;

pub(super) struct SyntheticProducer {
    pub(super) address: SocketAddr,
    records: Receiver<Result<ProducerRecord, String>>,
    worker: Option<JoinHandle<()>>,
}

#[derive(Debug)]
struct ProducerRecord {
    path: String,
    headers: BTreeMap<String, String>,
    body: Value,
}

impl SyntheticProducer {
    pub(super) fn start() -> TestResult<Self> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let address = listener.local_addr()?;
        let (sender, records) = channel();
        let worker = thread::spawn(move || {
            for _ in 0..3 {
                let (mut stream, _) = match listener.accept() {
                    Ok(connection) => connection,
                    Err(error) => {
                        let _ = sender.send(Err(error.to_string()));
                        return;
                    }
                };
                let record = match read_http_request(&mut stream) {
                    Ok(record) => record,
                    Err(error) => {
                        let _ = sender.send(Err(error.to_string()));
                        return;
                    }
                };
                let response = match synthetic_response(&record) {
                    Ok(response) => response,
                    Err(error) => {
                        let _ = sender.send(Err(error));
                        return;
                    }
                };
                if write_http_response(&mut stream, 200, &response).is_err() {
                    let _ = sender.send(Err("synthetic producer response write failed".to_owned()));
                    return;
                }
                if sender.send(Ok(record)).is_err() {
                    return;
                }
            }
        });
        Ok(Self {
            address,
            records,
            worker: Some(worker),
        })
    }

    pub(super) fn assert_records(mut self) -> TestResult<()> {
        let mut records = Vec::new();
        for _ in 0..3 {
            records.push(self.records.recv_timeout(Duration::from_secs(5))??);
        }
        if let Some(worker) = self.worker.take() {
            worker.join().map_err(|_| "synthetic producer panicked")?;
        }
        assert_eq!(
            records
                .iter()
                .map(|record| record.path.as_str())
                .collect::<Vec<_>>(),
            [
                "/api/v1/coop/native/action",
                "/api/v1/coop/native/recover",
                "/api/v1/coop/native/observation",
            ]
        );
        for record in records {
            assert_eq!(
                record.headers.get("authorization").map(String::as_str),
                Some("Bearer synthetic-mod-token")
            );
            assert!(
                !record.headers.contains_key("x-sts2-peer-token"),
                "private peer credential reached synthetic producer"
            );
            assert_ne!(record.body.to_string(), PEER_TOKEN);
            for header in [
                "x-sts2-protocol-version",
                "x-sts2-schema-digest",
                "x-sts2-host-generation",
            ] {
                assert!(
                    !record.headers.contains_key(header),
                    "redundant native metadata header reached gateway producer: {header}"
                );
            }
            if record.path != "/api/v1/coop/native/observation" {
                assert!(record.body.get("protocol_version").is_some());
                assert!(record.body.get("schema_digest").is_some());
            }
        }
        Ok(())
    }
}

fn synthetic_response(record: &ProducerRecord) -> Result<Vec<u8>, String> {
    let fixture = match record.path.as_str() {
        "/api/v1/coop/native/action" => include_str!(
            "../../../../protocol-artifact/coop-native-v1/golden/local-action-unknown-response.json"
        ),
        "/api/v1/coop/native/recover" => include_str!(
            "../../../../protocol-artifact/coop-native-v1/golden/local-action-recovered-response.json"
        ),
        "/api/v1/coop/native/observation" => include_str!(
            "../../../../protocol-artifact/coop-native-v1/golden/observation-response.json"
        ),
        _ => return Err("synthetic producer received an unapproved route".to_owned()),
    };
    let mut response: Value = serde_json::from_str(fixture).map_err(|error| error.to_string())?;
    let correlation = record
        .body
        .get("correlation_id")
        .and_then(Value::as_str)
        .or_else(|| {
            record
                .headers
                .get("x-sts2-correlation-id")
                .map(String::as_str)
        })
        .ok_or_else(|| "synthetic request lacks correlation".to_owned())?;
    response["correlation_id"] = Value::String(correlation.to_owned());
    response["instance_id"] = Value::String(INSTANCE_ID.to_owned());
    response["session_id"] = Value::String(SESSION_ID.to_owned());
    response["lease_id"] = Value::String(LEASE_ID.to_owned());
    response["lease_epoch"] = json!(LEASE_EPOCH);
    if record.path == "/api/v1/coop/native/observation" {
        let local = response["observation"]["peers"]
            .as_array_mut()
            .and_then(|peers| peers.iter_mut().find(|peer| peer["role"] == "local"))
            .ok_or_else(|| "observation fixture lacks local peer".to_owned())?;
        local["peer_token"] = Value::String("peer:wrong-local".to_owned());
    }
    serde_json::to_vec(&response).map_err(|error| error.to_string())
}

fn read_http_request(stream: &mut TcpStream) -> std::io::Result<ProducerRecord> {
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    let mut head = Vec::new();
    while !head.ends_with(b"\r\n\r\n") {
        if head.len() == MAX_HTTP_BYTES {
            return Err(std::io::Error::other(
                "synthetic request headers exceed bound",
            ));
        }
        let mut byte = [0_u8; 1];
        stream.read_exact(&mut byte)?;
        head.push(byte[0]);
    }
    let text = std::str::from_utf8(&head).map_err(std::io::Error::other)?;
    let path = text
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .ok_or_else(|| std::io::Error::other("synthetic path absent"))?
        .to_owned();
    let mut headers = BTreeMap::new();
    for line in text.lines().skip(1).filter(|line| !line.is_empty()) {
        let (name, value) = line
            .split_once(':')
            .ok_or_else(|| std::io::Error::other("synthetic header malformed"))?;
        headers.insert(name.to_ascii_lowercase(), value.trim().to_owned());
    }
    let body_length = headers
        .get("content-length")
        .ok_or_else(|| std::io::Error::other("synthetic content length absent"))?
        .parse::<usize>()
        .map_err(std::io::Error::other)?;
    if head.len().saturating_add(body_length) > MAX_HTTP_BYTES {
        return Err(std::io::Error::other("synthetic request exceeds bound"));
    }
    let mut body = vec![0_u8; body_length];
    stream.read_exact(&mut body)?;
    Ok(ProducerRecord {
        path,
        headers,
        body: if body.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&body).map_err(std::io::Error::other)?
        },
    })
}

fn write_http_response(stream: &mut TcpStream, status: u16, body: &[u8]) -> std::io::Result<()> {
    write!(
        stream,
        "HTTP/1.1 {status} OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
        body.len()
    )?;
    stream.write_all(body)
}
