//! Negotiated Basemind privacy capability for host-boundary handshakes.
//!
//! This module describes facts about the host that owns the workspace. It does
//! not start a sidecar, inspect the filesystem, or install a privacy policy;
//! those actions remain owned by the host/service composition root. A host
//! must publish the positive peer capability only after it has assembled these
//! facts and the controller must negotiate them before sending an API turn.

use std::fmt;

use semver::Version;
use serde::{Deserialize, Serialize};

/// Wire capability id used by `peer_mode_ping` projections.
pub const BASEMIND_CAPABILITY_ID: &str = "basemind_privacy_v1";
/// Version of this capability advertisement and negotiation contract.
pub const BASEMIND_CAPABILITY_PROTOCOL_VERSION: u16 = 1;
/// Lowest sidecar version whose privacy contract this product understands.
pub const BASEMIND_MIN_SIDECAR_VERSION: &str = "0.36.0";

/// Why a host cannot offer a usable Basemind privacy pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BasemindUnavailableReason {
    NotConfigured,
    SidecarBinaryMissing,
    SidecarUnavailable,
    SidecarVersionUnknown,
    NerModelMissing,
    NerModelVersionUnknown,
    PrivacyPipelineUnavailable,
    WorkspaceOwnerUnknown,
}

impl Default for BasemindUnavailableReason {
    fn default() -> Self {
        Self::NotConfigured
    }
}

/// Privacy guarantees a host claims to provide for the advertised workspace.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BasemindPrivacyCapabilities {
    pub prompt_pseudonymization: bool,
    pub document_redaction: bool,
    pub rag_search: bool,
    pub session_vault: bool,
    pub identity_registry: bool,
    pub fail_closed_api_route: bool,
}

impl BasemindPrivacyCapabilities {
    /// The complete set required before an API turn may leave the host.
    pub const fn required() -> Self {
        Self {
            prompt_pseudonymization: true,
            document_redaction: true,
            rag_search: true,
            session_vault: true,
            identity_registry: true,
            fail_closed_api_route: true,
        }
    }

    fn first_missing(self, required: Self) -> Option<&'static str> {
        [
            (
                self.prompt_pseudonymization,
                required.prompt_pseudonymization,
                "prompt_pseudonymization",
            ),
            (
                self.document_redaction,
                required.document_redaction,
                "document_redaction",
            ),
            (self.rag_search, required.rag_search, "rag_search"),
            (self.session_vault, required.session_vault, "session_vault"),
            (
                self.identity_registry,
                required.identity_registry,
                "identity_registry",
            ),
            (
                self.fail_closed_api_route,
                required.fail_closed_api_route,
                "fail_closed_api_route",
            ),
        ]
        .into_iter()
        .find_map(|(provided, required, name)| (required && !provided).then_some(name))
    }
}

/// State of the host-owned Basemind pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "state", content = "reason")]
pub enum BasemindCapabilityState {
    Ready,
    Unavailable(BasemindUnavailableReason),
}

impl Default for BasemindCapabilityState {
    fn default() -> Self {
        Self::Unavailable(BasemindUnavailableReason::NotConfigured)
    }
}

/// Host facts projected in `peer_mode_ping` and reusable by headless hosts.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BasemindCapabilityAdvertisement {
    #[serde(default)]
    pub protocol_version: u16,
    #[serde(default)]
    pub state: BasemindCapabilityState,
    #[serde(default)]
    pub sidecar_binary: bool,
    #[serde(default)]
    pub sidecar_version: Option<String>,
    #[serde(default)]
    pub ner_model_available: bool,
    #[serde(default)]
    pub ner_model_version: Option<String>,
    #[serde(default)]
    pub privacy: BasemindPrivacyCapabilities,
    #[serde(default)]
    pub workspace_owner: bool,
}

impl BasemindCapabilityAdvertisement {
    /// Build an unavailable advertisement without exposing host paths or raw PII.
    pub const fn unavailable(reason: BasemindUnavailableReason) -> Self {
        Self {
            protocol_version: BASEMIND_CAPABILITY_PROTOCOL_VERSION,
            state: BasemindCapabilityState::Unavailable(reason),
            sidecar_binary: false,
            sidecar_version: None,
            ner_model_available: false,
            ner_model_version: None,
            privacy: BasemindPrivacyCapabilities {
                prompt_pseudonymization: false,
                document_redaction: false,
                rag_search: false,
                session_vault: false,
                identity_registry: false,
                fail_closed_api_route: false,
            },
            workspace_owner: false,
        }
    }

