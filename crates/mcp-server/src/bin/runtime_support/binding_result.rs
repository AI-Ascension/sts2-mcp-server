// SPDX-License-Identifier: MIT

use sts2_mcp_server::JsonValue;

pub(crate) fn is_runtime_result(body: &JsonValue) -> bool {
    matches!(
        body,
        JsonValue::Object(object)
            if matches!(
                object.get("kind"),
                Some(JsonValue::String(kind))
                    if matches!(
                        kind.as_str(),
                        "state_response"
                            | "action_response"
                            | "reconcile_response"
                            | "legal_actions_response"
                            | "dispatch_action_response"
                            | "wait_response"
                            | "reobserve_response"
                            | "recover_response"
                            | "snapshot_response"
                            | "receipt_query_response"
                            | "start_response"
                            | "observation"
                            | "legal_catalog_response"
                            | "effect_response"
                            | "recovery_response"
                            | "query_response"
                            | "capabilities_response"
                            | "error_response"
                    )
            )
    )
}
