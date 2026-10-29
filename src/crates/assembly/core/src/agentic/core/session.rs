//! Compatibility facade for runtime-owned session facts.

pub use alex_agent_runtime::session::{
    sanitize_persisted_session_state, CompressionState, PersistedSessionStateFile, Session,
    SessionAgentRouteOwner, SessionConfig, SessionContinuationPolicy, SessionKind,
    SessionModelBindingPolicy, SessionSummary,
};
