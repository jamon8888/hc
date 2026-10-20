//! Persisted configuration DTOs and pure compatibility validation.
pub mod agent_identity_migration;
pub mod conversion;
mod generated_locale_contract;
pub mod normalization;
mod types;
pub use generated_locale_contract::*;
pub use types::*;
