// SPDX-License-Identifier: MIT
#![allow(clippy::unwrap_used)] // Local contract fixtures fail immediately.

use sts2_mcp_server::JsonValue;

use super::remote_offers;

fn offer(required_scope: &str, scope: &[&str]) -> JsonValue {
    JsonValue::object([
        (
            String::from("operation"),
            JsonValue::string("runtime_v3.state"),
        ),
        (
            String::from("revision"),
            JsonValue::string("runtime-v3-gameplay"),
        ),
        (
            String::from("required_scope"),
            JsonValue::string(required_scope),
        ),
        (
            String::from("scope"),
            JsonValue::Array(
                scope
                    .iter()
                    .map(|value| JsonValue::string(*value))
                    .collect(),
            ),
        ),
        (
            String::from("wire_limits"),
            JsonValue::object([
                (String::from("max_request_bytes"), JsonValue::Number(16_384)),
                (
                    String::from("max_response_bytes"),
                    JsonValue::Number(131_072),
                ),
            ]),
        ),
        (
            String::from("content_limits"),
            JsonValue::object([
                (
                    String::from("max_content_bytes"),
                    JsonValue::Number(131_072),
                ),
                (String::from("max_page_items"), JsonValue::Number(1)),
            ]),
        ),
    ])
}

fn snapshot(offers: Vec<JsonValue>) -> JsonValue {
    JsonValue::object([(String::from("offers"), JsonValue::Array(offers))])
}

#[test]
fn remote_offer_parser_keeps_wire_and_content_budgets_separate() {
    let parsed = remote_offers(&snapshot(vec![offer("read", &["read"])]));
    let parsed = parsed.unwrap();
    assert_eq!(parsed.len(), 1);
    let offer = parsed.get("runtime_v3.state").unwrap();
    assert_eq!(offer.wire_limits.max_request_bytes, 16_384);
    assert_eq!(offer.wire_limits.max_response_bytes, 131_072);
    assert_eq!(offer.content_limits.max_content_bytes, 131_072);
    assert_eq!(offer.content_limits.max_page_items, 1);
}

#[test]
fn duplicate_offers_and_scope_mismatch_are_rejected() {
    assert!(
        remote_offers(&snapshot(vec![
            offer("read", &["read"]),
            offer("read", &["read"]),
        ]))
        .is_err()
    );
    assert!(remote_offers(&snapshot(vec![offer("mutate", &["read"])])).is_err());
}
