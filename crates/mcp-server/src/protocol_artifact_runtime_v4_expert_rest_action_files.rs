// SPDX-License-Identifier: MIT

use super::{CONFORMANCE, MANIFEST, SCHEMA, SOURCE_SCHEMA};

pub(super) struct ArtifactFile {
    pub(super) path: &'static str,
    pub(super) bytes: &'static [u8],
}

pub(super) const ARTIFACT_FILES: &[ArtifactFile] = &[
    ArtifactFile {
        path: "../../conformance/cases/runtime-v4-expert-rest-action-v1.json",
        bytes: CONFORMANCE,
    },
    ArtifactFile {
        path: "../../schemas/runtime-v4-expert-rest-action-v1.schema.json",
        bytes: SOURCE_SCHEMA,
    },
    ArtifactFile {
        path: "schema.json",
        bytes: SCHEMA,
    },
    ArtifactFile {
        path: "manifest.json",
        bytes: MANIFEST,
    },
    ArtifactFile {
        path: "golden/action-accepted.json",
        bytes: include_bytes!(
            "../../../protocol-artifact/runtime-v4-expert-rest-action/golden/action-accepted.json"
        ),
    },
    ArtifactFile {
        path: "golden/action-completed.json",
        bytes: include_bytes!(
            "../../../protocol-artifact/runtime-v4-expert-rest-action/golden/action-completed.json"
        ),
    },
    ArtifactFile {
        path: "golden/action-mend-selection-completed.json",
        bytes: include_bytes!(
            "../../../protocol-artifact/runtime-v4-expert-rest-action/golden/action-mend-selection-completed.json"
        ),
    },
    ArtifactFile {
        path: "golden/action-mend-selection-requested.json",
        bytes: include_bytes!(
            "../../../protocol-artifact/runtime-v4-expert-rest-action/golden/action-mend-selection-requested.json"
        ),
    },
    ArtifactFile {
        path: "golden/action-rejected.json",
        bytes: include_bytes!(
            "../../../protocol-artifact/runtime-v4-expert-rest-action/golden/action-rejected.json"
        ),
    },
    ArtifactFile {
        path: "golden/action-request.json",
        bytes: include_bytes!(
            "../../../protocol-artifact/runtime-v4-expert-rest-action/golden/action-request.json"
        ),
    },
    ArtifactFile {
        path: "golden/action-selection-completed.json",
        bytes: include_bytes!(
            "../../../protocol-artifact/runtime-v4-expert-rest-action/golden/action-selection-completed.json"
        ),
    },
    ArtifactFile {
        path: "golden/action-selection-confirm-request.json",
        bytes: include_bytes!(
            "../../../protocol-artifact/runtime-v4-expert-rest-action/golden/action-selection-confirm-request.json"
        ),
    },
    ArtifactFile {
        path: "golden/action-selection-early-confirm-rejected.json",
        bytes: include_bytes!(
            "../../../protocol-artifact/runtime-v4-expert-rest-action/golden/action-selection-early-confirm-rejected.json"
        ),
    },
    ArtifactFile {
        path: "golden/action-selection-first-request.json",
        bytes: include_bytes!(
            "../../../protocol-artifact/runtime-v4-expert-rest-action/golden/action-selection-first-request.json"
        ),
    },
    ArtifactFile {
        path: "golden/action-selection-option-request.json",
        bytes: include_bytes!(
            "../../../protocol-artifact/runtime-v4-expert-rest-action/golden/action-selection-option-request.json"
        ),
    },
    ArtifactFile {
        path: "golden/action-selection-progressed.json",
        bytes: include_bytes!(
            "../../../protocol-artifact/runtime-v4-expert-rest-action/golden/action-selection-progressed.json"
        ),
    },
    ArtifactFile {
        path: "golden/action-selection-requested.json",
        bytes: include_bytes!(
            "../../../protocol-artifact/runtime-v4-expert-rest-action/golden/action-selection-requested.json"
        ),
    },
    ArtifactFile {
        path: "golden/action-selection-second-progressed.json",
        bytes: include_bytes!(
            "../../../protocol-artifact/runtime-v4-expert-rest-action/golden/action-selection-second-progressed.json"
        ),
    },
    ArtifactFile {
        path: "golden/action-selection-second-request.json",
        bytes: include_bytes!(
            "../../../protocol-artifact/runtime-v4-expert-rest-action/golden/action-selection-second-request.json"
        ),
    },
    ArtifactFile {
        path: "golden/action-unknown.json",
        bytes: include_bytes!(
            "../../../protocol-artifact/runtime-v4-expert-rest-action/golden/action-unknown.json"
        ),
    },
    ArtifactFile {
        path: "../../conformance/mutations/runtime-v4-expert-rest-action-v1/action-accepted-empty-action.json",
        bytes: include_bytes!(
            "../../../conformance/mutations/runtime-v4-expert-rest-action-v1/action-accepted-empty-action.json"
        ),
    },
    ArtifactFile {
        path: "../../conformance/mutations/runtime-v4-expert-rest-action-v1/action-accepted-unknown-action-kind.json",
        bytes: include_bytes!(
            "../../../conformance/mutations/runtime-v4-expert-rest-action-v1/action-accepted-unknown-action-kind.json"
        ),
    },
    ArtifactFile {
        path: "../../conformance/mutations/runtime-v4-expert-rest-action-v1/action-accepted-unrelated-option-action.json",
        bytes: include_bytes!(
            "../../../conformance/mutations/runtime-v4-expert-rest-action-v1/action-accepted-unrelated-option-action.json"
        ),
    },
    ArtifactFile {
        path: "../../conformance/mutations/runtime-v4-expert-rest-action-v1/action-completed-evidence-hp-change.json",
        bytes: include_bytes!(
            "../../../conformance/mutations/runtime-v4-expert-rest-action-v1/action-completed-evidence-hp-change.json"
        ),
    },
    ArtifactFile {
        path: "../../conformance/mutations/runtime-v4-expert-rest-action-v1/action-completed-evidence-native-completion.json",
        bytes: include_bytes!(
            "../../../conformance/mutations/runtime-v4-expert-rest-action-v1/action-completed-evidence-native-completion.json"
        ),
    },
    ArtifactFile {
        path: "../../conformance/mutations/runtime-v4-expert-rest-action-v1/action-completed-heal-noop.json",
        bytes: include_bytes!(
            "../../../conformance/mutations/runtime-v4-expert-rest-action-v1/action-completed-heal-noop.json"
        ),
    },
    ArtifactFile {
        path: "../../conformance/mutations/runtime-v4-expert-rest-action-v1/action-completed-unrelated-option-action.json",
        bytes: include_bytes!(
            "../../../conformance/mutations/runtime-v4-expert-rest-action-v1/action-completed-unrelated-option-action.json"
        ),
    },
    ArtifactFile {
        path: "../../conformance/mutations/runtime-v4-expert-rest-action-v1/action-completed-untested-option-witness.json",
        bytes: include_bytes!(
            "../../../conformance/mutations/runtime-v4-expert-rest-action-v1/action-completed-untested-option-witness.json"
        ),
    },
    ArtifactFile {
        path: "../../conformance/mutations/runtime-v4-expert-rest-action-v1/action-mend-selection-completed-absent-player.json",
        bytes: include_bytes!(
            "../../../conformance/mutations/runtime-v4-expert-rest-action-v1/action-mend-selection-completed-absent-player.json"
        ),
    },
    ArtifactFile {
        path: "../../conformance/mutations/runtime-v4-expert-rest-action-v1/action-mend-selection-completed-evidence-hp-change.json",
        bytes: include_bytes!(
            "../../../conformance/mutations/runtime-v4-expert-rest-action-v1/action-mend-selection-completed-evidence-hp-change.json"
        ),
    },
    ArtifactFile {
        path: "../../conformance/mutations/runtime-v4-expert-rest-action-v1/action-mend-selection-completed-evidence-native-completion.json",
        bytes: include_bytes!(
            "../../../conformance/mutations/runtime-v4-expert-rest-action-v1/action-mend-selection-completed-evidence-native-completion.json"
        ),
    },
    ArtifactFile {
        path: "../../conformance/mutations/runtime-v4-expert-rest-action-v1/action-selection-completed-bogus.json",
        bytes: include_bytes!(
            "../../../conformance/mutations/runtime-v4-expert-rest-action-v1/action-selection-completed-bogus.json"
        ),
    },
    ArtifactFile {
        path: "../../conformance/mutations/runtime-v4-expert-rest-action-v1/action-selection-completed-duplicates.json",
        bytes: include_bytes!(
            "../../../conformance/mutations/runtime-v4-expert-rest-action-v1/action-selection-completed-duplicates.json"
        ),
    },
    ArtifactFile {
        path: "../../conformance/mutations/runtime-v4-expert-rest-action-v1/action-selection-completed-required-count-1.json",
        bytes: include_bytes!(
            "../../../conformance/mutations/runtime-v4-expert-rest-action-v1/action-selection-completed-required-count-1.json"
        ),
    },
    ArtifactFile {
        path: "../../conformance/mutations/runtime-v4-expert-rest-action-v1/action-selection-completed-selection-kind-player.json",
        bytes: include_bytes!(
            "../../../conformance/mutations/runtime-v4-expert-rest-action-v1/action-selection-completed-selection-kind-player.json"
        ),
    },
    ArtifactFile {
        path: "../../conformance/mutations/runtime-v4-expert-rest-action-v1/action-selection-completed-unrelated-selection-action.json",
        bytes: include_bytes!(
            "../../../conformance/mutations/runtime-v4-expert-rest-action-v1/action-selection-completed-unrelated-selection-action.json"
        ),
    },
    ArtifactFile {
        path: "../../conformance/mutations/runtime-v4-expert-rest-action-v1/action-selection-progressed-remaining-count-99.json",
        bytes: include_bytes!(
            "../../../conformance/mutations/runtime-v4-expert-rest-action-v1/action-selection-progressed-remaining-count-99.json"
        ),
    },
    ArtifactFile {
        path: "../../conformance/mutations/runtime-v4-expert-rest-action-v1/action-selection-progressed-unlisted-choice.json",
        bytes: include_bytes!(
            "../../../conformance/mutations/runtime-v4-expert-rest-action-v1/action-selection-progressed-unlisted-choice.json"
        ),
    },
    ArtifactFile {
        path: "../../conformance/mutations/runtime-v4-expert-rest-action-v1/action-selection-progressed-unrelated-selection-action.json",
        bytes: include_bytes!(
            "../../../conformance/mutations/runtime-v4-expert-rest-action-v1/action-selection-progressed-unrelated-selection-action.json"
        ),
    },
    ArtifactFile {
        path: "../../conformance/mutations/runtime-v4-expert-rest-action-v1/action-selection-requested-unrelated-selection-action.json",
        bytes: include_bytes!(
            "../../../conformance/mutations/runtime-v4-expert-rest-action-v1/action-selection-requested-unrelated-selection-action.json"
        ),
    },
    ArtifactFile {
        path: "../../conformance/mutations/runtime-v4-expert-rest-action-v1/action-selection-requested-no-choice-action.json",
        bytes: include_bytes!(
            "../../../conformance/mutations/runtime-v4-expert-rest-action-v1/action-selection-requested-no-choice-action.json"
        ),
    },
    ArtifactFile {
        path: "../../conformance/mutations/runtime-v4-expert-rest-action-v1/action-selection-requested-option-kind-mismatch.json",
        bytes: include_bytes!(
            "../../../conformance/mutations/runtime-v4-expert-rest-action-v1/action-selection-requested-option-kind-mismatch.json"
        ),
    },
    ArtifactFile {
        path: "producer/mend-selection-lifecycle.json",
        bytes: include_bytes!(
            "../../../protocol-artifact/runtime-v4-expert-rest-action/producer/mend-selection-lifecycle.json"
        ),
    },
    ArtifactFile {
        path: "producer/smith-selection-lifecycle.json",
        bytes: include_bytes!(
            "../../../protocol-artifact/runtime-v4-expert-rest-action/producer/smith-selection-lifecycle.json"
        ),
    },
];
