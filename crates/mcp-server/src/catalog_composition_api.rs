// SPDX-License-Identifier: MIT

use super::{
    CapabilityLayer, CapabilityScope, NegotiatedCapabilitySet, NegotiationError, ToolCatalog,
};

impl ToolCatalog {
    pub fn compose_profiles(
        profiles: &[Self],
        gateway: CapabilityLayer,
        producer: CapabilityLayer,
        caller_scope: CapabilityScope,
    ) -> Result<Self, NegotiationError> {
        super::composition::compose_profiles(profiles, gateway, producer, caller_scope)
    }

    pub fn compose(
        profiles: &[Self],
        gateway: CapabilityLayer,
        producer: CapabilityLayer,
        caller_scope: CapabilityScope,
    ) -> Result<Self, NegotiationError> {
        Self::compose_profiles(profiles, gateway, producer, caller_scope)
    }

    pub fn compose_gameplay_lookup(
        gateway: CapabilityLayer,
        producer: CapabilityLayer,
        caller_scope: CapabilityScope,
    ) -> Result<Self, NegotiationError> {
        Self::compose_profiles(
            &[
                Self::runtime_map_v1(),
                Self::game_information_query_v1(),
                Self::game_information_live_observation_bootstrap(),
            ],
            gateway,
            producer,
            caller_scope,
        )
    }

    pub fn capability_layer(&self) -> Result<CapabilityLayer, NegotiationError> {
        CapabilityLayer::from_catalog(self)
    }

    pub fn negotiated_capabilities(&self) -> Option<&NegotiatedCapabilitySet> {
        self.composition.as_ref()
    }
}
