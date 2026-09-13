// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;
use std::fmt;
use std::ops::{BitAnd, BitOr};

use crate::json::JsonValue;

pub const NEGOTIATED_COMPOSITION_REVISION: &str = "negotiated-composition-v1-mcp";
pub const CAPABILITY_DISCOVERY_TOOL: &str = "sts2.capabilities";
pub const NEGOTIATION_STALE_CODE: i32 = -32009;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum CapabilityGroup {
    StaticReference,
    LiveDetails,
    GameplayActions,
    Maps,
    ProfileReads,
    ResearchReads,
}

impl CapabilityGroup {
    pub const ALL: [Self; 6] = [
        Self::StaticReference,
        Self::LiveDetails,
        Self::GameplayActions,
        Self::Maps,
        Self::ProfileReads,
        Self::ResearchReads,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::StaticReference => "static_reference",
            Self::LiveDetails => "live_details",
            Self::GameplayActions => "gameplay_actions",
            Self::Maps => "maps",
            Self::ProfileReads => "profile_reads",
            Self::ResearchReads => "research_reads",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CapabilityScope(u16);

impl CapabilityScope {
    pub const NONE: Self = Self(0);
    pub const READ: Self = Self(1);
    pub const MUTATE: Self = Self(1 << 1);
    pub const CONTROL: Self = Self(1 << 2);
    pub const PROFILE: Self = Self(1 << 3);
    pub const RESEARCH: Self = Self(1 << 4);
    pub const ALL: Self =
        Self(Self::READ.0 | Self::MUTATE.0 | Self::CONTROL.0 | Self::PROFILE.0 | Self::RESEARCH.0);

    pub const fn contains(self, required: Self) -> bool {
        self.0 & required.0 == required.0
    }

    pub const fn intersect(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }

    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub const fn bits(self) -> u16 {
        self.0
    }

    pub fn names(self) -> Vec<JsonValue> {
        [
            (Self::READ, "read"),
            (Self::MUTATE, "mutate"),
            (Self::CONTROL, "control"),
            (Self::PROFILE, "profile"),
            (Self::RESEARCH, "research"),
        ]
        .into_iter()
        .filter(|(scope, _)| self.contains(*scope))
        .map(|(_, name)| JsonValue::string(name))
        .collect()
    }
}

impl BitOr for CapabilityScope {
    type Output = Self;

    fn bitor(self, other: Self) -> Self::Output {
        Self(self.0 | other.0)
    }
}

impl BitAnd for CapabilityScope {
    type Output = Self;

    fn bitand(self, other: Self) -> Self::Output {
        Self(self.0 & other.0)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ToolLimits {
    pub max_request_bytes: usize,
    pub max_response_bytes: usize,
    pub max_content_bytes: usize,
    pub max_page_items: usize,
}

impl ToolLimits {
    pub const fn bounded(
        max_request_bytes: usize,
        max_response_bytes: usize,
        max_content_bytes: usize,
        max_page_items: usize,
    ) -> Self {
        Self {
            max_request_bytes,
            max_response_bytes,
            max_content_bytes,
            max_page_items,
        }
    }

    pub fn minimum(self, other: Self) -> Self {
        Self {
            max_request_bytes: self.max_request_bytes.min(other.max_request_bytes),
            max_response_bytes: self.max_response_bytes.min(other.max_response_bytes),
            max_content_bytes: self.max_content_bytes.min(other.max_content_bytes),
            max_page_items: self.max_page_items.min(other.max_page_items),
        }
    }

    pub(crate) fn valid(self) -> bool {
        self.max_request_bytes > 0
            && self.max_response_bytes > 0
            && self.max_content_bytes > 0
            && self.max_page_items > 0
            && self.max_request_bytes <= crate::transport::MAX_FRAME_BYTES
            && self.max_response_bytes <= crate::transport::MAX_FRAME_BYTES
            && self.max_content_bytes <= crate::transport::MAX_FRAME_BYTES
            && self.max_page_items <= 4096
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum CapabilityOwner {
    Producer,
    Gateway,
    Mcp,
}

impl CapabilityOwner {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Producer => "producer",
            Self::Gateway => "gateway",
            Self::Mcp => "mcp",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UnavailableReason {
    MissingProducer,
    MissingGateway,
    MissingMcp,
    ProducerUnsupported,
    ScopeDenied,
    PermissionDenied,
}

impl UnavailableReason {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::MissingProducer => "producer_missing",
            Self::MissingGateway => "gateway_missing",
            Self::MissingMcp => "mcp_missing",
            Self::ProducerUnsupported => "producer_unsupported",
            Self::ScopeDenied => "caller_scope_denied",
            Self::PermissionDenied => "permission_denied",
        }
    }
}

/// Authority identity attached to a gateway or producer capability layer.
///
/// The epoch fences capability evidence across restart/reload events while the
/// digest distinguishes two offers observed at the same epoch.  A digest is an
/// opaque owner-issued identity; the MCP boundary only validates that it is a
/// bounded transport-safe value and compares it byte-for-byte.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapabilityAuthority {
    pub epoch: u64,
    pub digest: String,
}

impl CapabilityAuthority {
    pub fn new(epoch: u64, digest: impl Into<String>) -> Result<Self, NegotiationError> {
        let digest = digest.into();
        if digest.is_empty()
            || digest.len() > 128
            || !digest.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-')
            })
        {
            return Err(NegotiationError::InvalidAuthorityDigest(digest));
        }
        Ok(Self { epoch, digest })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NegotiatedOperation {
    pub operation: String,
    pub revision: String,
    pub group: CapabilityGroup,
    pub required_scope: CapabilityScope,
    pub effective_scope: CapabilityScope,
    pub limits: ToolLimits,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnavailableCapability {
    pub operation: String,
    pub group: CapabilityGroup,
    pub reason: UnavailableReason,
    pub required_scope: CapabilityScope,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NegotiatedCapabilitySet {
    pub revision: String,
    pub(crate) caller_scope: CapabilityScope,
    pub(crate) gateway_authority: CapabilityAuthority,
    pub(crate) producer_authority: CapabilityAuthority,
    pub(crate) operations: BTreeMap<String, NegotiatedOperation>,
    pub(crate) unavailable: BTreeMap<String, UnavailableCapability>,
}

impl NegotiatedCapabilitySet {
    pub fn available(&self, operation: &str) -> Option<&NegotiatedOperation> {
        self.operations.get(operation)
    }

    pub fn operations(&self) -> impl Iterator<Item = &NegotiatedOperation> {
        self.operations.values()
    }

    pub fn unavailable(&self) -> impl Iterator<Item = &UnavailableCapability> {
        self.unavailable.values()
    }

    pub fn len(&self) -> usize {
        self.operations.len()
    }

    pub fn is_empty(&self) -> bool {
        self.operations.is_empty()
    }

    pub fn caller_scope(&self) -> CapabilityScope {
        self.caller_scope
    }

    pub fn gateway_authority(&self) -> &CapabilityAuthority {
        &self.gateway_authority
    }

    pub fn producer_authority(&self) -> &CapabilityAuthority {
        &self.producer_authority
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NegotiationError {
    InvalidOperation(String),
    InvalidRevision(String),
    InvalidLimits(String),
    InvalidAuthorityDigest(String),
    UntrustedCapabilityLayer(CapabilityOwner),
    DuplicateOperation(String),
    RevisionConflict {
        operation: String,
        left: String,
        right: String,
    },
    NoCompatibleOperations,
}

impl fmt::Display for NegotiationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidOperation(operation) => {
                write!(formatter, "invalid capability operation {operation}")
            }
            Self::InvalidRevision(revision) => {
                write!(formatter, "invalid capability revision {revision}")
            }
            Self::InvalidLimits(operation) => write!(formatter, "invalid limits for {operation}"),
            Self::InvalidAuthorityDigest(digest) => {
                write!(formatter, "invalid capability authority digest {digest}")
            }
            Self::UntrustedCapabilityLayer(owner) => {
                write!(
                    formatter,
                    "{} capability layer lacks injected authority",
                    owner.as_str()
                )
            }
            Self::DuplicateOperation(operation) => {
                write!(formatter, "duplicate capability operation {operation}")
            }
            Self::RevisionConflict {
                operation,
                left,
                right,
            } => {
                write!(
                    formatter,
                    "revision conflict for {operation}: {left} versus {right}"
                )
            }
            Self::NoCompatibleOperations => {
                formatter.write_str("no compatible capability operations")
            }
        }
    }
}

impl std::error::Error for NegotiationError {}
