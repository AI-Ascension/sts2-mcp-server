// SPDX-License-Identifier: MIT
//! Owned synthetic game-information producer for the scoped issue #51
//! acceptance sequence.
//!
//! The producer is a second, independent implementation of the pinned
//! `game-information-query-v1` *downstream* contract. It owns a synthetic
//! content manifest, opaque cursors and one live snapshot, validates every
//! mapped gateway request against the fixed route mapping, and generates
//! conforming or deliberately non-conforming responses. It is deterministic
//! synthetic test code, not a game host, provider, or evidence of native
//! behavior, and it never touches a network, profile or save.

use serde_json::Value;
use sts2_mcp_server::{
    GAME_INFORMATION_MAX_MESSAGE_BYTES, GAME_INFORMATION_PROTOCOL_VERSION,
    GAME_INFORMATION_SCHEMA_DIGEST, GatewayAdapter, GatewayError, GatewayMethod, GatewayRequest,
    GatewayResponse, JsonValue, parse_json,
};

use super::content::{
    CARD_FIELDS, DEFINITIONS, Definition, FieldPolicy, RELIC_FIELDS, definition_item,
    live_instance_ref, requested_fields,
};
use super::envelope::{
    capabilities_envelope, corrupt_digest, error_envelope, provenance, query_envelope,
};
use super::page::{
    ProducerError, STALE_SNAPSHOT, cursor_for, cursor_page, filter_matches, oversized_page,
    page_value, query_limit, query_result, snapshot_entity_id, snapshot_is_coherent,
};

/// Synthetic identity bound to the fixed gateway route mapping.
pub(crate) const INSTANCE_ID: &str = "instance-1";
pub(crate) const GATEWAY_SESSION_ID: &str = "gateway-session-1";
pub(crate) const MCP_SESSION_ID: &str = "mcp-session-1";
pub(crate) const LEASE_ID: &str = "lease-1";
pub(crate) const LEASE_EPOCH: i64 = 7;
pub(crate) const CONTENT_MANIFEST_ID: &str = "synthetic-content-1";
pub(crate) const RUN_ID: &str = "run-1";
pub(crate) const SNAPSHOT_ID: &str = "snapshot-42";
pub(crate) const SNAPSHOT_GENERATION: i64 = 42;
pub(crate) const LIVE_ENTITY_ID: &str = "card-17";
pub(crate) const CAPABILITIES_PATH: &str = "/v1/instances/instance-1/game-information/capabilities";
pub(crate) const QUERY_PATH: &str = "/v1/instances/instance-1/game-information/query";

const STRIKE: &str = "synthetic:strike";

/// Producer behaviors that keep the acceptance sequence honest: the producer
/// must refuse bounded requests with typed errors, and it must be able to
/// violate its own declared bounds so the MCP boundary is proven to reject that
/// data instead of projecting it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ProducerMode {
    /// Advertised contract only.
    Contract,
    /// Well-formed page whose envelope exceeds the pinned message bound.
    OversizedPage,
    /// Envelope whose pinned identity does not match the MCP request.
    CorruptEnvelope,
}

/// One mapped downstream request observed by the producer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct QueryRecord {
    pub(crate) path: String,
    pub(crate) query_kind: String,
    pub(crate) cursor: Option<String>,
}

pub(crate) struct SyntheticProducer {
    mode: ProducerMode,
    records: Vec<QueryRecord>,
    violations: Vec<String>,
}

impl SyntheticProducer {
    pub(crate) fn contract() -> Self {
        Self::with_mode(ProducerMode::Contract)
    }

    pub(crate) fn with_mode(mode: ProducerMode) -> Self {
        Self {
            mode,
            records: Vec::new(),
            violations: Vec::new(),
        }
    }

    pub(crate) fn records(&self) -> &[QueryRecord] {
        &self.records
    }

    pub(crate) fn violations(&self) -> &[String] {
        &self.violations
    }

    fn respond(&self, value: &Value) -> Result<GatewayResponse, GatewayError> {
        Ok(GatewayResponse {
            status: 200,
            body: to_json_value(value),
        })
    }

    fn check_headers(&mut self, request: &GatewayRequest) {
        for (name, expected) in [
            ("x-sts2-instance-id", INSTANCE_ID),
            ("x-sts2-session-id", GATEWAY_SESSION_ID),
            ("x-sts2-lease-id", LEASE_ID),
            ("x-sts2-lease-epoch", "7"),
            ("x-mcp-session-id", MCP_SESSION_ID),
        ] {
            if request.headers.get(name).map(String::as_str) != Some(expected) {
                self.violations
                    .push(format!("mapped header {name} is missing or wrong"));
            }
        }
        if !request.headers.contains_key("x-mcp-request-id") {
            self.violations
                .push(String::from("mapped request correlation is absent"));
        }
    }

