// SPDX-License-Identifier: MIT

use crate::json::JsonValue;

use super::{
    COOP_RECEIPT_QUERY_ARTIFACT, COOP_RECEIPT_QUERY_GENERATOR, COOP_RECEIPT_QUERY_PROTOCOL_VERSION,
    COOP_RECEIPT_QUERY_SCHEMA_DIGEST, COOP_RECEIPT_QUERY_SCHEMA_SOURCE, Context,
};

impl Context {
    pub(super) fn to_request(&self) -> JsonValue {
        JsonValue::object([
            (
                String::from("protocol_version"),
                JsonValue::string(COOP_RECEIPT_QUERY_PROTOCOL_VERSION),
            ),
            (
                String::from("schema_digest"),
                JsonValue::string(COOP_RECEIPT_QUERY_SCHEMA_DIGEST),
            ),
            (
                String::from("provenance"),
                JsonValue::object([
                    (
                        String::from("artifact"),
                        JsonValue::string(COOP_RECEIPT_QUERY_ARTIFACT),
                    ),
                    (
                        String::from("source"),
                        JsonValue::string(COOP_RECEIPT_QUERY_SCHEMA_SOURCE),
                    ),
                    (
                        String::from("generator"),
                        JsonValue::string(COOP_RECEIPT_QUERY_GENERATOR),
                    ),
                ]),
            ),
            (
                String::from("correlation_id"),
                JsonValue::string(self.correlation.as_str()),
            ),
            (
                String::from("instance_id"),
                JsonValue::string(self.instance.as_str()),
            ),
            (
                String::from("session_id"),
                JsonValue::string(self.session.as_str()),
            ),
            (
                String::from("lease_id"),
                JsonValue::string(self.lease.as_str()),
            ),
            (String::from("lease_epoch"), JsonValue::Number(self.epoch)),
            (
                String::from("kind"),
                JsonValue::string("receipt_query_request"),
            ),
            (
                String::from("operation_id"),
                JsonValue::string(self.operation_id.as_str()),
            ),
            (
                String::from("action_kind"),
                JsonValue::string(self.action_kind.as_str()),
            ),
            (
                String::from("action_fingerprint"),
                JsonValue::string(self.action_fingerprint.as_str()),
            ),
            (
                String::from("run_id"),
                JsonValue::string(self.run_id.as_str()),
            ),
            (String::from("location"), self.location.clone()),
            (
                String::from("actor_id"),
                JsonValue::string(self.actor_id.as_str()),
            ),
            (
                String::from("authority_id"),
                JsonValue::string(self.authority_id.as_str()),
            ),
            (
                String::from("authority_epoch"),
                JsonValue::string(self.authority_epoch.as_str()),
            ),
            (
                String::from("expected_host_generation"),
                JsonValue::Number(self.expected_host_generation),
            ),
            (
                String::from("before_host_generation"),
                JsonValue::Number(self.before_host_generation),
            ),
            (
                String::from("participant_ids"),
                JsonValue::Array(
                    self.participant_ids
                        .iter()
                        .map(|value| JsonValue::string(value.as_str()))
                        .collect(),
                ),
            ),
            (String::from("status"), JsonValue::Null),
            (String::from("evidence_scope"), JsonValue::Null),
            (String::from("receipt"), JsonValue::Null),
            (String::from("error_code"), JsonValue::Null),
        ])
    }
}
