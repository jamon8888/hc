//! Prompt pseudonymization — replaces PII in user prompts before sending to cloud LLMs.

#[cfg(feature = "basemind")]
use std::collections::HashMap;
#[cfg(feature = "basemind")]
use std::sync::Arc;
#[cfg(feature = "basemind")]
use anyhow::{Result, Error as AnyhowError};

#[cfg(feature = "basemind")]
use crate::ipc::{BasemindClient, BasemindError};
#[cfg(feature = "basemind")]
use crate::models::PiiFinding;
#[cfg(feature = "basemind")]
use crate::session_vault::{SessionVault, VaultError};

/// Errors from pseudonymization.
#[cfg(feature = "basemind")]
#[derive(Debug, thiserror::Error)]
pub enum PseudonymError {
    #[error("Basemind error: {0}")]
    Basemind(#[from] BasemindError),
    #[error("Vault error: {0}")]
    Vault(#[from] VaultError),
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    #[error("Internal error: {0}")]
    Internal(AnyhowError),
}

#[cfg(feature = "basemind")]
impl From<anyhow::Error> for PseudonymError {
    fn from(err: anyhow::Error) -> Self {
        PseudonymError::Internal(err)
    }
}

/// Core pseudonymization engine.
#[cfg(feature = "basemind")]
pub struct PromptPseudonymizer {
    basemind_client: Arc<BasemindClient>,
    vault: Arc<SessionVault>,
}

#[cfg(feature = "basemind")]
impl PromptPseudonymizer {
    /// Create a new pseudonymizer.
    pub fn new(basemind_client: Arc<BasemindClient>, vault: Arc<SessionVault>) -> Self {
        Self {
            basemind_client,
            vault,
        }
    }

    /// Pseudonymize user prompt text.
    /// Returns (pseudonymized_text, rehydration_map).
    pub async fn pseudonymize(&self, text: &str, session_id: &str) 
        -> Result<(String, HashMap<String, String>), PseudonymError> 
    {
        if text.trim().is_empty() {
            return Ok((text.to_string(), HashMap::new()));
        }

        // Detect PII using Basemind
        let findings = self.basemind_client.detect_pii(text).await?;
        
        if findings.is_empty() {
            return Ok((text.to_string(), HashMap::new()));
        }

        // Build rehydration map and pseudonymized text
        let mut rehydration_map = HashMap::new();
        let mut pseudonymized = text.to_string();
        
        // Sort by byte offset descending so replacements don't shift indices
        let mut sorted_findings = findings;
        sorted_findings.sort_by(|a, b| b.start.cmp(&a.start));
        
        for (idx, finding) in sorted_findings.iter().enumerate() {
            let token = format!("[{}_{}]", finding.category.to_uppercase(), idx + 1);
            let original = &text[finding.start..finding.end];
            rehydration_map.insert(token.clone(), original.to_string());
            
            // Replace in pseudonymized text
            pseudonymized.replace_range(finding.start..finding.end, &token);
        }
        
        // Encrypt and store in session vault
        let vault_key = format!("session:{}:prompt_rehydration", session_id);
        self.vault.encrypt_and_store(&vault_key, &rehydration_map).await?;
        
        Ok((pseudonymized, rehydration_map))
    }
    
    /// Rehydrate a pseudonymized text for display.
    pub async fn rehydrate(&self, text: &str, session_id: &str) -> Result<String, PseudonymError> {
        let vault_key = format!("session:{}:prompt_rehydration", session_id);
        let map = self.vault.decrypt_and_load(&vault_key).await?;
        
        if map.is_empty() {
            return Ok(text.to_string());
        }
        
        let mut result = text.to_string();
        
        // Sort by token length descending to avoid partial replacements
        let mut tokens: Vec<_> = map.keys().collect();
        tokens.sort_by(|a, b| b.len().cmp(&a.len()));
        
        for token in tokens {
            if let Some(original) = map.get(token) {
                result = result.replace(token, original);
            }
        }
        
        Ok(result)
    }
}