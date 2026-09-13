// SPDX-License-Identifier: MIT

#[path = "catalog_composition_json.rs"]
mod json;
#[path = "catalog_composition_negotiation.rs"]
mod negotiation;
#[path = "catalog_composition_offers.rs"]
mod offers;
#[path = "catalog_composition_profile.rs"]
mod profile;
#[path = "catalog_composition_types.rs"]
mod types;

#[cfg(test)]
#[path = "catalog_composition_tests.rs"]
mod tests;

pub(crate) use json::composition_metadata;
pub(crate) use negotiation::compose_profiles;
pub use offers::{CapabilityLayer, CapabilityOffer, NegotiationRequest};
pub use types::{
    CAPABILITY_DISCOVERY_TOOL, CapabilityAuthority, CapabilityGroup, CapabilityOwner,
    CapabilityScope, NEGOTIATED_COMPOSITION_REVISION, NEGOTIATION_STALE_CODE,
    NegotiatedCapabilitySet, NegotiatedOperation, NegotiationError, ToolLimits,
    UnavailableCapability, UnavailableReason,
};