    fn query(&mut self, request: &GatewayRequest) -> Result<GatewayResponse, GatewayError> {
        let Some(body) = request.body.as_ref() else {
            self.violations
                .push(String::from("query route without a body"));
            return Err(GatewayError::Rejected);
        };
        let body = to_serde(body);
        let Some(correlation) = json_text(&body, "correlation_id") else {
            self.violations
                .push(String::from("query envelope has no correlation"));
            return Err(GatewayError::MalformedResponse);
        };
        if request.headers.get("x-mcp-request-id").map(String::as_str) != Some(correlation) {
            self.violations.push(String::from(
                "correlation header does not match the envelope",
            ));
        }
        if !self.envelope_matches(&body) {
            return Err(GatewayError::MalformedResponse);
        }
        let query = body.get("query").cloned().unwrap_or(Value::Null);
        self.records.push(QueryRecord {
            path: request.path.clone(),
            query_kind: json_text(&query, "query_kind")
                .unwrap_or_default()
                .to_owned(),
            cursor: json_text(&query, "cursor").map(str::to_owned),
        });
        let correlation = correlation.to_owned();
        let mut response = match self.handle_query(&query) {
            Ok(result) => query_envelope(&correlation, &query, result),
            Err((code, field, reason, retryable)) => {
                error_envelope(&correlation, code, field, reason, retryable)
            }
        };
        if self.mode == ProducerMode::CorruptEnvelope {
            response = corrupt_digest(response);
        }
        if self.mode == ProducerMode::OversizedPage && is_bounded_page(&query) {
            let len = response.to_string().len();
            assert!(
                len > GAME_INFORMATION_MAX_MESSAGE_BYTES,
                "synthetic oversized envelope is {len} bytes, within the pinned bound"
            );
        }
        self.respond(&response)
    }

    fn envelope_matches(&mut self, body: &Value) -> bool {
        let matches = json_text(body, "protocol_version")
            == Some(GAME_INFORMATION_PROTOCOL_VERSION)
            && json_text(body, "schema_digest") == Some(GAME_INFORMATION_SCHEMA_DIGEST)
            && json_text(body, "kind") == Some("query_request")
            && body.get("provenance") == Some(&provenance());
        if !matches {
            self.violations.push(String::from(
                "query envelope identity is not the pinned contract",
            ));
        }
        matches
    }

    fn handle_query(&self, query: &Value) -> Result<Value, ProducerError> {
        let mode = query
            .get("binding")
            .and_then(|binding| json_text(binding, "mode"))
            .unwrap_or("static");
        let entity_kind = match json_text(query, "entity_kind") {
            Some("card") => "card",
            Some("relic") if mode == "static" => "relic",
            _ => {
                return Err((
                    "missing_capability",
                    Some("entity_kind"),
                    "entity kind or mode is not advertised by this producer",
                    false,
                ));
            }
        };
        if !self.requested_fields_supported(query, entity_kind) {
            return Err((
                "unsupported_field",
                None,
                "requested field is not supported for this entity kind",
                false,
            ));
        }
        match json_text(query, "query_kind") {
            Some("list" | "search") => {
                self.static_result_with(query, entity_kind, FieldPolicy::Available)
            }
            Some("get") => self.get_result(query, entity_kind),
            Some("detail") => self.live_result(query),
            Some("availability") if mode == "static" => {
                self.static_result_with(query, entity_kind, FieldPolicy::AvailabilityProbe)
            }
            Some(_) => Err((
                "missing_capability",
                Some("query_kind"),
                "query kind is not advertised by this producer",
                false,
            )),
            None => Err((
                "malformed",
                Some("query_kind"),
                "query kind is missing",
                false,
            )),
        }
    }

    fn requested_fields_supported(&self, query: &Value, entity_kind: &str) -> bool {
        let supported: &[&str] = if entity_kind == "card" {
            &CARD_FIELDS
        } else {
            &RELIC_FIELDS
        };
        query
            .get("fields")
            .and_then(Value::as_array)
            .is_some_and(|fields| {
                fields
                    .iter()
                    .all(|field| field.as_str().is_some_and(|name| supported.contains(&name)))
            })
    }

