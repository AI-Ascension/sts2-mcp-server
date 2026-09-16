// SPDX-License-Identifier: MIT

pub(super) struct ArtifactFile {
    pub(super) path: &'static str,
    pub(super) bytes: &'static [u8],
}

pub(super) const NEUTRAL_FILES: &[ArtifactFile] = &[
    ArtifactFile {
        path: "README.md",
        bytes: include_bytes!("../../../protocol-artifact/exact-restore-v1/README.md"),
    },
    ArtifactFile {
        path: "golden/closure-digest.json",
        bytes: include_bytes!(
            "../../../protocol-artifact/exact-restore-v1/golden/closure-digest.json"
        ),
    },
    ArtifactFile {
        path: "golden/frames.json",
        bytes: include_bytes!("../../../protocol-artifact/exact-restore-v1/golden/frames.json"),
    },
    ArtifactFile {
        path: "golden/rejections.json",
        bytes: include_bytes!("../../../protocol-artifact/exact-restore-v1/golden/rejections.json"),
    },
    ArtifactFile {
        path: "manifest.json",
        bytes: include_bytes!("../../../protocol-artifact/exact-restore-v1/manifest.json"),
    },
    ArtifactFile {
        path: "schema.json",
        bytes: include_bytes!("../../../protocol-artifact/exact-restore-v1/schema.json"),
    },
];

pub(super) const GATEWAY_FILES: &[ArtifactFile] = &[
    ArtifactFile {
        path: "README.md",
        bytes: include_bytes!("../../../protocol-artifact/exact-restore-gateway-v1/README.md"),
    },
    ArtifactFile {
        path: "golden/begin-request.json",
        bytes: include_bytes!(
            "../../../protocol-artifact/exact-restore-gateway-v1/golden/begin-request.json"
        ),
    },
    ArtifactFile {
        path: "golden/begin-response.json",
        bytes: include_bytes!(
            "../../../protocol-artifact/exact-restore-gateway-v1/golden/begin-response.json"
        ),
    },
    ArtifactFile {
        path: "golden/commit-unknown-response.json",
        bytes: include_bytes!(
            "../../../protocol-artifact/exact-restore-gateway-v1/golden/commit-unknown-response.json"
        ),
    },
    ArtifactFile {
        path: "golden/rejections.json",
        bytes: include_bytes!(
            "../../../protocol-artifact/exact-restore-gateway-v1/golden/rejections.json"
        ),
    },
    ArtifactFile {
        path: "manifest.json",
        bytes: include_bytes!("../../../protocol-artifact/exact-restore-gateway-v1/manifest.json"),
    },
    ArtifactFile {
        path: "schema.json",
        bytes: include_bytes!("../../../protocol-artifact/exact-restore-gateway-v1/schema.json"),
    },
];
