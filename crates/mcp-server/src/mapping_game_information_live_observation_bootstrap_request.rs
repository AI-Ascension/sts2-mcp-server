// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use crate::gateway::GatewayAdapter;
use crate::json::JsonValue;
use crate::protocol::RequestId;
use crate::server::McpServer;
use crate::{
    LIVE_BOOTSTRAP_ARTIFACT, LIVE_BOOTSTRAP_GENERATOR, LIVE_BOOTSTRAP_PROTOCOL_VERSION,
    LIVE_BOOTSTRAP_SCHEMA_DIGEST, LIVE_BOOTSTRAP_SCHEMA_SOURCE,
};

use super::shapes;

const MAX_INTEGER: i64 = 9_007_199_254_740_991;
use shapes::{bounded_int, definition_ref, identity, instance_ref_value, limits, locale};

pub(super) struct RequestContext {
    pub(super) instance_id: String,
    pub(super) mcp_session_id: String,
    pub(super) gateway_session_id: String,
    pub(super) lease_id: String,
    pub(super) lease_epoch: i64,
    pub(super) run_id: String,
    pub(super) authority_epoch: i64,
    pub(super) content_manifest_id: String,
    pub(super) locale: String,
    pub(super) definition_ref: JsonValue,
    pub(super) instance_ref: JsonValue,
    pub(super) max_visible_entities: i64,
    pub(super) max_item_bytes: i64,
    pub(super) max_message_bytes: usize,
    pub(super) correlation_id: String,
}

impl RequestContext {
    pub(super) fn read<G: GatewayAdapter>(
        server: &McpServer<G>,
        arguments: &BTreeMap<String, JsonValue>,
        correlation_id: String,
        _request_id: RequestId,
    ) -> Result<Self, &'static str> {
        let instance_id = identity(arguments, "instance_id", true)?;
        let mcp_session_id = identity(arguments, "mcp_session_id", false)?;
        if server
            .mcp_session_id()
            .is_some_and(|expected| expected != mcp_session_id)
        {
            return Err("MCP session identity does not match the configured session");
        }
        let gateway_session_id = server
            .gateway_session_id()
            .map_or_else(|| mcp_session_id.clone(), str::to_owned);
        let lease_id = identity(arguments, "lease_id", false)?;
        let lease_epoch = bounded_int(arguments, "lease_epoch", 0, MAX_INTEGER)?;
        let run_id = identity(arguments, "run_id", false)?;
        let authority_epoch = bounded_int(arguments, "authority_epoch", 1, MAX_INTEGER)?;
        let content_manifest_id = identity(arguments, "content_manifest_id", false)?;
        let locale = locale(arguments, "locale")?;
        let definition_ref = definition_ref(arguments.get("definition_ref"))?;
        if definition_ref
            .as_object()
            .and_then(|object| object.get("content_manifest_id"))
            .and_then(JsonValue::as_string)
            != Some(content_manifest_id.as_str())
        {
            return Err("definition_ref does not match content_manifest_id");
        }
        let instance_ref = match arguments.get("instance_ref") {
            Some(JsonValue::Null) => JsonValue::Null,
            Some(value) => instance_ref_value(value)?,
            None => return Err("instance_ref is required and may be null"),
        };
        if let JsonValue::Object(ref_value) = &instance_ref
            && (ref_value.get("instance_id").and_then(JsonValue::as_string) != Some(&instance_id)
                || ref_value.get("run_id").and_then(JsonValue::as_string) != Some(&run_id))
        {
            return Err("instance_ref does not match the requested scope");
        }
        let max_visible_entities = bounded_int(arguments, "max_visible_entities", 1, 64)?;
        let max_item_bytes = bounded_int(arguments, "max_item_bytes", 1, 65_536)?;
        let max_message = bounded_int(arguments, "max_message_bytes", 1, 262_144)?;
        Ok(Self {
            instance_id,
            mcp_session_id,
            gateway_session_id,
            lease_id,
            lease_epoch,
            run_id,
            authority_epoch,
            content_manifest_id,
            locale,
            definition_ref,
            instance_ref,
            max_visible_entities,
            max_item_bytes,
            max_message_bytes: usize::try_from(max_message)
                .map_err(|_| "max_message_bytes is outside the protocol bound")?,
            correlation_id,
        })
    }

    pub(super) fn headers(&self) -> BTreeMap<String, String> {
        let mut headers = crate::mapping::headers(&self.mcp_session_id, &self.correlation_id);
        headers.extend([
            ("x-sts2-instance-id".to_owned(), self.instance_id.clone()),
            (
                "x-sts2-session-id".to_owned(),
                self.gateway_session_id.clone(),
            ),
            ("x-sts2-lease-id".to_owned(), self.lease_id.clone()),
            (
                "x-sts2-lease-epoch".to_owned(),
                self.lease_epoch.to_string(),
            ),
        ]);
        headers
    }

    pub(super) fn to_request(&self) -> JsonValue {
        envelope(
            &self.correlation_id,
            "bootstrap_request",
            JsonValue::object([
                (
                    "selector".to_owned(),
                    JsonValue::object([
                        ("definition_ref".to_owned(), self.definition_ref.clone()),
                        ("instance_ref".to_owned(), self.instance_ref.clone()),
                    ]),
                ),
                (
                    "scope".to_owned(),
                    JsonValue::object([
                        (
                            "instance_id".to_owned(),
                            JsonValue::string(self.instance_id.clone()),
                        ),
                        ("run_id".to_owned(), JsonValue::string(self.run_id.clone())),
                        (
                            "authority_epoch".to_owned(),
                            JsonValue::Number(self.authority_epoch),
                        ),
                        (
                            "content_manifest_id".to_owned(),
                            JsonValue::string(self.content_manifest_id.clone()),
                        ),
                        ("locale".to_owned(), JsonValue::string(self.locale.clone())),
                    ]),
                ),
                (
                    "limits".to_owned(),
                    limits(
                        self.max_visible_entities,
                        self.max_item_bytes,
                        self.max_message_bytes as i64,
                    ),
                ),
            ]),
        )
    }
}

