// SPDX-License-Identifier: MIT

#[path = "coop_gateway_runtime/support.rs"]
mod support;

use serde_json::{Value, json};
use std::net::{SocketAddr, TcpListener};
use support::{
    CONTROL_TOKEN, Mcp, READ_TOKEN, TestResult, await_gateway, http, report, spawn_gateway,
};

#[test]
#[ignore = "Requires the exact reviewed gateway binary in STS2_COOP_GATEWAY_BINARY; this is a separate executable integration gate"]
fn real_gateway_and_mcp_complete_peer_report_lifecycle() -> TestResult<()> {
    let gateway_binary = std::env::var("STS2_COOP_GATEWAY_BINARY")?;
    let reservation = TcpListener::bind("127.0.0.1:0")?;
    let address = reservation.local_addr()?;
    drop(reservation);
    let downstream = TcpListener::bind("127.0.0.1:0")?;
    downstream.set_nonblocking(true)?;
    let mut gateway = spawn_gateway(&gateway_binary, address, downstream.local_addr()?)?;
    await_gateway(&mut gateway, address)?;
    let allocation = http(
        address,
        "POST",
        "/v1/sessions/allocate",
        CONTROL_TOKEN,
        Some(json!({"instance_id":"instance-1","caller_id":"harness","session_id":"session-1"})),
        None,
    )?;
    assert_eq!(allocation.0, 200);
    let mut mcp = Mcp::start(address)?;
    let init = mcp.request("initialize", json!({"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"co-op-integration","version":"1"}}))?;
    assert!(init.get("result").is_some());
    let catalog = mcp.request("tools/list", json!({}))?;
    assert_eq!(
        catalog["result"]["tools"]
            .as_array()
            .ok_or("tools absent")?
            .len(),
        1
    );
    assert_eq!(
        catalog["result"]["tools"][0]["name"],
        "sts2.coop_synchronization"
    );
    lifecycle(address, &mut mcp)?;
    refusals(address, &mut mcp)?;
    assert!(
        matches!(downstream.accept(), Err(error) if error.kind() == std::io::ErrorKind::WouldBlock),
        "co-op routes must never contact the game"
    );
    Ok(())
}

fn assert_snapshot(value: &Value, status: &str, generation: u64) -> TestResult<()> {
    assert_eq!(value["source"], "gateway_peer_reports");
    assert_eq!(value["synchronization"]["status"], status);
    assert_eq!(value["generation"], generation);
    assert_eq!(value["session_id"], "session-1");
    let schema: Value = serde_json::from_str(include_str!(
        "../../../protocol-artifact/coop-synchronization-v1/schema.json"
    ))?;
    assert!(
        jsonschema::draft202012::options()
            .build(&schema)?
            .is_valid(value)
    );
    Ok(())
}

fn lifecycle(address: SocketAddr, mcp: &mut Mcp) -> TestResult<()> {
    let initial = mcp.synchronization()?;
    assert_snapshot(&initial, "disconnected", 0)?;
    assert_eq!(
        initial["synchronization"]["missing_peers"],
        json!(["local-1", "ally-1"])
    );
    assert_eq!(report(address, "local-1", 4, true)?.0, 200);
    assert_snapshot(&mcp.synchronization()?, "disconnected", 0)?;
    assert_eq!(report(address, "ally-1", 4, true)?.0, 200);
    assert_snapshot(&mcp.synchronization()?, "synchronized", 4)?;
    assert_eq!(report(address, "local-1", 5, true)?.0, 200);
    assert_snapshot(&mcp.synchronization()?, "disagreement", 4)?;
    assert_eq!(report(address, "ally-1", 4, false)?.0, 200);
    assert_snapshot(&mcp.synchronization()?, "disconnected", 4)?;
    assert_eq!(report(address, "ally-1", 5, true)?.0, 200);
    assert_snapshot(&mcp.synchronization()?, "synchronized", 5)?;
    Ok(())
}

fn refusals(address: SocketAddr, mcp: &mut Mcp) -> TestResult<()> {
    assert_eq!(report(address, "local-1", 4, true)?.0, 409);
    assert_eq!(report(address, "foreign", 5, true)?.0, 409);
    let denied = http(
        address,
        "POST",
        "/v1/instances/instance-1/coop/peer-report",
        READ_TOKEN,
        Some(json!({"peer_id":"local-1","generation":5,"connected":true})),
        None,
    )?;
    assert_eq!(denied.0, 403);
    let stale = http(
        address,
        "GET",
        "/v1/instances/instance-1/coop/synchronization",
        READ_TOKEN,
        None,
        Some(("x-sts2-lease-epoch", "2")),
    )?;
    assert_eq!(stale.0, 409);
    let before = mcp.synchronization()?;
    assert_snapshot(&before, "synchronized", 5)?;
    assert_eq!(
        http(
            address,
            "POST",
            "/v1/instances/instance-1/release",
            CONTROL_TOKEN,
            None,
            None
        )?
        .0,
        200
    );
    let after = mcp.request("tools/call",json!({"name":"sts2.coop_synchronization","arguments":{
        "instance_id":"instance-1","mcp_session_id":"mcp-session-1","lease_id":"lease-1","lease_epoch":1,
    }}))?;
    assert_eq!(after["result"]["isError"], true);
    Ok(())
}
