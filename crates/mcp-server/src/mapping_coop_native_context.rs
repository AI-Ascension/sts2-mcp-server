// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use crate::gateway::{Correlation, GatewayAdapter, GatewayMethod, GatewayRequest};
use crate::json::JsonValue;
use crate::mapping::{has_only_arguments, headers, safe_header_value};
use crate::projection::NativeContext;
use crate::protocol::RequestId;
use crate::protocol_artifact_coop_native::{
    COOP_NATIVE_ARTIFACT, COOP_NATIVE_GENERATOR, COOP_NATIVE_PROTOCOL_VERSION,
    COOP_NATIVE_SCHEMA_DIGEST, COOP_NATIVE_SCHEMA_SOURCE,
};
use crate::server::McpServer;

use super::COMMON_ARGUMENTS;
use super::request::generation;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct Context {
    pub(super) correlation: String,
    pub(super) instance: String,
    pub(super) session: String,
    pub(super) mcp_session: String,
    pub(super) lease: String,
    pub(super) epoch: i64,
    pub(super) bound_peer: Option<String>,
}

impl Context {
    pub(super) fn read<G: GatewayAdapter>(
        server: &McpServer<G>,
        arguments: &BTreeMap<String, JsonValue>,
        correlation: &str,
        extra: &[&str],
    ) -> Result<Self, &'static str> {
        let mut allowed = COMMON_ARGUMENTS.to_vec();
        allowed.extend_from_slice(extra);
        if !has_only_arguments(arguments, &allowed) {
            return Err("native co-op arguments contain an unsupported field");
        }
        let instance = super::request::identity(arguments, "instance_id", true)?;
        let supplied_session = super::request::identity(arguments, "mcp_session_id", false)?;
        let lease = super::request::identity(arguments, "lease_id", false)?;
        let epoch = generation(arguments, "lease_epoch")?;
        let session = server.gateway_session_id().unwrap_or(supplied_session);
        if server
            .mcp_session_id()
            .is_some_and(|expected| expected != supplied_session)
            || !safe_header_value(session)
            || !safe_header_value(correlation)
        {
            return Err(
                "native co-op identity or MCP session does not match the configured session",
            );
        }
        Ok(Self {
            correlation: correlation.to_owned(),
            instance: instance.to_owned(),
            session: session.to_owned(),
            mcp_session: supplied_session.to_owned(),
            lease: lease.to_owned(),
            epoch,
            bound_peer: server.native_peer_id().map(str::to_owned),
        })
    }

    pub(super) fn projection_context(&self) -> NativeContext {
        NativeContext {
            correlation: self.correlation.clone(),
            instance: self.instance.clone(),
            session: self.session.clone(),
            lease: self.lease.clone(),
            epoch: self.epoch,
            bound_peer: self.bound_peer.clone(),
        }
    }

    fn headers(&self) -> BTreeMap<String, String> {
        let mut result = headers(&self.mcp_session, &self.correlation);
        result.extend([
            (String::from("x-sts2-instance-id"), self.instance.clone()),
            (String::from("x-sts2-session-id"), self.session.clone()),
            (String::from("x-sts2-lease-id"), self.lease.clone()),
            (String::from("x-sts2-lease-epoch"), self.epoch.to_string()),
        ]);
        result
    }

    pub(super) fn gateway_request(
        &self,
        method: GatewayMethod,
        route: &str,
        body: Option<JsonValue>,
        id: RequestId,
    ) -> GatewayRequest {
        GatewayRequest {
            method,
            path: format!("/v1/instances/{}/coop/native/{route}", self.instance),
            headers: self.headers(),
            body,
            correlation: Correlation {
                mcp_session_id: self.mcp_session.clone(),
                mcp_request_id: id,
            },
        }
    }

    pub(super) fn legal_catalog_request(
        &self,
        actor_peer: &str,
        expected_generation: i64,
        id: RequestId,
    ) -> GatewayRequest {
        let body = envelope(
            self,
            "legal_catalog_request",
            None,
            Some(actor_peer),
            Some(expected_generation),
            EnvelopePayload {
                action: None,
                vote: None,
                recovery: None,
            },
        );
        self.gateway_request(GatewayMethod::Post, "legal-catalog", Some(body), id)
    }
}

pub(super) struct EnvelopePayload {
    pub(super) action: Option<JsonValue>,
    pub(super) vote: Option<JsonValue>,
    pub(super) recovery: Option<JsonValue>,
}

pub(super) fn envelope(
    context: &Context,
    kind: &str,
    operation_id: Option<&str>,
    actor_peer: Option<&str>,
    expected_generation: Option<i64>,
    payload: EnvelopePayload,
) -> JsonValue {
    JsonValue::object([
        (
            String::from("protocol_version"),
            JsonValue::string(COOP_NATIVE_PROTOCOL_VERSION),
        ),
        (
            String::from("schema_digest"),
            JsonValue::string(COOP_NATIVE_SCHEMA_DIGEST),
        ),
        (
            String::from("provenance"),
            JsonValue::object([
                (
                    String::from("artifact"),
                    JsonValue::string(COOP_NATIVE_ARTIFACT),
                ),
                (
                    String::from("source"),
                    JsonValue::string(COOP_NATIVE_SCHEMA_SOURCE),
                ),
                (
                    String::from("generator"),
                    JsonValue::string(COOP_NATIVE_GENERATOR),
                ),
            ]),
        ),
        (
            String::from("correlation_id"),
            JsonValue::string(context.correlation.as_str()),
        ),
        (
            String::from("instance_id"),
            JsonValue::string(context.instance.as_str()),
        ),
        (
            String::from("session_id"),
            JsonValue::string(context.session.as_str()),
        ),
        (
            String::from("lease_id"),
            JsonValue::string(context.lease.as_str()),
        ),
        (
            String::from("lease_epoch"),
            JsonValue::Number(context.epoch),
        ),
        (String::from("kind"), JsonValue::string(kind)),
        (
            String::from("operation_id"),
            operation_id.map_or(JsonValue::Null, JsonValue::string),
        ),
        (
            String::from("actor_peer"),
            actor_peer.map_or(JsonValue::Null, JsonValue::string),
        ),
        (
            String::from("expected_host_generation"),
            expected_generation.map_or(JsonValue::Null, JsonValue::Number),
        ),
        (
            String::from("action"),
            payload.action.unwrap_or(JsonValue::Null),
        ),
        (
            String::from("vote"),
            payload.vote.unwrap_or(JsonValue::Null),
        ),
        (String::from("status"), JsonValue::Null),
        (String::from("observation"), JsonValue::Null),
        (String::from("effect"), JsonValue::Null),
        (
            String::from("recovery"),
            payload.recovery.unwrap_or(JsonValue::Null),
        ),
        (String::from("catalog"), JsonValue::Null),
        (String::from("receipt"), JsonValue::Null),
    ])
}
