//! Per-session encrypted vault for rehydration maps and session salts.
//!
//! Stores per-session payloads (rehydration map + salt) encrypted with a passphrase
//! retrieved from the OS keychain. The passphrase is obtained via
//! `crate::keychain::get_or_create_vault_passphrase()` at construction.

#[cfg(feature = "basemind")]
use anyhow::Result;
#[cfg(feature = "basemind")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "basemind")]
use std::collections::BTreeMap;
#[cfg(feature = "basemind")]
use std::fs;
#[cfg(feature = "basemind")]
use std::path::PathBuf;

#[cfg(feature = "basemind")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionPayload {
    pub map: BTreeMap<String, String>, // salted_token -> original value
    pub salt: String,                  // hex-encoded per-session salt
}

#[cfg(feature = "basemind")]
#[derive(Debug, Clone)]
pub struct SessionVault {
    vault_dir: PathBuf,
    passphrase: String, // from OS keychain
}

#[cfg(feature = "basemind")]
impl SessionVault {
    /// Create a new SessionVault, fetching the passphrase from the OS keychain.
    pub fn new(vault_dir: PathBuf) -> Result<Self> {
        let passphrase = crate::keychain::get_or_create_vault_passphrase()?;
        Ok(Self {
            vault_dir,
            passphrase,
        })
    }

    /// Create with an explicit passphrase (for testing).
    pub fn with_passphrase(vault_dir: PathBuf, passphrase: String) -> Self {
        Self {
            vault_dir,
            passphrase,
        }
    }

    /// Store a session's rehydration map and salt.
    pub fn store_session(&self, session_id: &str, payload: SessionPayload) -> Result<()> {
        let encrypted = crate::vault::vault_encrypt(&payload, &self.passphrase)?;
        let path = self.vault_dir.join(format!("{session_id}.vault"));
        fs::write(path, encrypted)?;
        Ok(())
    }

    /// Load a session's rehydration map and salt for UI rehydration.
    pub fn load_session(&self, session_id: &str) -> Result<SessionPayload> {
        let path = self.vault_dir.join(format!("{session_id}.vault"));
        let encrypted = fs::read_to_string(path)?;
        crate::vault::vault_decrypt(&encrypted, &self.passphrase).map_err(anyhow::Error::new)
    }

    /// Delete a session's vault file on expiry.
    pub fn delete_session(&self, session_id: &str) -> Result<()> {
        let path = self.vault_dir.join(format!("{session_id}.vault"));
        let _ = fs::remove_file(path);
        Ok(())
    }

    /// Generate a fresh per-session salt (≥128-bit, CSPRNG).
    pub fn generate_salt() -> String {
        let mut bytes = [0u8; 16]; // 128-bit
        rand::RngCore::fill_bytes(&mut rand::rngs::OsRng, &mut bytes);
        hex::encode(bytes)
    }
}
