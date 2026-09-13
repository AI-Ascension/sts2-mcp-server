// SPDX-License-Identifier: MIT

use sts2_mcp_server::{JsonValue, SAVE_PROFILE_CONTRACT};

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
            || object.get("contract") == Some(&JsonValue::string(SAVE_PROFILE_CONTRACT))
    )
}

pub(crate) fn is_save_profile_result(body: &JsonValue) -> bool {
    matches!(
        body,
        JsonValue::Object(object)
            if object.get("contract") == Some(&JsonValue::string(SAVE_PROFILE_CONTRACT))
                || object.get("error_code").is_some()
    )
}
