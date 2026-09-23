// SPDX-License-Identifier: MIT

//! Executable boundary gate for the `watchdog-recovery-v1` sideband.
//!
//! This drives the real `sts2-mcp-server` executable over stdio and the real
//! `sts2-gateway` runtime against a settled durable recovery record. The gateway
//! is the runtime binary named by `STS2_COOP_GATEWAY_BINARY`; the build is
//! supplied by the caller and its provenance is not re-verified here. The only
//! loopback producer is a synthetic recovery-mux host that terminates
//! `POST /api/v1/runtime/recovery`; it is deterministic test code, not a game
//! host and not evidence of native host behavior. No game process exists, so a
//! lookup or reconcile that replayed (re-dispatched) an effect would be visible
//! as a second dispatch control frame.

use serde_json::{Value, json};
use std::net::{SocketAddr, TcpListener};
use std::time::Duration;
use support::{
    DEPLOYMENT, INCARNATION, INSTANCE, RecoveryHost, TestResult, V3_SCHEMA, ZERO_DIGEST,
    await_gateway, operation_intent, operation_ref, original_context, recovery_arguments,
    recovery_post, recovery_request, response_payload, spawn_gateway, uuid_v4,
};

#[path = "watchdog_recovery_sideband/support.rs"]
mod support;

const LOOKUP_TOOL: &str = "watchdog.operation_lookup";
const RECONCILE_TOOL: &str = "watchdog.operation_reconcile";

#[test]
#[ignore = "requires STS2_COOP_GATEWAY_BINARY gateway runtime; synthetic loopback recovery-mux host only"]
fn real_gateway_settles_watchdog_recovery_without_game_resend() -> TestResult<()> {
    let gateway_binary = std::env::var("STS2_COOP_GATEWAY_BINARY")?;
    let reservation = TcpListener::bind("127.0.0.1:0")?;
    let address = reservation.local_addr()?;
    drop(reservation);

    let mut host = RecoveryHost::start()?;
    let store = recovery_store_path();
    let mut gateway = spawn_gateway(&gateway_binary, address, host.address, &store)?;
    await_gateway(&mut gateway, address)?;

    let boot = bootstrap(address)?;
    let fence = host_fence(address, &boot)?;
    let lease = lease_acquire(address, &boot, &fence)?;
    let context = original_context(&lease);

    let operation_id = uuid_v4();
    intent(address, &lease, &context, &operation_id)?;
    let reference = operation_ref(&operation_id, &support::action_digest(), &context);
    assert_operation(dispatch(address, &lease, &reference)?, 200, "SETTLED");

    assert_operation(lookup(address, &reference)?, 200, "SETTLED");
    assert_operation(reconcile(address, &reference, &fence)?, 200, "SETTLED");

    let mut observed = host.observed();
    assert_eq!(dispatch_count(&observed), 1, "reads must not re-dispatch");
    assert_eq!(intent_count(&observed), 1);

    let mut mcp = support::Mcp::start(address)?;
    let init = mcp.request(
        "initialize",
        json!({
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {"name": "watchdog-recovery-sideband", "version": "1"},
        }),
    )?;
    assert!(init.get("result").is_some());
    let catalog = mcp.request("tools/list", json!({}))?;
    assert_eq!(catalog["result"]["revision"], "watchdog-recovery-v1-mcp");
    assert_eq!(
        catalog["result"]["tools"]
            .as_array()
            .ok_or("tools absent")?
            .len(),
        9
    );
    let tools = catalog["result"]["tools"]
        .as_array()
        .ok_or("tools absent")?
        .iter()
        .filter_map(|tool| tool["name"].as_str())
        .collect::<Vec<_>>();
    assert!(tools.contains(&LOOKUP_TOOL) && tools.contains(&RECONCILE_TOOL));

    let mcp_lookup = mcp_settled(&mut mcp, LOOKUP_TOOL, lookup_payload(&reference))?;
    assert_eq!(mcp_lookup, "SETTLED");
    let mcp_reconcile = mcp_settled(
        &mut mcp,
        RECONCILE_TOOL,
        reconcile_payload(&reference, &fence),
    )?;
    assert_eq!(mcp_reconcile, "SETTLED");

    observed.extend(host.observed());
    assert_eq!(
        dispatch_count(&observed),
        1,
        "MCP reads must not re-dispatch"
    );

    missing_capability(address, &reference)?;

    let unknown = operation_ref(&uuid_v4(), &support::action_digest(), &context);
    let (status, frame) = lookup(address, &unknown)?;
    assert_eq!(status, 404);
    assert_eq!(response_payload(&frame)["result"]["status"], "NOT_FOUND");
    assert_eq!(response_payload(&frame)["operation"], Value::Null);
    let unknown_mcp = mcp.tools_call(LOOKUP_TOOL, recovery_arguments(lookup_payload(&unknown)))?;
    assert_eq!(mcp_frame_status(&unknown_mcp)?, "NOT_FOUND");

    observed.extend(host.observed());
    assert_eq!(dispatch_count(&observed), 1);
    assert_eq!(intent_count(&observed), 1);

    let loss_id = uuid_v4();
    intent(address, &lease, &context, &loss_id)?;
    let loss_reference = operation_ref(&loss_id, &support::action_digest(), &context);
    host.fail_next_dispatch();
    assert_operation(dispatch(address, &lease, &loss_reference)?, 503, "UNKNOWN");
    assert_operation(lookup(address, &loss_reference)?, 503, "UNKNOWN");
    let loss_mcp = mcp.tools_call(
        LOOKUP_TOOL,
        recovery_arguments(lookup_payload(&loss_reference)),
    )?;
    assert_eq!(mcp_frame_status(&loss_mcp)?, "UNKNOWN");

    observed.extend(host.observed());
    assert_eq!(dispatch_count(&observed), 2);
    assert_eq!(intent_count(&observed), 2);
    let kinds = observed
        .iter()
        .map(|request| request.kind.as_str())
        .collect::<Vec<_>>();
    assert!(kinds.contains(&"host_fence_request"));
    assert!(kinds.contains(&"lease_install_request"));
    assert!(
        observed
            .iter()
            .all(|request| request.path == support::RECOVERY_CONTROL_PATH),
        "only the closed recovery mux may be contacted"
    );
    assert!(host.quiet(Duration::from_millis(750)).is_empty());

    Ok(())
}

