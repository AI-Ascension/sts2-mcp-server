// SPDX-License-Identifier: MIT

use crate::mapping::safe_segment;

#[test]
fn accepts_only_path_safe_instance_segments() {
    assert!(safe_segment("instance-1_alpha"));
    assert!(!safe_segment("../instance"));
    assert!(!safe_segment("instance/child"));
}