    /// Build an advertisement from host-owned, already-probed facts.
    pub fn from_host_facts(
        sidecar_binary: bool,
        sidecar_version: Option<String>,
        ner_model_available: bool,
        ner_model_version: Option<String>,
        privacy: BasemindPrivacyCapabilities,
        workspace_owner: bool,
    ) -> Self {
        let reason = if !sidecar_binary {
            Some(BasemindUnavailableReason::SidecarBinaryMissing)
        } else if sidecar_version.is_none() {
            Some(BasemindUnavailableReason::SidecarVersionUnknown)
        } else if !ner_model_available {
            Some(BasemindUnavailableReason::NerModelMissing)
        } else if ner_model_version.is_none() {
            Some(BasemindUnavailableReason::NerModelVersionUnknown)
        } else if privacy
            .first_missing(BasemindPrivacyCapabilities::required())
            .is_some()
        {
            Some(BasemindUnavailableReason::PrivacyPipelineUnavailable)
        } else if !workspace_owner {
            Some(BasemindUnavailableReason::WorkspaceOwnerUnknown)
        } else {
            None
        };

        Self {
            protocol_version: BASEMIND_CAPABILITY_PROTOCOL_VERSION,
            state: reason.map_or(
                BasemindCapabilityState::Ready,
                BasemindCapabilityState::Unavailable,
            ),
            sidecar_binary,
            sidecar_version,
            ner_model_available,
            ner_model_version,
            privacy,
            workspace_owner,
        }
    }

    /// Whether this advertisement is sufficient for the default API contract.
    pub fn is_usable(&self) -> bool {
        negotiate_basemind(Some(self), &BasemindCapabilityRequirement::default()).is_ok()
    }
}

/// Requirements a controller places on the host that will execute a turn.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BasemindCapabilityRequirement {
    pub protocol_version: u16,
    pub minimum_sidecar_version: String,
    pub require_ner_model: bool,
    pub require_workspace_owner: bool,
    pub privacy: BasemindPrivacyCapabilities,
}

impl Default for BasemindCapabilityRequirement {
    fn default() -> Self {
        Self {
            protocol_version: BASEMIND_CAPABILITY_PROTOCOL_VERSION,
            minimum_sidecar_version: BASEMIND_MIN_SIDECAR_VERSION.to_string(),
            require_ner_model: true,
            require_workspace_owner: true,
            privacy: BasemindPrivacyCapabilities::required(),
        }
    }
}

/// The negotiated result that can be attached to a host/session context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BasemindNegotiatedCapability {
    pub protocol_version: u16,
    pub sidecar_version: Version,
    pub ner_model_version: Option<String>,
}

/// A fail-closed explanation for rejecting a host capability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BasemindNegotiationError {
    MissingAdvertisement,
    Unavailable(BasemindUnavailableReason),
    ProtocolTooOld {
        required: u16,
        advertised: u16,
    },
    SidecarBinaryUnavailable,
    SidecarVersionMissing,
    InvalidSidecarVersion(String),
    SidecarVersionUnsupported {
        minimum: String,
        advertised: Version,
    },
    NerModelUnavailable,
    NerModelVersionMissing,
    WorkspaceNotOwned,
    MissingPrivacyCapability(&'static str),
    InvalidRequiredSidecarVersion(String),
}

impl fmt::Display for BasemindNegotiationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingAdvertisement => f.write_str("Basemind capability was not advertised"),
            Self::Unavailable(reason) => {
                write!(f, "Basemind capability is unavailable: {reason:?}")
            }
            Self::ProtocolTooOld {
                required,
                advertised,
            } => write!(
                f,
                "Basemind capability protocol {advertised} is older than required {required}"
            ),
            Self::SidecarBinaryUnavailable => f.write_str("Basemind sidecar binary is unavailable"),
            Self::SidecarVersionMissing => f.write_str("Basemind sidecar version is missing"),
            Self::InvalidSidecarVersion(version) => {
                write!(f, "Basemind sidecar version is invalid: {version}")
            }
            Self::SidecarVersionUnsupported {
                minimum,
                advertised,
            } => write!(
                f,
                "Basemind sidecar {advertised} is older than required {minimum}"
            ),
            Self::NerModelUnavailable => f.write_str("Basemind NER model is unavailable"),
            Self::NerModelVersionMissing => f.write_str("Basemind NER model version is missing"),
            Self::WorkspaceNotOwned => f.write_str("Basemind host does not own the workspace"),
            Self::MissingPrivacyCapability(capability) => {
                write!(f, "Basemind privacy capability is missing: {capability}")
            }
            Self::InvalidRequiredSidecarVersion(version) => {
                write!(f, "required Basemind sidecar version is invalid: {version}")
            }
        }
    }
}

impl std::error::Error for BasemindNegotiationError {}