fn recovery_store_path() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let path = std::env::temp_dir().join(format!(
        "sts2-watchdog-recovery-sideband-{}-{nanos}.db",
        std::process::id()
    ));
    path.to_string_lossy().into_owned()
}

fn bootstrap(address: SocketAddr) -> TestResult<Value> {
    let payload = json!({
        "deployment_id": DEPLOYMENT,
        "instance_id": INSTANCE,
        "instance_incarnation": INCARNATION,
        "release": {
            "release_digest": ZERO_DIGEST,
            "config_digest": ZERO_DIGEST,
            "profile_digest": ZERO_DIGEST,
            "runtime_v3_schema_digest": V3_SCHEMA,
        },
        "lease_policy": {"ttl_seconds": 30, "renewal_interval_seconds": 10},
    });
    let (status, frame) = recovery_post(
        address,
        "/v1/recovery/bootstrap",
        "bootstrap",
        &recovery_request("bootstrap_request", "bootstrap", payload),
    )?;
    assert_eq!(status, 200, "bootstrap: {frame}");
    Ok(response_payload(&frame)["boot"].clone())
}

fn host_fence(address: SocketAddr, boot: &Value) -> TestResult<Value> {
    let (status, frame) = recovery_post(
        address,
        "/v1/recovery/host-fence",
        "host_fence",
        &recovery_request("host_fence_request", "host_fence", json!({"boot": boot})),
    )?;
    assert_eq!(status, 200, "host fence: {frame}");
    Ok(response_payload(&frame)["fence"].clone())
}

fn lease_acquire(address: SocketAddr, boot: &Value, fence: &Value) -> TestResult<Value> {
    let mut ready = boot.clone();
    ready["state"] = json!("READY");
    let (status, frame) = recovery_post(
        address,
        "/v1/recovery/lease/acquire",
        "lease_acquire",
        &recovery_request(
            "lease_acquire_request",
            "lease_acquire",
            json!({"boot": ready, "fence": fence}),
        ),
    )?;
    assert_eq!(status, 200, "lease acquire: {frame}");
    Ok(response_payload(&frame)["lease"].clone())
}

