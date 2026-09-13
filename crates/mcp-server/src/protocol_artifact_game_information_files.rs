// SPDX-License-Identifier: MIT

const CHECKSUMS: &str =
    include_str!("../../../protocol-artifact/game-information-query-v1/SHA256SUMS");

struct ArtifactFile {
    path: &'static str,
    bytes: &'static [u8],
}

const ARTIFACT_FILES: &[ArtifactFile] = &[
    ArtifactFile {
        path: "../../conformance/cases/game-information-query-v1.json",
        bytes: include_bytes!("../../../conformance/cases/game-information-query-v1.json"),
    },
    ArtifactFile {
        path: "../../schemas/game-information-query-v1.schema.json",
        bytes: include_bytes!("../../../schemas/game-information-query-v1.schema.json"),
    },
    ArtifactFile {
        path: "README.md",
        bytes: include_bytes!("../../../protocol-artifact/game-information-query-v1/README.md"),
    },
    ArtifactFile {
        path: "conformance.json",
        bytes: include_bytes!(
            "../../../protocol-artifact/game-information-query-v1/conformance.json"
        ),
    },
    ArtifactFile {
        path: "golden/capabilities-response.json",
        bytes: include_bytes!(
            "../../../protocol-artifact/game-information-query-v1/golden/capabilities-response.json"
        ),
    },
    ArtifactFile {
        path: "golden/error-stale-cursor.json",
        bytes: include_bytes!(
            "../../../protocol-artifact/game-information-query-v1/golden/error-stale-cursor.json"
        ),
    },
    ArtifactFile {
        path: "golden/live-detail-request.json",
        bytes: include_bytes!(
            "../../../protocol-artifact/game-information-query-v1/golden/live-detail-request.json"
        ),
    },
    ArtifactFile {
        path: "golden/live-detail-response.json",
        bytes: include_bytes!(
            "../../../protocol-artifact/game-information-query-v1/golden/live-detail-response.json"
        ),
    },
    ArtifactFile {
        path: "golden/static-page-1-request.json",
        bytes: include_bytes!(
            "../../../protocol-artifact/game-information-query-v1/golden/static-page-1-request.json"
        ),
    },
    ArtifactFile {
        path: "golden/static-page-1-response.json",
        bytes: include_bytes!(
            "../../../protocol-artifact/game-information-query-v1/golden/static-page-1-response.json"
        ),
    },
    ArtifactFile {
        path: "golden/static-page-2-request.json",
        bytes: include_bytes!(
            "../../../protocol-artifact/game-information-query-v1/golden/static-page-2-request.json"
        ),
    },
    ArtifactFile {
        path: "golden/static-page-2-response.json",
        bytes: include_bytes!(
            "../../../protocol-artifact/game-information-query-v1/golden/static-page-2-response.json"
        ),
    },
    ArtifactFile {
        path: "manifest.json",
        bytes: include_bytes!("../../../protocol-artifact/game-information-query-v1/manifest.json"),
    },
    ArtifactFile {
        path: "schema.json",
        bytes: include_bytes!("../../../protocol-artifact/game-information-query-v1/schema.json"),
    },
    ArtifactFile {
        path: "../../conformance/fixtures/game-information-query-v1/invalid/ambiguous-display-name.json",
        bytes: include_bytes!(
            "../../../conformance/fixtures/game-information-query-v1/invalid/ambiguous-display-name.json"
        ),
    },
    ArtifactFile {
        path: "../../conformance/fixtures/game-information-query-v1/invalid/cross-content-cursor.json",
        bytes: include_bytes!(
            "../../../conformance/fixtures/game-information-query-v1/invalid/cross-content-cursor.json"
        ),
    },
    ArtifactFile {
        path: "../../conformance/fixtures/game-information-query-v1/invalid/cross-epoch-cursor.json",
        bytes: include_bytes!(
            "../../../conformance/fixtures/game-information-query-v1/invalid/cross-epoch-cursor.json"
        ),
    },
    ArtifactFile {
        path: "../../conformance/fixtures/game-information-query-v1/invalid/cross-locale-cursor.json",
        bytes: include_bytes!(
            "../../../conformance/fixtures/game-information-query-v1/invalid/cross-locale-cursor.json"
        ),
    },
    ArtifactFile {
        path: "../../conformance/fixtures/game-information-query-v1/invalid/cross-run-cursor.json",
        bytes: include_bytes!(
            "../../../conformance/fixtures/game-information-query-v1/invalid/cross-run-cursor.json"
        ),
    },
    ArtifactFile {
        path: "../../conformance/fixtures/game-information-query-v1/invalid/cross-scope-cursor.json",
        bytes: include_bytes!(
            "../../../conformance/fixtures/game-information-query-v1/invalid/cross-scope-cursor.json"
        ),
    },
    ArtifactFile {
        path: "../../conformance/fixtures/game-information-query-v1/invalid/duplicate-key.json",
        bytes: include_bytes!(
            "../../../conformance/fixtures/game-information-query-v1/invalid/duplicate-key.json"
        ),
    },
    ArtifactFile {
        path: "../../conformance/fixtures/game-information-query-v1/invalid/integer-bounds.json",
        bytes: include_bytes!(
            "../../../conformance/fixtures/game-information-query-v1/invalid/integer-bounds.json"
        ),
    },
    ArtifactFile {
        path: "../../conformance/fixtures/game-information-query-v1/invalid/missing-capability.json",
        bytes: include_bytes!(
            "../../../conformance/fixtures/game-information-query-v1/invalid/missing-capability.json"
        ),
    },
    ArtifactFile {
        path: "../../conformance/fixtures/game-information-query-v1/invalid/missing-vs-empty.json",
        bytes: include_bytes!(
            "../../../conformance/fixtures/game-information-query-v1/invalid/missing-vs-empty.json"
        ),
    },
    ArtifactFile {
        path: "../../conformance/fixtures/game-information-query-v1/invalid/mixed-generation.json",
        bytes: include_bytes!(
            "../../../conformance/fixtures/game-information-query-v1/invalid/mixed-generation.json"
        ),
    },
    ArtifactFile {
        path: "../../conformance/fixtures/game-information-query-v1/invalid/oversized.json",
        bytes: include_bytes!(
            "../../../conformance/fixtures/game-information-query-v1/invalid/oversized.json"
        ),
    },
    ArtifactFile {
        path: "../../conformance/fixtures/game-information-query-v1/invalid/read-only-violation.json",
        bytes: include_bytes!(
            "../../../conformance/fixtures/game-information-query-v1/invalid/read-only-violation.json"
        ),
    },
    ArtifactFile {
        path: "../../conformance/fixtures/game-information-query-v1/invalid/unknown-enum-version.json",
        bytes: include_bytes!(
            "../../../conformance/fixtures/game-information-query-v1/invalid/unknown-enum-version.json"
        ),
    },
    ArtifactFile {
        path: "../../conformance/fixtures/game-information-query-v1/invalid/unsupported-field.json",
        bytes: include_bytes!(
            "../../../conformance/fixtures/game-information-query-v1/invalid/unsupported-field.json"
        ),
    },
    ArtifactFile {
        path: "../../conformance/fixtures/game-information-query-v1/valid/empty-vs-unavailable.json",
        bytes: include_bytes!(
            "../../../conformance/fixtures/game-information-query-v1/valid/empty-vs-unavailable.json"
        ),
    },
    ArtifactFile {
        path: "../../conformance/fixtures/game-information-query-v1/valid/source-provenance.json",
        bytes: include_bytes!(
            "../../../conformance/fixtures/game-information-query-v1/valid/source-provenance.json"
        ),
    },
    ArtifactFile {
        path: "../../conformance/fixtures/game-information-query-v1/valid/utf8-boundary.json",
        bytes: include_bytes!(
            "../../../conformance/fixtures/game-information-query-v1/valid/utf8-boundary.json"
        ),
    },
    ArtifactFile {
        path: "../../conformance/fixtures/game-information-query-v1/valid/zero-vs-missing.json",
        bytes: include_bytes!(
            "../../../conformance/fixtures/game-information-query-v1/valid/zero-vs-missing.json"
        ),
    },
];

pub(super) fn verify() -> Result<(), super::GameInformationArtifactError> {
    let mut verified = Vec::new();
    for line in CHECKSUMS.lines() {
        let (expected, path) = line
            .split_once("  ")
            .ok_or(super::GameInformationArtifactError::ChecksumMismatch)?;
        if expected.len() != 64
            || !expected
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            || verified.contains(&path)
        {
            return Err(super::GameInformationArtifactError::ChecksumMismatch);
        }
        let file = ARTIFACT_FILES
            .iter()
            .find(|file| file.path == path)
            .ok_or(super::GameInformationArtifactError::ChecksumMismatch)?;
        if crate::protocol_artifact_hash::sha256_hex(file.bytes) != expected {
            return Err(super::GameInformationArtifactError::ChecksumMismatch);
        }
        verified.push(path);
    }
    if verified.len() != ARTIFACT_FILES.len()
        || ARTIFACT_FILES
            .iter()
            .any(|file| !verified.contains(&file.path))
    {
        return Err(super::GameInformationArtifactError::ChecksumMismatch);
    }
    Ok(())
}