/// Negotiate a host-owned Basemind pipeline before an API or detached turn.
///
/// Missing, malformed, stale, or partial advertisements always return an
/// error. Callers must not treat an error as a local fallback.
pub fn negotiate_basemind(
    advertisement: Option<&BasemindCapabilityAdvertisement>,
    requirement: &BasemindCapabilityRequirement,
) -> Result<BasemindNegotiatedCapability, BasemindNegotiationError> {
    let advertisement = advertisement.ok_or(BasemindNegotiationError::MissingAdvertisement)?;
    if let BasemindCapabilityState::Unavailable(reason) = advertisement.state {
        return Err(BasemindNegotiationError::Unavailable(reason));
    }
    if advertisement.protocol_version < requirement.protocol_version {
        return Err(BasemindNegotiationError::ProtocolTooOld {
            required: requirement.protocol_version,
            advertised: advertisement.protocol_version,
        });
    }
    if !advertisement.sidecar_binary {
        return Err(BasemindNegotiationError::SidecarBinaryUnavailable);
    }
    let sidecar_version = advertisement
        .sidecar_version
        .as_deref()
        .ok_or(BasemindNegotiationError::SidecarVersionMissing)
        .and_then(|version| {
            Version::parse(version)
                .map_err(|_| BasemindNegotiationError::InvalidSidecarVersion(version.to_string()))
        })?;
    let minimum = Version::parse(&requirement.minimum_sidecar_version).map_err(|_| {
        BasemindNegotiationError::InvalidRequiredSidecarVersion(
            requirement.minimum_sidecar_version.clone(),
        )
    })?;
    if sidecar_version < minimum {
        return Err(BasemindNegotiationError::SidecarVersionUnsupported {
            minimum: requirement.minimum_sidecar_version.clone(),
            advertised: sidecar_version,
        });
    }
    let ner_model_version = if requirement.require_ner_model {
        if !advertisement.ner_model_available {
            return Err(BasemindNegotiationError::NerModelUnavailable);
        }
        Some(
            advertisement
                .ner_model_version
                .clone()
                .ok_or(BasemindNegotiationError::NerModelVersionMissing)?,
        )
    } else {
        advertisement.ner_model_version.clone()
    };
    if requirement.require_workspace_owner && !advertisement.workspace_owner {
        return Err(BasemindNegotiationError::WorkspaceNotOwned);
    }
    if let Some(capability) = advertisement.privacy.first_missing(requirement.privacy) {
        return Err(BasemindNegotiationError::MissingPrivacyCapability(
            capability,
        ));
    }
    Ok(BasemindNegotiatedCapability {
        protocol_version: advertisement.protocol_version,
        sidecar_version,
        ner_model_version,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ready() -> BasemindCapabilityAdvertisement {
        BasemindCapabilityAdvertisement::from_host_facts(
            true,
            Some("0.36.1".to_string()),
            true,
            Some("gliner2-bundled".to_string()),
            BasemindPrivacyCapabilities::required(),
            true,
        )
    }

    #[test]
    fn negotiation_accepts_a_ready_workspace_owner() {
        let negotiated = negotiate_basemind(Some(&ready()), &Default::default()).unwrap();
        assert_eq!(negotiated.protocol_version, 1);
        assert_eq!(
            negotiated.sidecar_version,
            Version::parse("0.36.1").unwrap()
        );
        assert_eq!(
            negotiated.ner_model_version.as_deref(),
            Some("gliner2-bundled")
        );
    }

    #[test]
    fn negotiation_fails_closed_without_an_advertisement() {
        assert_eq!(
            negotiate_basemind(None, &Default::default()),
            Err(BasemindNegotiationError::MissingAdvertisement)
        );
    }

    #[test]
    fn negotiation_rejects_partial_or_stale_hosts() {
        let mut partial = ready();
        partial.privacy.document_redaction = false;
        assert_eq!(
            negotiate_basemind(Some(&partial), &Default::default()),
            Err(BasemindNegotiationError::MissingPrivacyCapability(
                "document_redaction"
            ))
        );

        let mut stale = ready();
        stale.sidecar_version = Some("0.35.9".to_string());
        assert!(matches!(
            negotiate_basemind(Some(&stale), &Default::default()),
            Err(BasemindNegotiationError::SidecarVersionUnsupported { .. })
        ));

        let mut foreign = ready();
        foreign.workspace_owner = false;
        foreign.state = BasemindCapabilityState::Ready;
        assert_eq!(
            negotiate_basemind(Some(&foreign), &Default::default()),
            Err(BasemindNegotiationError::WorkspaceNotOwned)
        );
    }

    #[test]
    fn unavailable_advertisement_never_claims_the_positive_capability() {
        let unavailable = BasemindCapabilityAdvertisement::unavailable(
            BasemindUnavailableReason::SidecarUnavailable,
        );
        assert!(!unavailable.is_usable());
        assert!(matches!(
            negotiate_basemind(Some(&unavailable), &Default::default()),
            Err(BasemindNegotiationError::Unavailable(
                BasemindUnavailableReason::SidecarUnavailable
            ))
        ));
    }
}