fn intent(
    address: SocketAddr,
    lease: &Value,
    context: &Value,
    operation_id: &str,
) -> TestResult<()> {
    let operation = operation_intent(operation_id, context);
    let (status, frame) = recovery_post(
        address,
        "/v1/recovery/operation/intent",
        "operation_submit",
        &recovery_request(
            "operation_intent_request",
            "operation_submit",
            json!({"lease": lease, "operation": operation}),
        ),
    )?;
    assert_eq!(status, 200, "intent: {frame}");
    assert_eq!(
        response_payload(&frame)["result"]["status"],
        "INTENT_RECORDED"
    );
    Ok(())
}

fn dispatch(address: SocketAddr, lease: &Value, reference: &Value) -> TestResult<(u16, Value)> {
    recovery_post(
        address,
        "/v1/recovery/operation/dispatch",
        "operation_submit",
        &recovery_request(
            "operation_dispatch_request",
            "operation_submit",
            json!({"lease": lease, "operation": reference}),
        ),
    )
}

fn lookup(address: SocketAddr, reference: &Value) -> TestResult<(u16, Value)> {
    recovery_post(
        address,
        "/v1/recovery/operation/lookup",
        "recovery_read",
        &recovery_request(
            "operation_lookup_request",
            "recovery_read",
            lookup_payload(reference),
        ),
    )
}

fn reconcile(address: SocketAddr, reference: &Value, fence: &Value) -> TestResult<(u16, Value)> {
    recovery_post(
        address,
        "/v1/recovery/operation/reconcile",
        "recovery_reconcile",
        &recovery_request(
            "operation_reconcile_request",
            "recovery_reconcile",
            reconcile_payload(reference, fence),
        ),
    )
}

fn lookup_payload(reference: &Value) -> Value {
    json!({"operation": reference, "lookup_scope": "historical_read"})
}

fn reconcile_payload(reference: &Value, fence: &Value) -> Value {
    json!({"operation": reference, "strategy": "reobserve", "current_fence": fence})
}

fn missing_capability(address: SocketAddr, reference: &Value) -> TestResult<()> {
    let (status, body) = recovery_post(
        address,
        "/v1/recovery/operation/lookup",
        "operation_submit",
        &recovery_request(
            "operation_lookup_request",
            "recovery_read",
            lookup_payload(reference),
        ),
    )?;
    assert_eq!(status, 403);
    assert_eq!(body["error_code"], "recovery_capability_forbidden");
    Ok(())
}

fn mcp_settled(mcp: &mut support::Mcp, tool: &str, payload: Value) -> TestResult<String> {
    let response = mcp.tools_call(tool, recovery_arguments(payload))?;
    assert_eq!(
        response["result"]["isError"], false,
        "mcp error: {response}"
    );
    let frame = mcp_frame(&response)?;
    let state = response_payload(&frame)["operation"]["state"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    Ok(state)
}

fn mcp_frame(response: &Value) -> TestResult<Value> {
    let text = response["result"]["content"][0]["text"]
        .as_str()
        .ok_or("mcp content text absent")?;
    Ok(serde_json::from_str(text)?)
}

/// The recovery adapter surfaces the gateway frame verbatim, so an unresolved
/// or unknown read is a successful tool call whose payload carries the outcome.
fn mcp_frame_status(response: &Value) -> TestResult<String> {
    assert_eq!(
        response["result"]["isError"], false,
        "recovery frames surface verbatim: {response}"
    );
    let frame = mcp_frame(response)?;
    Ok(response_payload(&frame)["result"]["status"]
        .as_str()
        .unwrap_or_default()
        .to_owned())
}

fn assert_operation(response: (u16, Value), status: u16, state: &str) {
    let (actual, frame) = response;
    assert_eq!(actual, status, "operation status: {frame}");
    let payload = response_payload(&frame);
    assert_eq!(payload["operation"]["state"], state);
}

fn dispatch_count(requests: &[support::HostRequest]) -> usize {
    requests
        .iter()
        .filter(|request| request.kind == "operation_dispatch_request")
        .count()
}

fn intent_count(requests: &[support::HostRequest]) -> usize {
    requests
        .iter()
        .filter(|request| request.kind == "operation_intent_request")
        .count()
}