    fn static_result_with(
        &self,
        query: &Value,
        entity_kind: &str,
        policy: FieldPolicy,
    ) -> Result<Value, ProducerError> {
        if self.mode == ProducerMode::OversizedPage && policy == FieldPolicy::Available {
            return Ok(oversized_page(query));
        }
        let page_items = query_limit(query, "page_items")?;
        if !(1..=128).contains(&page_items) {
            return Err((
                "invalid_bounds",
                Some("limits"),
                "page item bound is outside the producer bound",
                false,
            ));
        }
        let matches: Vec<&Definition> = DEFINITIONS
            .iter()
            .filter(|definition| definition.kind == entity_kind)
            .filter(|definition| filter_matches(query, definition))
            .collect();
        let total = i64::try_from(matches.len()).unwrap_or(i64::MAX);
        let page = cursor_page(query, entity_kind, total, page_items)?;
        let offset = usize::try_from((page - 1) * page_items).unwrap_or(usize::MAX);
        let start = offset.min(matches.len());
        let length = usize::try_from(page_items).unwrap_or(0);
        let end = (start + length).min(matches.len());
        let fields = requested_fields(query);
        let items: Vec<Value> = matches[start..end]
            .iter()
            .map(|definition| definition_item(definition, &fields, policy, Value::Null))
            .collect();
        let final_page = end >= matches.len();
        let next_cursor = (!final_page).then(|| cursor_for(query, entity_kind, page + 1));
        Ok(query_result(
            query,
            page_value(
                query,
                items,
                "definition_ref",
                next_cursor,
                final_page,
                total,
            ),
            Value::Null,
        ))
    }

    fn get_result(&self, query: &Value, entity_kind: &str) -> Result<Value, ProducerError> {
        let target = query
            .get("target")
            .and_then(|target| target.get("definition_ref"))
            .filter(|target| !target.is_null())
            .ok_or((
                "invalid_identity",
                Some("definition_ref"),
                "get requires one definition reference",
                false,
            ))?;
        let namespaced_id = json_text(target, "namespaced_id").unwrap_or_default();
        let definition = DEFINITIONS
            .iter()
            .find(|definition| {
                definition.kind == entity_kind && definition.namespaced_id == namespaced_id
            })
            .ok_or((
                "unknown_id",
                Some("definition_ref"),
                "definition is not present in the synthetic content manifest",
                false,
            ))?;
        let fields = requested_fields(query);
        let item = definition_item(definition, &fields, FieldPolicy::Available, Value::Null);
        Ok(query_result(
            query,
            page_value(query, vec![item], "definition_ref", None, true, 1),
            Value::Null,
        ))
    }

    fn live_result(&self, query: &Value) -> Result<Value, ProducerError> {
        let binding = query.get("binding").unwrap_or(&Value::Null);
        if !snapshot_is_coherent(binding) {
            return Err(STALE_SNAPSHOT);
        }
        if snapshot_entity_id(binding) != Some(LIVE_ENTITY_ID) {
            return Err((
                "unknown_id",
                Some("instance_ref"),
                "live entity is not present in the pinned snapshot",
                false,
            ));
        }
        let definition = DEFINITIONS
            .iter()
            .find(|definition| definition.namespaced_id == STRIKE)
            .ok_or(STALE_SNAPSHOT)?;
        let fields = requested_fields(query);
        let item = definition_item(definition, &fields, FieldPolicy::Live, live_instance_ref());
        Ok(query_result(
            query,
            page_value(query, vec![item], "instance_ref", None, true, 1),
            Value::from(SNAPSHOT_GENERATION),
        ))
    }
}

impl GatewayAdapter for SyntheticProducer {
    fn forward(&mut self, request: GatewayRequest) -> Result<GatewayResponse, GatewayError> {
        self.check_headers(&request);
        let correlation = request
            .headers
            .get("x-mcp-request-id")
            .cloned()
            .unwrap_or_default();
        match (request.method, request.path.as_str()) {
            (GatewayMethod::Get, CAPABILITIES_PATH) if request.body.is_none() => {
                self.records.push(QueryRecord {
                    path: request.path.clone(),
                    query_kind: String::from("capabilities"),
                    cursor: None,
                });
                self.respond(&capabilities_envelope(&correlation))
            }
            (GatewayMethod::Post, QUERY_PATH) => self.query(&request),
            (method, path) => {
                self.violations
                    .push(format!("unmapped downstream route {method:?} {path}"));
                Err(GatewayError::NotFound)
            }
        }
    }
}

fn is_bounded_page(query: &Value) -> bool {
    matches!(
        json_text(query, "query_kind"),
        Some("list" | "search" | "availability")
    )
}

fn to_json_value(value: &Value) -> JsonValue {
    parse_json(&value.to_string()).expect("synthetic producer response is valid JSON")
}

fn to_serde(value: &JsonValue) -> Value {
    serde_json::from_str(&value.to_json()).expect("mapped request body is valid JSON")
}

fn json_text<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}
