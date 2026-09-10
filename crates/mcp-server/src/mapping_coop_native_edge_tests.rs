// SPDX-License-Identifier: MIT

use super::*;

#[test]
fn rejects_duplicate_catalog_ids_and_foreign_voters() -> Result<(), String> {
    let valid = response("catalog", "corr-catalog-invalid", None)?.body;
    for mutation in ["duplicate", "foreign-voter"] {
        let mut body = valid.clone();
        let JsonValue::Object(root) = &mut body else {
            return Err(String::from("catalog response is not an object"));
        };
        let JsonValue::Object(catalog) = root
            .get_mut("catalog")
            .ok_or_else(|| String::from("catalog member is missing"))?
        else {
            return Err(String::from("catalog member is not an object"));
        };
        if mutation == "duplicate" {
            let JsonValue::Array(actions) = catalog
                .get_mut("actions")
                .ok_or_else(|| String::from("catalog actions are missing"))?
            else {
                return Err(String::from("catalog actions are not an array"));
            };
            let first = actions
                .first()
                .and_then(JsonValue::as_object)
                .and_then(|action| action.get("action_id"))
                .cloned()
                .ok_or_else(|| String::from("first action ID is missing"))?;
            let JsonValue::Object(second) = actions
                .get_mut(1)
                .ok_or_else(|| String::from("second action is missing"))?
            else {
                return Err(String::from("second action is not an object"));
            };
            second.insert(String::from("action_id"), first);
        } else {
            let JsonValue::Array(votes) = catalog
                .get_mut("votes")
                .ok_or_else(|| String::from("catalog votes are missing"))?
            else {
                return Err(String::from("catalog votes are not an array"));
            };
            let JsonValue::Object(vote) = votes
                .first_mut()
                .ok_or_else(|| String::from("catalog vote is missing"))?
            else {
                return Err(String::from("catalog vote is not an object"));
            };
            vote.insert(
                String::from("voter_peer"),
                JsonValue::string("peer:client1"),
            );
        }
        let mut server = server(GatewayResponse { status: 200, body });
        let arguments = format!(
            r#"{{{} ,"actor_peer":"peer:host1","expected_host_generation":1}}"#,
            common()
        );
        let output = server.handle_frame(&frame(
            "corr-catalog-invalid",
            COOP_NATIVE_LEGAL_CATALOG_TOOL,
            &arguments,
        ));
        assert!(output.contains(r#""isError":true"#), "{mutation}: {output}");
    }
    Ok(())
}

#[test]
fn effect_tool_projects_a_rejected_response_as_an_error_without_http_status() -> Result<(), String>
{
    let mut body = response("effect", "corr-effect-rejected", Some("op-effect-rejected"))?.body;
    let JsonValue::Object(root) = &mut body else {
        return Err(String::from("effect response is not an object"));
    };
    root.insert(String::from("status"), JsonValue::string("rejected"));
    root.insert(String::from("effect"), JsonValue::Null);
    let JsonValue::Object(observation) = root
        .get_mut("observation")
        .ok_or_else(|| String::from("effect observation is missing"))?
    else {
        return Err(String::from("effect observation is not an object"));
    };
    observation.insert(String::from("host_generation"), JsonValue::Number(1));
    let JsonValue::Object(receipt) = root
        .get_mut("receipt")
        .ok_or_else(|| String::from("effect receipt is missing"))?
    else {
        return Err(String::from("effect receipt is not an object"));
    };
    receipt.insert(String::from("status"), JsonValue::string("rejected"));
    receipt.insert(String::from("after_host_generation"), JsonValue::Null);
    receipt.insert(
        String::from("error_code"),
        JsonValue::string("native_action_rejected"),
    );
    let mut server = server(GatewayResponse {
        status: 500,
        body: JsonValue::Null,
    });
    let arguments = format!(r#"{{{} ,"envelope":{}}}"#, common(), body.to_json());
    let output = server.handle_frame(&frame(
        "corr-effect-rejected",
        COOP_NATIVE_EFFECT_TOOL,
        &arguments,
    ));
    assert!(output.contains(r#""isError":true"#), "{output}");
    assert!(server.gateway().requests.is_empty());
    Ok(())
}

#[test]
fn recovery_request_echo_is_not_reported_as_a_successful_response() -> Result<(), String> {
    let mut body = response("recovery", "corr-recover-echo", Some("op-recover-echo"))?.body;
    let JsonValue::Object(root) = &mut body else {
        return Err(String::from("recovery response is not an object"));
    };
    root.insert(String::from("status"), JsonValue::Null);
    root.insert(String::from("observation"), JsonValue::Null);
    root.insert(String::from("receipt"), JsonValue::Null);
    let JsonValue::Object(recovery) = root
        .get_mut("recovery")
        .ok_or_else(|| String::from("recovery member is missing"))?
    else {
        return Err(String::from("recovery member is not an object"));
    };
    recovery.insert(String::from("kind"), JsonValue::string("reconcile"));
    let mut server = server(GatewayResponse { status: 200, body });
    let arguments = format!(
        r#"{{{} ,"operation_id":"op-recover-echo","recovery":{{"kind":"reconcile","rejoin_epoch":3}}}}"#,
        common()
    );
    let output = server.handle_frame(&frame(
        "corr-recover-echo",
        COOP_NATIVE_RECOVER_TOOL,
        &arguments,
    ));
    assert!(output.contains(r#""isError":true"#), "{output}");
    Ok(())
}
