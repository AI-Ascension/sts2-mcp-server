// SPDX-License-Identifier: MIT

use crate::catalog::NegotiatedCapabilitySet;

use super::{McpServer, SessionRefreshReason};

pub(super) fn advance_authority_epoch<G>(
    server: &mut McpServer<G>,
    reason: &SessionRefreshReason,
) -> Result<(), String> {
    let producer_authority_changed = matches!(
        reason,
        SessionRefreshReason::ProducerRestart
            | SessionRefreshReason::ContentReload
            | SessionRefreshReason::ToolSetRevisionChanged(_)
    );
    let gateway_authority_changed =
        matches!(reason, SessionRefreshReason::ToolSetRevisionChanged(_));
    if producer_authority_changed {
        server.producer_authority_epoch = server
            .producer_authority_epoch
            .checked_add(1)
            .ok_or_else(|| String::from("producer capability epoch exhausted"))?;
        let previous_digest = server.producer_authority_digest.take();
        if server.stale_producer_authority_digest.is_none() {
            server.stale_producer_authority_digest = previous_digest;
        }
    }
    if gateway_authority_changed {
        server.gateway_authority_epoch = server
            .gateway_authority_epoch
            .checked_add(1)
            .ok_or_else(|| String::from("gateway capability epoch exhausted"))?;
        let previous_digest = server.gateway_authority_digest.take();
        if server.stale_gateway_authority_digest.is_none() {
            server.stale_gateway_authority_digest = previous_digest;
        }
    }
    Ok(())
}

pub(super) fn validate_refresh_authority<G>(
    server: &McpServer<G>,
    composition: &NegotiatedCapabilitySet,
) -> Result<(), String> {
    if composition.gateway_authority().epoch != server.gateway_authority_epoch {
        return Err(String::from(
            "refreshed catalog has a stale gateway capability epoch",
        ));
    }
    if composition.producer_authority().epoch != server.producer_authority_epoch {
        return Err(String::from(
            "refreshed catalog has a stale producer capability epoch",
        ));
    }
    if server
        .gateway_authority_digest
        .as_deref()
        .is_some_and(|digest| digest != composition.gateway_authority().digest)
    {
        return Err(String::from(
            "refreshed catalog has a different gateway authority digest without a refresh event",
        ));
    }
    if server
        .producer_authority_digest
        .as_deref()
        .is_some_and(|digest| digest != composition.producer_authority().digest)
    {
        return Err(String::from(
            "refreshed catalog has a different producer authority digest without a refresh event",
        ));
    }
    if server
        .stale_gateway_authority_digest
        .as_deref()
        .is_some_and(|digest| digest == composition.gateway_authority().digest)
        || server
            .stale_producer_authority_digest
            .as_deref()
            .is_some_and(|digest| digest == composition.producer_authority().digest)
    {
        return Err(String::from(
            "refreshed catalog reuses capability evidence invalidated by the session event",
        ));
    }
    Ok(())
}
