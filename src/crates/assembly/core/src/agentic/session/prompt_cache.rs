//! Prompt cache compatibility facade.
//!
//! `alex-agent-runtime` owns prompt-cache identities, policy, DTOs, and
//! in-memory runtime store. Core keeps this module for old import paths and
//! concrete session persistence wiring.

pub use alex_agent_runtime::prompt_cache::*;
