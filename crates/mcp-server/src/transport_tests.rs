// SPDX-License-Identifier: MIT
#![allow(clippy::unwrap_used)] // Fail immediately if the control frame stops decoding.

use super::{FrameCodec, FrameError, MAX_FRAME_BYTES};

const ACCEPTED_RECOVERY_FRAME: &str = concat!(
    r#"{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"#,
    r#""name":"watchdog.operation_lookup","arguments":{"#,
    r#""operation":"op-1","instance":"instance-1"}}}"#,
);

#[test]
fn a_frame_that_repeats_a_top_level_member_is_refused() {
    let frame = concat!(
        r#"{"jsonrpc":"2.0","id":7,"id":8,"method":"tools/call","#,
        r#""params":{"name":"watchdog.operation_lookup","arguments":{}}}"#,
    );
    assert_eq!(
        FrameCodec::decode(frame, MAX_FRAME_BYTES),
        Err(FrameError::InvalidJson)
    );
}

#[test]
fn a_recovery_frame_that_repeats_an_envelope_member_is_refused() {
    // The sideband carries its envelope in `params.arguments`; a second
    // `arguments` member must not be folded into the first one.
    let frame = concat!(
        r#"{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"#,
        r#""name":"watchdog.operation_lookup","#,
        r#""arguments":{"operation":"op-1","instance":"instance-1"},"#,
        r#""arguments":{"operation":"op-2","instance":"instance-2"}}}"#,
    );
    assert_eq!(
        FrameCodec::decode(frame, MAX_FRAME_BYTES),
        Err(FrameError::InvalidJson)
    );
}

#[test]
fn the_same_recovery_frame_with_one_envelope_member_is_accepted() {
    // Control for the refusal above: the only difference is the repetition.
    let request = FrameCodec::decode(ACCEPTED_RECOVERY_FRAME, MAX_FRAME_BYTES)
        .unwrap()
        .unwrap();
    assert_eq!(request.method, "tools/call");
}