fn envelope(correlation: &str, kind: &str, values: JsonValue) -> JsonValue {
    let Some(fields) = values.as_object() else {
        return JsonValue::Null;
    };
    JsonValue::object([
        (
            "protocol_version".to_owned(),
            JsonValue::string(LIVE_BOOTSTRAP_PROTOCOL_VERSION),
        ),
        (
            "schema_digest".to_owned(),
            JsonValue::string(LIVE_BOOTSTRAP_SCHEMA_DIGEST),
        ),
        (
            "provenance".to_owned(),
            JsonValue::object([
                (
                    "artifact".to_owned(),
                    JsonValue::string(LIVE_BOOTSTRAP_ARTIFACT),
                ),
                (
                    "source".to_owned(),
                    JsonValue::string(LIVE_BOOTSTRAP_SCHEMA_SOURCE),
                ),
                (
                    "generator".to_owned(),
                    JsonValue::string(LIVE_BOOTSTRAP_GENERATOR),
                ),
            ]),
        ),
        (
            "correlation_id".to_owned(),
            JsonValue::string(correlation.to_owned()),
        ),
        ("kind".to_owned(), JsonValue::string(kind)),
        ("selector".to_owned(), fields["selector"].clone()),
        ("scope".to_owned(), fields["scope"].clone()),
        ("limits".to_owned(), fields["limits"].clone()),
        ("parent_observation".to_owned(), JsonValue::Null),
        ("visible_entities".to_owned(), JsonValue::Null),
        ("owner_provenance".to_owned(), JsonValue::Null),
        ("error".to_owned(), JsonValue::Null),
    ])
}
