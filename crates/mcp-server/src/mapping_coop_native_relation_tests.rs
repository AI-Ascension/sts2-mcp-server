// SPDX-License-Identifier: MIT

use super::*;

#[test]
fn rejects_effect_relation_drift_and_status_mismatch() -> Result<(), String> {
    let mut drifted = response("effect", "corr-drift", Some("op-drift"))?.body;
    let JsonValue::Object(root) = &mut drifted else {
        return Err(String::from("effect response is not an object"));
    };
    let JsonValue::Object(receipt) = root
        .get_mut("receipt")
        .ok_or_else(|| String::from("effect receipt is missing"))?
    else {
        return Err(String::from("effect receipt is not an object"));
    };
    receipt.insert(
        String::from("state_digest"),
        JsonValue::string("cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"),
    );
    let mut drift_server = server(GatewayResponse {
        status: 200,
        body: drifted,
    });
    let arguments = format!(
        r#"{{{} ,"operation_id":"op-drift","actor_peer":"peer:host1","expected_host_generation":1,"action":{{"kind":"end_turn","action_id":"turn:1","target_peer":null}}}}"#,
        common()
    );
    let output =
        drift_server.handle_frame(&frame("corr-drift", COOP_NATIVE_ACTION_TOOL, &arguments));
    assert!(output.contains(r#""isError":true"#), "{output}");

    let mut status_server = server(GatewayResponse {
        status: 409,
        body: response("effect", "corr-status", Some("op-status"))?.body,
    });
    let arguments = format!(
        r#"{{{} ,"operation_id":"op-status","actor_peer":"peer:host1","expected_host_generation":1,"action":{{"kind":"end_turn","action_id":"turn:1","target_peer":null}}}}"#,
        common()
    );
    let output =
        status_server.handle_frame(&frame("corr-status", COOP_NATIVE_ACTION_TOOL, &arguments));
    assert!(output.contains(r#""isError":true"#), "{output}");
    Ok(())
}

#[test]
fn rejects_rejoin_terminal_kind_drift() -> Result<(), String> {
    let mut body = response("recovery", "corr-rejoin-drift", Some("op-rejoin"))?.body;
    let JsonValue::Object(root) = &mut body else {
        return Err(String::from("recovery response is not an object"));
    };
    let JsonValue::Object(recovery) = root
        .get_mut("recovery")
        .ok_or_else(|| String::from("recovery member is missing"))?
    else {
        return Err(String::from("recovery member is not an object"));
    };
    recovery.insert(String::from("kind"), JsonValue::string("rejoin"));
    let mut server = server(GatewayResponse { status: 200, body });
    let arguments = format!(
        r#"{{{} ,"operation_id":"op-rejoin","actor_peer":"peer:client1","expected_host_generation":1,"recovery":{{"kind":"rejoin","rejoin_epoch":3}}}}"#,
        common()
    );
    let output = server.handle_frame(&frame(
        "corr-rejoin-drift",
        COOP_NATIVE_REJOIN_TOOL,
        &arguments,
    ));
    assert!(output.contains(r#""isError":true"#), "{output}");
    Ok(())
}

#[test]
fn rejects_unknown_reconcile_response_on_rejoin_route() -> Result<(), String> {
    let body = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../protocol-artifact/coop-native-v1/golden/rejoin-pending-response.json"
    ))
    .replace("corr:native:rejoin", "corr-rejoin-unknown")
    .replace("instance:native-test", "instance-1")
    .replace("session:native-test", "session-1")
    .replace("lease:native-test", "lease-1")
    .replace("\"lease_epoch\":7", "\"lease_epoch\":1")
    .replace("op:native:rejoin", "op-rejoin-unknown");
    let mut value = parse_json(&body)?;
    let JsonValue::Object(root) = &mut value else {
        return Err(String::from("recovery response is not an object"));
    };
    let JsonValue::Object(recovery) = root
        .get_mut("recovery")
        .ok_or_else(|| String::from("recovery member is missing"))?
    else {
        return Err(String::from("recovery member is not an object"));
    };
    recovery.insert(String::from("kind"), JsonValue::string("reconcile"));
    let mut server = server(GatewayResponse {
        status: 200,
        body: value,
    });
    let arguments = format!(
        r#"{{{} ,"operation_id":"op-rejoin-unknown","actor_peer":"peer:client1","expected_host_generation":1,"recovery":{{"kind":"rejoin","rejoin_epoch":3}}}}"#,
        common()
    );
    let output = server.handle_frame(&frame(
        "corr-rejoin-unknown",
        COOP_NATIVE_REJOIN_TOOL,
        &arguments,
    ));
    assert!(output.contains(r#""isError":true"#), "{output}");
    Ok(())
}

#[test]
fn rejects_rejoin_kind_on_reconcile_route_for_unknown_outcome() -> Result<(), String> {
    let body = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../protocol-artifact/coop-native-v1/golden/rejoin-pending-response.json"
    ))
    .replace("corr:native:rejoin", "corr-recover-unknown")
    .replace("instance:native-test", "instance-1")
    .replace("session:native-test", "session-1")
    .replace("lease:native-test", "lease-1")
    .replace("\"lease_epoch\":7", "\"lease_epoch\":1")
    .replace("op:native:rejoin", "op-recover-unknown");
    let mut server = server(GatewayResponse {
        status: 200,
        body: parse_json(&body)?,
    });
    let arguments = format!(
        r#"{{{} ,"operation_id":"op-recover-unknown","recovery":{{"kind":"reconcile","rejoin_epoch":3}}}}"#,
        common()
    );
    let output = server.handle_frame(&frame(
        "corr-recover-unknown",
        COOP_NATIVE_RECOVER_TOOL,
        &arguments,
    ));
    assert!(output.contains(r#""isError":true"#), "{output}");
    Ok(())
}
