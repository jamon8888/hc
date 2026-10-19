//! Identity registry — per-workspace persistent store mapping value_hash -> stable alias.
//!
//! Encrypted at rest via vault_encrypt (AES-256-GCM + scrypt). Passphrase from OS keychain.
//!
//! The registry is the cross-document PII identity layer that basemind lacks:
//! basemind's `entity_id` is a per-file span id (`file:category:start:end`) and its
//! `value_hash` is a hash of the positional token, not the value. This registry maps
//! a hash of the REAL PII value to a stable per-workspace alias.

#[cfg(feature = "basemind")]
use crate::keychain;
#[cfg(feature = "basemind")]
use anyhow::Result;
#[cfg(feature = "basemind")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "basemind")]
use std::collections::HashMap;
#[cfg(feature = "basemind")]
use std::path::PathBuf;

#[cfg(feature = "basemind")]
#[derive(Debug, Clone, Serialize, Deserialize)]
struct RegistryStore {
    version: u32,
    entries: HashMap<String, String>, // value_hash -> alias
}

#[cfg(feature = "basemind")]
#[derive(Debug, Clone)]
pub struct IdentityRegistry {
    store_path: PathBuf,
    passphrase: String,
    cache: HashMap<String, String>, // value_hash -> alias
}

#[cfg(feature = "basemind")]
impl IdentityRegistry {
    /// Create a new IdentityRegistry, fetching the passphrase from the OS keychain.
    pub fn new(workspace_data_dir: PathBuf) -> Result<Self> {
        let passphrase = keychain::get_or_create_vault_passphrase()?;
        Self::with_passphrase(workspace_data_dir, passphrase)
    }

    /// Create with an explicit passphrase (for testing).
    pub fn with_passphrase(workspace_data_dir: PathBuf, passphrase: String) -> Result<Self> {
        let store_path = workspace_data_dir.join("identity_registry.enc");
        let mut cache = HashMap::new();
        if store_path.exists() {
            let encrypted = std::fs::read_to_string(&store_path)?;
            let store: RegistryStore = crate::vault::vault_decrypt(&encrypted, &passphrase)?;
            cache = store.entries;
        }
        Ok(Self {
            store_path,
            passphrase,
            cache,
        })
    }

    /// Compute the value_hash for a PII value (SHA-256 hex).
    pub fn compute_value_hash(value: &str) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(value.as_bytes());
        hex::encode(hasher.finalize())
    }

    /// Get stable alias for a value; creates new one if absent.
    /// Alias format: `ENT_<CATEGORY>_<short_hash>` where short_hash = first 8 hex of value_hash.
    pub fn get_or_create_alias(&mut self, value_hash: &str, category: &str) -> String {
        if let Some(alias) = self.cache.get(value_hash) {
            return alias.clone();
        }
        let short_hash = &value_hash[..8.min(value_hash.len())];
        let alias = format!("ENT_{}_{}", category.to_uppercase(), short_hash);
        self.cache.insert(value_hash.to_string(), alias.clone());
        alias
    }

    /// Get alias for a value, computing its hash first.
    pub fn get_or_create_alias_for_value(&mut self, value: &str, category: &str) -> String {
        let value_hash = Self::compute_value_hash(value);
        self.get_or_create_alias(&value_hash, category)
    }

    /// Reverse lookup for rehydration: alias -> value_hash.
    pub fn resolve_alias(&self, alias: &str) -> Option<String> {
        self.cache
            .iter()
            .find(|(_, v)| *v == alias)
            .map(|(k, _)| k.clone())
    }

    /// Number of registered entities (callers use it to detect fresh mints).
    pub fn entry_count(&self) -> usize {
        self.cache.len()
    }

    /// Persist the registry to disk (encrypted).
    pub fn flush(&self) -> Result<()> {
        let store = RegistryStore {
            version: 1,
            entries: self.cache.clone(),
        };
        let encrypted = crate::vault::vault_encrypt(&store, &self.passphrase)?;
        std::fs::write(&self.store_path, encrypted)?;
        Ok(())
    }
}
