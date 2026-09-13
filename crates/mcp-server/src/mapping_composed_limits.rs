// SPDX-License-Identifier: MIT

use crate::catalog::ToolLimits;
use crate::json::JsonValue;
use crate::protocol::{RequestId, RpcResponse};

use super::helpers::tool_error_result;

pub(super) fn enforce_composed_response(
    id: RequestId,
    response: RpcResponse,
    limits: ToolLimits,
) -> RpcResponse {
    if let Some(content_bytes) = response_content_bytes(&response)
        && content_bytes > limits.max_content_bytes
    {
        return limit_error(
            id.clone(),
            "negotiated_content_limit_exceeded",
            "response exceeds the negotiated content byte limit",
            limits.max_content_bytes,
        );
    }
    if response_page_items(&response) > limits.max_page_items {
        return limit_error(
            id,
            "negotiated_pagination_limit_exceeded",
            "response exceeds the negotiated page-item limit",
            limits.max_content_bytes,
        );
    }
    response
}

pub(super) fn contains_page_items_over(value: &JsonValue, maximum: usize) -> bool {
    match value {
        JsonValue::Object(object) => object.iter().any(|(key, child)| {
            (key == "page_items"
                && matches!(child, JsonValue::Number(value)
                    if *value >= 0 && usize::try_from(*value).is_ok_and(|value| value > maximum)))
                || contains_page_items_over(child, maximum)
        }),
        JsonValue::Array(values) => values
            .iter()
            .any(|value| contains_page_items_over(value, maximum)),
        _ => false,
    }
}

pub(super) fn limit_error(
    id: RequestId,
    code: &'static str,
    message: &'static str,
    max_content_bytes: usize,
) -> RpcResponse {
    tool_error_result(
        id,
        code,
        "size",
        bounded_message(message, max_content_bytes),
    )
}

fn bounded_message(message: &str, maximum: usize) -> String {
    if message.len() <= maximum {
        return message.to_owned();
    }
    let mut end = 0;
    for (index, character) in message.char_indices() {
        let next = index + character.len_utf8();
        if next > maximum {
            break;
        }
        end = next;
    }
    message[..end].to_owned()
}

fn response_content_bytes(response: &RpcResponse) -> Option<usize> {
    let content = response.result()?.as_object()?.get("content")?.as_array()?;
    Some(
        content
            .iter()
            .filter_map(JsonValue::as_object)
            .filter_map(|item| item.get("text").and_then(JsonValue::as_string))
            .map(str::len)
            .sum(),
    )
}

fn response_page_items(response: &RpcResponse) -> usize {
    response
        .result()
        .map_or(0, |result| page_items_in_value(result, true))
}

fn page_items_in_value(value: &JsonValue, parse_text: bool) -> usize {
    match value {
        JsonValue::Object(object) => {
            let own = object
                .get("items")
                .and_then(JsonValue::as_array)
                .map_or(0, Vec::len);
            let nested = object
                .values()
                .map(|child| page_items_in_value(child, parse_text))
                .max()
                .unwrap_or(0);
            let parsed = if parse_text {
                object
                    .get("text")
                    .and_then(JsonValue::as_string)
                    .and_then(|text| crate::json::parse(text).ok())
                    .map_or(0, |value| page_items_in_value(&value, false))
            } else {
                0
            };
            own.max(nested).max(parsed)
        }
        JsonValue::Array(values) => values
            .iter()
            .map(|value| page_items_in_value(value, parse_text))
            .max()
            .unwrap_or(0),
        _ => 0,
    }
}
