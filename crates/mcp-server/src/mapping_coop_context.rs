// SPDX-License-Identifier: MIT

use crate::json::JsonValue;
use crate::mapping::{has_only_arguments, headers, safe_header_value, safe_segment};
use std::collections::BTreeMap;

pub(super) struct Context {
    pub(super) correlation: String,
    pub(super) instance: String,
    pub(super) session: String,
    pub(super) mcp_session: String,
    pub(super) lease: String,
    pub(super) epoch: i64,
}

impl Context {
    pub(super) fn read(
        arguments: &BTreeMap<String, JsonValue>,
        gateway_session: Option<&str>,
        mcp_session: Option<&str>,
        correlation: &str,
    ) -> Result<Self, &'static str> {
        if !has_only_arguments(
            arguments,
            &["instance_id", "mcp_session_id", "lease_id", "lease_epoch"],
        ) {
            return Err("co-op arguments contain an unsupported field");
        }
        let instance = identity(arguments, "instance_id")?;
        let supplied_session = identity(arguments, "mcp_session_id")?;
        let lease = identity(arguments, "lease_id")?;
        let Some(JsonValue::Number(epoch)) = arguments.get("lease_epoch") else {
            return Err("lease_epoch must be an integer");
        };
        let session = gateway_session.unwrap_or(supplied_session);
        if !(0..=9_007_199_254_740_991).contains(epoch)
            || !safe_segment(instance)
            || !safe_header_value(session)
            || !safe_header_value(correlation)
            || mcp_session.is_some_and(|expected| expected != supplied_session)
        {
            return Err("co-op identity, MCP session, or epoch is invalid");
        }
        Ok(Self {
            correlation: correlation.to_owned(),
            instance: instance.to_owned(),
            session: session.to_owned(),
            mcp_session: supplied_session.to_owned(),
            lease: lease.to_owned(),
            epoch: *epoch,
        })
    }

    pub(super) fn headers(&self) -> BTreeMap<String, String> {
        let mut result = headers(&self.mcp_session, &self.correlation);
        for (name, value) in [
            ("x-sts2-instance-id", self.instance.clone()),
            ("x-sts2-session-id", self.session.clone()),
            ("x-sts2-lease-id", self.lease.clone()),
            ("x-sts2-lease-epoch", self.epoch.to_string()),
        ] {
            result.insert(name.to_owned(), value);
        }
        result
    }
}

fn identity<'a>(
    arguments: &'a BTreeMap<String, JsonValue>,
    name: &str,
) -> Result<&'a str, &'static str> {
    arguments
        .get(name)
        .and_then(JsonValue::as_string)
        .filter(|value| safe_header_value(value))
        .ok_or("co-op identity is missing, unsafe, or oversized")
}
