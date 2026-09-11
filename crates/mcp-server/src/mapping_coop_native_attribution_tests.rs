// SPDX-License-Identifier: MIT

use super::*;
use crate::gateway::GatewayMethod;

fn replace_local_peer(response: &mut GatewayResponse) -> Result<(), String> {
    let JsonValue::Object(root) = &mut response.body else {
        return Err(String::from("native response is not an object"));
    };
    let Some(JsonValue::Object(observation)) = root.get_mut("observation") else {
        return Err(String::from("native response observation is missing"));
    };
    let Some(JsonValue::Array(peers)) = observation.get_mut("peers") else {
        return Err(String::from("native response peers are missing"));
    };
    let Some(JsonValue::Object(local)) = peers.first_mut() else {
        return Err(String::from("native local peer is missing"));
    };
    local.insert(
        String::from("peer_token"),
        JsonValue::string("peer:foreign"),
    );
    Ok(())
}

fn assert_attribution_error(output: &str) {
    assert!(output.contains(r#""isError":true"#), "{output}");
    assert!(
        output.contains("local peer does not match configured gateway peer"),
        "{output}"
    );
}

#[test]
fn rejects_catalog_observation_not_attributable_to_bound_peer() -> Result<(), String> {
    let mut gateway_response = response("catalog", "corr-catalog-peer", None)?;
    replace_local_peer(&mut gateway_response)?;
    let mut server = server(gateway_response).with_native_peer_id("peer:host1");
    let arguments = format!(
        r#"{{{},"actor_peer":"peer:host1","expected_host_generation":1}}"#,
        common()
    );
    assert_attribution_error(&server.handle_frame(&frame(
        "corr-catalog-peer",
        COOP_NATIVE_LEGAL_CATALOG_TOOL,
        &arguments,
    )));
    Ok(())
}

#[test]
fn rejects_effect_observation_not_attributable_to_bound_peer() -> Result<(), String> {
    let mut gateway_response = response("effect", "corr-effect-peer", Some("op-effect-peer"))?;
    replace_local_peer(&mut gateway_response)?;
    let mut server = server(gateway_response).with_native_peer_id("peer:host1");
    let arguments = format!(
        r#"{{{},"operation_id":"op-effect-peer","actor_peer":"peer:host1","expected_host_generation":1,"action":{{"kind":"end_turn","action_id":"turn:1","target_peer":null}}}}"#,
        common()
    );
    assert_attribution_error(&server.handle_frame(&frame(
        "corr-effect-peer",
        COOP_NATIVE_ACTION_TOOL,
        &arguments,
    )));
    Ok(())
}

#[test]
fn rejects_settled_and_unknown_recovery_observations_without_changing_fence() -> Result<(), String>
{
    for status in ["settled", "unknown"] {
        let operation = format!("op-recover-{status}-peer");
        let mut gateway_response = response("recovery", "corr-recover-peer", Some(&operation))?;
        if status == "unknown" {
            let JsonValue::Object(root) = &mut gateway_response.body else {
                return Err(String::from("recovery response is not an object"));
            };
            root.insert(String::from("status"), JsonValue::string("unknown"));
            let Some(JsonValue::Object(receipt)) = root.get_mut("receipt") else {
                return Err(String::from("recovery receipt is missing"));
            };
            receipt.insert(String::from("status"), JsonValue::string("unknown"));
            receipt.insert(String::from("after_host_generation"), JsonValue::Null);
        }
        replace_local_peer(&mut gateway_response)?;
        let mut server = server(gateway_response).with_native_peer_id("peer:client1");
        let arguments = format!(
            r#"{{{},"operation_id":"{operation}","recovery":{{"kind":"reconcile","rejoin_epoch":3}}}}"#,
            common()
        );
        assert_attribution_error(&server.handle_frame(&frame(
            "corr-recover-peer",
            COOP_NATIVE_RECOVER_TOOL,
            &arguments,
        )));
        let request = server
            .gateway()
            .requests
            .first()
            .ok_or_else(|| String::from("recovery did not reach gateway"))?;
        assert_eq!(request.method, GatewayMethod::Post, "{status}");
        assert_eq!(request.path, "/v1/instances/instance-1/coop/native/recover");
        assert_eq!(
            request
                .body
                .as_ref()
                .and_then(JsonValue::as_object)
                .and_then(|body| body.get("operation_id"))
                .and_then(JsonValue::as_string),
            Some(operation.as_str()),
            "{status}"
        );
        for (name, expected) in [
            ("x-sts2-instance-id", "instance-1"),
            ("x-sts2-session-id", "session-1"),
            ("x-sts2-lease-id", "lease-1"),
            ("x-sts2-lease-epoch", "1"),
        ] {
            assert_eq!(
                request.headers.get(name).map(String::as_str),
                Some(expected)
            );
        }
    }
    Ok(())
}
