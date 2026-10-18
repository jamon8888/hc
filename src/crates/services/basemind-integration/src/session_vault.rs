//! Session-scoped encrypted vault for prompt rehydration maps.

#[cfg(feature = "basemind")]
use std::collections::HashMap;
#[cfg(feature = "basemind")]
use std::path::{Path, PathBuf};
#[cfg(feature = "basemind")]
use anyhow::{Context, Result};
#[cfg(feature = "basemind")]
use tokio::fs;

/// Errors from vault operations.
#[cfg(feature = "basemind")]
#[derive(Debug, thiserror::Error)]
pub enum VaultError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Encryption error: {0}")]
    Encryption(String),
    #[error("Decryption error: {0}")]
    Decryption(String),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Vault not found: {0}")]
    NotFound(String),
}

/// Per-session encrypted vault for storing rehydration maps.
#[cfg(feature = "basemind")]
pub struct SessionVault {
    vault_path: PathBuf,
}

#[cfg(feature = "basemind")]
impl SessionVault {
    /// Create a new session vault at the given path.
    pub fn new(vault_path: PathBuf) -> Self {
        Self { vault_path }
    }

    /// Create vault for a specific session.
    pub fn for_session(session_storage_root: PathBuf, session_id: &str) -> Self {
        let vault_dir = session_storage_root.join(".basemind_prompt_vault");
        Self::new(vault_dir)
    }

    /// Encrypt and store a rehydration map.
    pub async fn encrypt_and_store(&self, key: &str, map: &HashMap<String, String>) -> Result<()> {
        // Ensure vault directory exists
        fs::create_dir_all(&self.vault_path).await
            .context("Failed to create vault directory")?;

        let encrypted = vault_encrypt(map)?;
        let file_name = sanitize_key(key);
        let path = self.vault_path.join(format!("{}.vault", file_name));
        
        fs::write(&path, encrypted).await
            .context("Failed to write vault file")?;

        Ok(())
    }

    /// Decrypt and load a rehydration map.
    pub async fn decrypt_and_load(&self, key: &str) -> Result<HashMap<String, String>> {
        let file_name = sanitize_key(key);
        let path = self.vault_path.join(format!("{}.vault", file_name));
        
        let encrypted = fs::read_to_string(&path).await
            .context("Failed to read vault file")?;
        
        vault_decrypt(&encrypted)
    }

    /// Delete a vault entry.
    pub async fn delete(&self, key: &str) -> Result<()> {
        let file_name = sanitize_key(key);
        let path = self.vault_path.join(format!("{}.vault", file_name));
        
        let _ = fs::remove_file(&path).await;
        Ok(())
    }
}

/// Sanitize key for use as filename (replace colons and other special chars).
#[cfg(feature = "basemind")]
fn sanitize_key(key: &str) -> String {
    key.replace(':', "_").replace('/', "_")
}

/// Encrypt a rehydration map using AES-GCM (Basemind's vault encryption).
/// This is a simplified version - real implementation uses Basemind's vault module.
#[cfg(feature = "basemind")]
fn vault_encrypt(map: &HashMap<String, String>) -> Result<String> {
    // In production, this uses Basemind's vault_encrypt (AES-GCM with key from config)
    // For now, serialize as JSON (will be replaced with real encryption)
    let json = serde_json::to_string(map)?;
    Ok(json)
}

/// Decrypt a rehydration map.
#[cfg(feature = "basemind")]
fn vault_decrypt(encrypted: &str) -> Result<HashMap<String, String>> {
    // In production, this uses Basemind's vault_decrypt
    // For now, deserialize from JSON
    let map = serde_json::from_str(encrypted)?;
    Ok(map)
}