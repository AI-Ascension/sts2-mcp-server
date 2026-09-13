// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use super::types::{
    CapabilityGroup, CapabilityOwner, CapabilityScope, NegotiationError, ToolLimits,
    UnavailableReason,
};
use crate::catalog::ToolCatalog;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapabilityOffer {
    pub operation: String,
    pub revision: String,
    pub group: CapabilityGroup,
    pub scope: CapabilityScope,
    pub required_scope: CapabilityScope,
    pub limits: ToolLimits,
    pub supported: bool,
    pub unavailable_reason: Option<UnavailableReason>,
}

impl CapabilityOffer {
    pub fn supported(
        operation: impl Into<String>,
        revision: impl Into<String>,
        group: CapabilityGroup,
        required_scope: CapabilityScope,
        limits: ToolLimits,
    ) -> Self {
        Self {
            operation: operation.into(),
            revision: revision.into(),
            group,
            scope: required_scope,
            required_scope,
            limits,
            supported: true,
            unavailable_reason: None,
        }
    }

    pub fn unavailable(
        operation: impl Into<String>,
        revision: impl Into<String>,
        group: CapabilityGroup,
        reason: UnavailableReason,
    ) -> Self {
        Self {
            operation: operation.into(),
            revision: revision.into(),
            group,
            scope: CapabilityScope::NONE,
            required_scope: CapabilityScope::READ,
            limits: ToolLimits::bounded(1, 1, 1, 1),
            supported: false,
            unavailable_reason: Some(reason),
        }
    }

    pub fn with_scope(mut self, scope: CapabilityScope) -> Self {
        self.scope = scope;
        self
    }

    pub fn with_required_scope(mut self, required_scope: CapabilityScope) -> Self {
        self.required_scope = required_scope;
        self
    }

    pub(crate) fn validate(&self) -> Result<(), NegotiationError> {
        if self.operation.is_empty()
            || self.operation.len() > 128
            || !self.operation.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-' | b'/')
            })
        {
            return Err(NegotiationError::InvalidOperation(self.operation.clone()));
        }
        if self.revision.is_empty()
            || self.revision.len() > 128
            || !self
                .revision
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
        {
            return Err(NegotiationError::InvalidRevision(self.revision.clone()));
        }
        if !self.limits.valid() {
            return Err(NegotiationError::InvalidLimits(self.operation.clone()));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapabilityLayer {
    pub owner: CapabilityOwner,
    pub revision: String,
    pub(crate) offers: BTreeMap<String, CapabilityOffer>,
}

impl CapabilityLayer {
    pub fn new(owner: CapabilityOwner, revision: impl Into<String>) -> Self {
        Self {
            owner,
            revision: revision.into(),
            offers: BTreeMap::new(),
        }
    }

    pub fn insert(&mut self, offer: CapabilityOffer) -> Result<(), NegotiationError> {
        offer.validate()?;
        if self.offers.contains_key(&offer.operation) {
            return Err(NegotiationError::DuplicateOperation(offer.operation));
        }
        self.offers.insert(offer.operation.clone(), offer);
        Ok(())
    }

    pub fn with_offer(mut self, offer: CapabilityOffer) -> Result<Self, NegotiationError> {
        self.insert(offer)?;
        Ok(self)
    }

    pub fn offer(&self, operation: &str) -> Option<&CapabilityOffer> {
        self.offers.get(operation)
    }

    pub fn operations(&self) -> impl Iterator<Item = &CapabilityOffer> {
        self.offers.values()
    }

    pub fn len(&self) -> usize {
        self.offers.len()
    }

    pub fn is_empty(&self) -> bool {
        self.offers.is_empty()
    }
}

impl CapabilityLayer {
    pub fn from_catalog(catalog: &ToolCatalog) -> Result<Self, NegotiationError> {
        super::profile::layer_from_catalog(CapabilityOwner::Mcp, catalog)
    }

    pub fn from_catalogs(
        owner: CapabilityOwner,
        catalogs: &[ToolCatalog],
    ) -> Result<Self, NegotiationError> {
        super::profile::layer_from_catalogs(owner, catalogs)
    }

    pub fn from_catalog_for(
        owner: CapabilityOwner,
        catalog: &ToolCatalog,
    ) -> Result<Self, NegotiationError> {
        super::profile::layer_from_catalog(owner, catalog)
    }

    pub fn mirrored(
        owner: CapabilityOwner,
        catalog: &ToolCatalog,
    ) -> Result<Self, NegotiationError> {
        Self::from_catalog_for(owner, catalog)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NegotiationRequest {
    pub mcp: CapabilityLayer,
    pub gateway: CapabilityLayer,
    pub producer: CapabilityLayer,
    pub caller_scope: CapabilityScope,
}

impl NegotiationRequest {
    pub fn new(
        mcp: CapabilityLayer,
        gateway: CapabilityLayer,
        producer: CapabilityLayer,
        caller_scope: CapabilityScope,
    ) -> Self {
        Self {
            mcp,
            gateway,
            producer,
            caller_scope,
        }
    }
}
