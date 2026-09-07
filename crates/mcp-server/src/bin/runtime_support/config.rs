// SPDX-License-Identifier: MIT

use std::net::SocketAddr;

use sts2_mcp_server::{GatewayRequest, JsonValue, RECOVERY_CONTRACT};

pub(super) fn gateway_address(value: &str) -> Result<SocketAddr, String> {
    let address: SocketAddr = value
        .parse()
        .map_err(|_| String::from("STS2_GATEWAY_ADDR must be a numeric loopback socket address"))?;
    if !address.ip().is_loopback() || address.port() == 0 {
        return Err(String::from(
            "STS2_GATEWAY_ADDR must be loopback with a nonzero port",
        ));
    }
    Ok(address)
}

pub(super) fn safe_token(value: &str) -> bool {
    !value.is_empty() && value.len() <= 256 && value.bytes().all(|byte| byte.is_ascii_graphic())
}

pub(super) fn required_or_default(name: &str, default: &str) -> Result<String, String> {
    configured_value(name, std::env::var(name), default)
}

pub(super) fn configured_value(
    name: &str,
    supplied: Result<String, std::env::VarError>,
    default: &str,
) -> Result<String, String> {
    match supplied {
        Ok(value) if !value.is_empty() => Ok(value),
        Ok(_) => Err(format!("{name} must not be empty")),
        Err(std::env::VarError::NotPresent) => Ok(String::from(default)),
        Err(std::env::VarError::NotUnicode(_)) => Err(format!("{name} is not valid UTF-8")),
    }
}

pub(super) fn safe_header_value(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && !value.contains("..")
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':' | b'/')
        })
}

pub(super) fn recovery_or_gateway_token() -> Result<String, String> {
    if recovery_profile_selected() {
        return match std::env::var("STS2_RECOVERY_TOKEN") {
            Ok(value) if !value.is_empty() => Ok(value),
            Ok(_) => Err(String::from("STS2_RECOVERY_TOKEN must not be empty")),
            Err(std::env::VarError::NotUnicode(_)) => {
                Err(String::from("STS2_RECOVERY_TOKEN is not valid UTF-8"))
            }
            Err(std::env::VarError::NotPresent) => {
                Err(String::from("STS2_RECOVERY_TOKEN is required"))
            }
        };
    }
    required("STS2_GATEWAY_TOKEN")
}

pub(super) fn recovery_profile_selected() -> bool {
    matches!(
        std::env::var("STS2_RUNTIME_PROFILE"),
        Ok(value) if value == "watchdog-recovery-v1"
    )
}

pub(super) fn optional_recovery_proof() -> Result<Option<String>, String> {
    match std::env::var("STS2_RECOVERY_PROOF") {
        Ok(value) if !value.is_empty() => {
            if value.len() > 512 || !value.bytes().all(|byte| byte.is_ascii_graphic()) {
                return Err(String::from("STS2_RECOVERY_PROOF is unsafe or oversized"));
            }
            Ok(Some(value))
        }
        Ok(_) => Err(String::from("STS2_RECOVERY_PROOF must not be empty")),
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(std::env::VarError::NotUnicode(_)) => {
            Err(String::from("STS2_RECOVERY_PROOF is not valid UTF-8"))
        }
    }
}

pub(super) fn safe_recovery_principal(value: &str) -> bool {
    safe_recovery_uuid(value)
        && value.as_bytes().get(14) == Some(&b'4')
        && value
            .as_bytes()
            .get(19)
            .is_some_and(|byte| matches!(*byte, b'8' | b'9' | b'a' | b'b'))
}

pub(super) fn safe_recovery_uuid(value: &str) -> bool {
    value.len() == 36
        && value
            .as_bytes()
            .get(19)
            .is_some_and(|byte| matches!(*byte, b'8' | b'9' | b'a' | b'b'))
        && value.as_bytes().iter().enumerate().all(|(index, byte)| {
            matches!(index, 8 | 13 | 18 | 23) && *byte == b'-'
                || !matches!(index, 8 | 13 | 18 | 23)
                    && (byte.is_ascii_digit() || (b'a'..=b'f').contains(byte))
        })
}

pub(super) fn value_is_recovery(request: &GatewayRequest) -> bool {
    request.body.as_ref().is_some_and(|body| {
        matches!(body, JsonValue::Object(object) if object.get("contract") == Some(&JsonValue::string(RECOVERY_CONTRACT)))
    })
}

fn required(name: &str) -> Result<String, String> {
    std::env::var(name).map_err(|_| format!("{name} is required"))
}
