// SPDX-License-Identifier: MIT

use crate::json::JsonValue;

mod chunk;
mod request;
mod response;
mod schema;

pub use request::{
    ExactRestorePhase, ExactRestoreRequestBinding, ExactRestoreTransportOwner,
    exact_restore_request_input_schema, validate_exact_restore_request,
};
pub use response::{ExactRestoreResponse, validate_exact_restore_response};

pub const EXACT_RESTORE_BEGIN_TOOL: &str = "sts2.exact_restore.begin";
pub const EXACT_RESTORE_PUT_CHUNK_TOOL: &str = "sts2.exact_restore.put_chunk";
pub const EXACT_RESTORE_FINISH_BLOB_TOOL: &str = "sts2.exact_restore.finish_blob";
pub const EXACT_RESTORE_COMMIT_TOOL: &str = "sts2.exact_restore.commit";
pub const EXACT_RESTORE_LOOKUP_TOOL: &str = "sts2.exact_restore.lookup";

pub(crate) fn object_member<'a>(value: &'a JsonValue, key: &str) -> Option<&'a JsonValue> {
    value.as_object()?.get(key)
}

pub(crate) fn string_member<'a>(value: &'a JsonValue, key: &str) -> Option<&'a str> {
    object_member(value, key)?.as_string()
}

#[cfg(test)]
mod tests;
