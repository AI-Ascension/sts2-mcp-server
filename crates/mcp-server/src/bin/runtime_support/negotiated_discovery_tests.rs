// SPDX-License-Identifier: MIT
#![allow(clippy::unwrap_used)]

use super::{CORRELATION_ID, DiscoveryRequest, MAX_REQUEST_BYTES, parse};

const VALID: &str = r#"{"operation":"discovery","scope":{"project_id":"project","run_id":"run","episode_id":"episode","agent_id":"agent"},"authority_epoch":1,"correlation_id":"game-information-binding-discovery"}"#;

#[test]
fn accepts_only_the_bounded_fixed_discovery_request() {
    assert_eq!(
        parse(VALID).unwrap(),
        DiscoveryRequest {
            project_id: String::from("project"),
            run_id: String::from("run"),
            episode_id: String::from("episode"),
            agent_id: String::from("agent"),
            authority_epoch: 1,
        }
    );
    assert_eq!(CORRELATION_ID, "game-information-binding-discovery");
}

#[test]
fn rejects_duplicate_unknown_wrong_operation_and_unsafe_epoch() {
    for input in [
        VALID.replace(
            r#""authority_epoch":1"#,
            r#""authority_epoch":1,"authority_epoch":2"#,
        ),
        VALID.replace(r#""operation":"discovery""#, r#""operation":"observe""#),
        VALID.replace(r#""authority_epoch":1"#, r#""authority_epoch":0"#),
        VALID.replace(
            r#""correlation_id":"game-information-binding-discovery""#,
            r#""correlation_id":"caller-controlled""#,
        ),
        VALID.replace(
            r#""agent_id":"agent""#,
            r#""agent_id":"agent","unrecognized":true"#,
        ),
    ] {
        assert!(parse(&input).is_err(), "accepted {input}");
    }
}

#[test]
fn rejects_the_request_before_parsing_when_over_byte_bound() {
    let oversized = format!("{VALID}{}", " ".repeat(MAX_REQUEST_BYTES));
    assert!(parse(&oversized).is_err());
}
