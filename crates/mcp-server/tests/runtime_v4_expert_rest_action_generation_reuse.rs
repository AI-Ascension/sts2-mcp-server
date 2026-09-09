// SPDX-License-Identifier: MIT

use std::collections::VecDeque;

use sts2_mcp_server::{GatewayResponse, McpServer, ToolCatalog};

#[path = "support/generation_reuse_fixtures.rs"]
mod fixtures;
use fixtures::*;

fn call(tool: &str, arguments: &str) -> String {
    format!(
        "{{\"jsonrpc\":\"2.0\",\"id\":\"request-1\",\"method\":\"tools/call\",\"params\":{{\"name\":\"{tool}\",\"arguments\":{{{arguments}}}}}}}"
    )
}

#[test]
fn newer_generation_reuses_selection_id_with_fresh_admission_and_capacity_fence() {
    let a_open = response_fixture("smith-requested", "reuse-a-open");
    let a_one = accepted_for_action(
        "reuse-a-card-one",
        r#"{"action_id":"select_card:10:smith:card:1","action":{"kind":"select_card","selection_id":"selection:10:smith","rest_option_id":"smith","card_id":"card:1"}}"#,
        10,
    );
    let a_one_progress = response_fixture("smith-first", "reuse-a-card-one");
    let a_two = response_fixture("smith-second", "reuse-a-card-two");
    let a_complete = response_fixture("smith-completed", "reuse-a-complete");
    let b_open = newer_smith_requested("reuse-b-open");
    let b_one = accepted_for_action(
        "reuse-b-card-one",
        r#"{"action_id":"select_card:20:smith:card:3","action":{"kind":"select_card","selection_id":"selection:10:smith","rest_option_id":"smith","card_id":"card:3"}}"#,
        20,
    );
    let b_one_progress = newer_smith_progressed("smith-first", "reuse-b-card-one", 21, 20, 21);
    let b_two = newer_smith_progressed("smith-second", "reuse-b-card-two", 22, 21, 22);
    let b_complete = newer_smith_completed("reuse-b-complete");

    let mut responses = VecDeque::from([
        GatewayResponse {
            status: 200,
            body: a_open,
        },
        GatewayResponse {
            status: 202,
            body: a_one,
        },
        GatewayResponse {
            status: 200,
            body: a_one_progress,
        },
        GatewayResponse {
            status: 200,
            body: a_two,
        },
        GatewayResponse {
            status: 200,
            body: a_complete,
        },
    ]);
    for index in 0..127 {
        responses.push_back(GatewayResponse {
            status: 200,
            body: fresh_smith_requested(index, &format!("evict-before-b-{index}")),
        });
    }
    responses.extend([
        GatewayResponse {
            status: 200,
            body: b_open,
        },
        GatewayResponse {
            status: 200,
            body: response_fixture("smith-completed", "reuse-a-complete"),
        },
        GatewayResponse {
            status: 200,
            body: response_fixture("smith-completed", "reuse-a-complete"),
        },
        GatewayResponse {
            status: 200,
            body: response_fixture("smith-first", "reuse-a-card-one"),
        },
        GatewayResponse {
            status: 202,
            body: b_one,
        },
        GatewayResponse {
            status: 200,
            body: b_one_progress,
        },
        GatewayResponse {
            status: 200,
            body: b_two,
        },
        GatewayResponse {
            status: 200,
            body: b_complete,
        },
    ]);
    let mut server = McpServer::with_catalog_and_sessions(
        RecordingGateway {
            requests: Vec::new(),
            responses,
        },
        ToolCatalog::runtime_v4_expert_rest_action(),
        "session-1",
        "mcp-session-1",
    );
    let action_a_open = r#"{"action_id":"rest-option:9:smith","action":{"kind":"rest_option","rest_option_id":"smith"}}"#;
    let action_b_open = selector_action(200).to_json();
    let action_a_one = r#"{"action_id":"select_card:10:smith:card:1","action":{"kind":"select_card","selection_id":"selection:10:smith","rest_option_id":"smith","card_id":"card:1"}}"#;
    let action_a_two = r#"{"action_id":"select_card:11:smith:card:2","action":{"kind":"select_card","selection_id":"selection:10:smith","rest_option_id":"smith","card_id":"card:2"}}"#;
    let action_a_confirm = r#"{"action_id":"confirm_selection:12:smith","action":{"kind":"confirm_selection","selection_id":"selection:10:smith","rest_option_id":"smith"}}"#;
    let action_b_one = r#"{"action_id":"select_card:20:smith:card:3","action":{"kind":"select_card","selection_id":"selection:10:smith","rest_option_id":"smith","card_id":"card:3"}}"#;
    let action_b_two = r#"{"action_id":"select_card:21:smith:card:4","action":{"kind":"select_card","selection_id":"selection:10:smith","rest_option_id":"smith","card_id":"card:4"}}"#;
    let action_b_confirm = r#"{"action_id":"confirm_selection:22:smith","action":{"kind":"confirm_selection","selection_id":"selection:10:smith","rest_option_id":"smith"}}"#;
    let action_request = |operation: &str, generation: i64, action: &str| {
        call(
            "sts2.expert_rest_action",
            &format!(
                "\"instance_id\":\"instance-1\",\"mcp_session_id\":\"mcp-session-1\",\"lease_id\":\"lease-1\",\"lease_epoch\":4,\"generation\":{generation},\"state_id\":\"live:{generation}\",\"operation_id\":\"{operation}\",\"action\":{action}"
            ),
        )
    };
    let reconcile = |operation: &str| {
        call(
            "sts2.expert_rest_reconcile",
            &format!(
                "\"instance_id\":\"instance-1\",\"mcp_session_id\":\"mcp-session-1\",\"lease_id\":\"lease-1\",\"lease_epoch\":4,\"operation_id\":\"{operation}\""
            ),
        )
    };
    for (operation, generation, action) in [
        ("reuse-a-open", 9, action_a_open),
        ("reuse-a-card-one", 10, action_a_one),
    ] {
        assert!(
            server
                .handle_frame(&action_request(operation, generation, action))
                .contains("\"isError\":false")
        );
        if operation == "reuse-a-card-one" {
            assert!(
                server
                    .handle_frame(&reconcile("reuse-a-card-one"))
                    .contains("\"isError\":false")
            );
        }
    }
    for (operation, generation, action) in [
        ("reuse-a-card-two", 11, action_a_two),
        ("reuse-a-complete", 12, action_a_confirm),
    ] {
        assert!(
            server
                .handle_frame(&action_request(operation, generation, action))
                .contains("\"isError\":false")
        );
    }
    for index in 0..127 {
        let action = selector_action(index).to_json();
        assert!(
            server
                .handle_frame(&action_request(
                    &format!("evict-before-b-{index}"),
                    9,
                    &action,
                ))
                .contains("\"isError\":false")
        );
    }
    let b_open_output = server.handle_frame(&action_request("reuse-b-open", 19, &action_b_open));
    assert!(
        b_open_output.contains("\"isError\":false"),
        "{b_open_output}"
    );
    assert!(
        b_open_output.contains("select_card:20:smith:card:3")
            && b_open_output.contains("select_card:20:smith:card:4")
            && !b_open_output.contains("select_card:10:smith:card:1")
            && !b_open_output.contains("select_card:10:smith:card:2"),
        "{b_open_output}"
    );
    for operation in ["reuse-a-complete", "reuse-a-complete", "reuse-a-card-one"] {
        let output = server.handle_frame(&reconcile(operation));
        assert!(output.contains("\"isError\":false"), "{output}");
    }
    assert!(
        server
            .handle_frame(&action_request("reuse-b-card-one", 20, action_b_one))
            .contains("\"isError\":false")
    );
    let b_one_get_output = server.handle_frame(&reconcile("reuse-b-card-one"));
    assert!(
        b_one_get_output.contains("\"isError\":false"),
        "{b_one_get_output}"
    );
    let capacity = action_request(
        "capacity-after-b",
        9,
        r#"{"action_id":"rest-option:9:smith:capacity","action":{"kind":"rest_option","rest_option_id":"smith"}}"#,
    );
    assert!(
        server
            .handle_frame(&capacity)
            .contains("selector admission capacity is exhausted")
    );
    assert_eq!(server.gateway().requests.len(), 138);
    let b_two_output = server.handle_frame(&action_request("reuse-b-card-two", 21, action_b_two));
    assert!(b_two_output.contains("\"isError\":false"), "{b_two_output}");
    assert!(
        b_two_output.contains("confirm_selection:22:smith")
            && !b_two_output.contains("confirm_selection:12:smith"),
        "{b_two_output}"
    );
    assert!(
        server
            .handle_frame(&action_request("reuse-b-complete", 22, action_b_confirm))
            .contains("\"isError\":false")
    );
    assert_eq!(server.gateway().requests.len(), 140);
}
