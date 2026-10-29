//! Core-compatible event layer facade.
//!
//! Provider-neutral queue and routing owners live in `alex-agent-runtime`.

pub mod queue {
    pub use alex_agent_runtime::event_queue::*;
}

pub mod router {
    pub use alex_agent_runtime::event_router::*;
}

pub mod types;

pub use queue::*;
pub use router::*;
pub use types::*;
