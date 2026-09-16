// SPDX-License-Identifier: MIT

use crate::catalog::{CapabilityCatalog, ToolDescriptor};

pub(super) const REVISION: &str = "exact-restore-v1-mcp";
pub(super) const BEGIN_TOOL: &str = "sts2.exact_restore.begin";
pub(super) const PUT_CHUNK_TOOL: &str = "sts2.exact_restore.put_chunk";
pub(super) const FINISH_BLOB_TOOL: &str = "sts2.exact_restore.finish_blob";
pub(super) const COMMIT_TOOL: &str = "sts2.exact_restore.commit";
pub(super) const LOOKUP_TOOL: &str = "sts2.exact_restore.lookup";

pub(super) fn build() -> super::ToolCatalog {
    let input_schema = crate::exact_restore::exact_restore_request_input_schema();
    super::ToolCatalog {
        revision: String::from(REVISION),
        capabilities: CapabilityCatalog::default(),
        tools: [
            (
                BEGIN_TOOL,
                "Stage a verified exact-state closure. A known unsupported restore adapter is returned before any artifact upload.",
            ),
            (
                PUT_CHUNK_TOOL,
                "Transfer one bounded, digest-checked chunk for an already staged exact-state artifact.",
            ),
            (
                FINISH_BLOB_TOOL,
                "Verify one complete staged artifact blob; this does not invoke the host restore effect.",
            ),
            (
                COMMIT_TOOL,
                "Request the single exact-state restore effect after closure verification. An uncertain result must be reconciled with lookup; this tool never retries automatically.",
            ),
            (
                LOOKUP_TOOL,
                "Read bounded exact-restore operation state or one artifact's transfer progress without applying a host effect.",
            ),
        ]
        .into_iter()
        .map(|(name, description)| ToolDescriptor {
            name: String::from(name),
            description: String::from(description),
            input_schema: input_schema.clone(),
        })
        .collect(),
        composition: None,
    }
}
