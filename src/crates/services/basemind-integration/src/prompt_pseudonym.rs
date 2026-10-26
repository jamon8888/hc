//! Prompt pseudonymization — intercepts user prompts before they reach the LLM,
//! replaces PII with opaque session tokens, and stores the token -> value map
//! in the session vault (D3/D4).

#[cfg(feature = "basemind")]
use anyhow::Result;
#[cfg(feature = "basemind")]
use std::collections::BTreeMap;

#[cfg(feature = "basemind")]
#[derive(Debug, Clone)]
pub struct PseudonymizedPrompt {
    pub text: String,                              // Session tokens sent to the LLM
    pub rehydration_map: BTreeMap<String, String>, // session token -> original value (this prompt's new entries only)
    pub session_id: String,
}

#[cfg(feature = "basemind")]
pub struct PromptPseudonymizer {
    client: crate::ipc::BasemindClient,
    registry: crate::identity_registry::IdentityRegistry,
    vault: crate::session_vault::SessionVault,
}

#[cfg(feature = "basemind")]
impl PromptPseudonymizer {
    pub fn new(
        client: crate::ipc::BasemindClient,
        registry: crate::identity_registry::IdentityRegistry,
        vault: crate::session_vault::SessionVault,
    ) -> Self {
        Self {
            client,
            registry,
            vault,
        }
    }

    /// Pseudonymize a user prompt for a session.
    /// Returns tokenized text + stores the session map in the vault.
    pub async fn pseudonymize(
        &mut self,
        raw_prompt: &str,
        session_id: &str,
        workspace_root: &str,
    ) -> Result<PseudonymizedPrompt> {
        // 1. Call basemind `redact_text` with strategy="token-replace"
        let redact_result = self.client.redact_text(raw_prompt, workspace_root).await?;
        // The same D7 rule covers a truncated NER pass here: a prompt whose
        // text was only partly read must not reach the model, because the PII
        // past the cut would be sent unchecked.
        if let Some(message) = crate::redaction::ner_coverage_refusal(&redact_result) {
            anyhow::bail!("redact_text safety pass refused the prompt: {message}");
        }

        // Parse response
        let redacted_text = redact_result["redacted_text"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing redacted_text"))?
            .to_string();

        let positional_map = redact_result["rehydration_map"]
            .as_object()
            .ok_or_else(|| anyhow::anyhow!("missing rehydration_map"))?
            .iter()
            .map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string()))
            .collect::<BTreeMap<_, _>>();

        let detections = redact_result["detections"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("missing detections"))?;

        // 2. Load the session payload; on miss, create it with a fresh
        //    >=128-bit CSPRNG salt (never derived from user content, issue 04).
        let session_payload = match self.vault.load_session(session_id) {
            Ok(payload) => payload,
            Err(_) => crate::session_vault::SessionPayload {
                map: BTreeMap::new(),
                salt: crate::session_vault::SessionVault::generate_salt(),
            },
        };

        let session_salt = &session_payload.salt;

        // 3. Build the token map: positional_token -> session token.
        //    The session map is seeded from the vault so entries written by
        //    earlier turns keep rehydrating (D4).
        let mut positional_to_token = BTreeMap::new();
        let mut new_entries = BTreeMap::new();
        let mut session_map = session_payload.map.clone();

        for detection in detections {
            let category = detection["category"].as_str().unwrap_or("UNKNOWN");
            let original_value = detection["text"].as_str().unwrap_or("");
            let positional_token = detection["token"]
                .as_str()
                .or_else(|| {
                    positional_map
                        .keys()
                        .find(|k| positional_map[*k] == original_value)
                        .map(|s| s.as_str())
                })
                .unwrap_or("");

            if positional_token.is_empty() || original_value.is_empty() {
                continue;
            }

            // Stable workspace alias (the registry hashes the value internally)
            let stable_alias = self
                .registry
                .get_or_create_alias_for_value(original_value, category);

            // Mint the opaque session token [CATEGORY_<HMAC16(salt, alias)>] (D3)
            let session_token =
                crate::token::mint_session_token(category, &stable_alias, session_salt);

            positional_to_token.insert(positional_token.to_string(), session_token.clone());
            new_entries.insert(session_token.clone(), original_value.to_string());
            session_map.insert(session_token, original_value.to_string());
        }

        // 4. Replace positional tokens with session tokens in the redacted text.
        //    Positional tokens never reach the LLM (SPEC §11.5).
        let mut pseudonymized_text = redacted_text;
        for (pos_token, session_token) in &positional_to_token {
            pseudonymized_text = pseudonymized_text.replace(pos_token, session_token);
        }

        // 5. Store the updated session map + salt in the vault
        let final_payload = crate::session_vault::SessionPayload {
            map: session_map,
            salt: session_salt.clone(),
        };
        self.vault.store_session(session_id, final_payload)?;

        // 6. Persist registry changes
        self.registry.flush()?;

        Ok(PseudonymizedPrompt {
            text: pseudonymized_text,
            rehydration_map: new_entries,
            session_id: session_id.to_string(),
        })
    }
}

/// The pseudonymizer seen through the port the host publishes.
///
/// The port is shared, so the pseudonymizer behind it is guarded: it
/// mutates the identity registry and the session vault while it works.
#[cfg(feature = "basemind")]
pub struct BasemindPromptPseudonymizer {
    inner: tokio::sync::Mutex<PromptPseudonymizer>,
}

#[cfg(feature = "basemind")]
impl BasemindPromptPseudonymizer {
    pub fn new(pseudonymizer: PromptPseudonymizer) -> Self {
        Self {
            inner: tokio::sync::Mutex::new(pseudonymizer),
        }
    }
}

#[cfg(feature = "basemind")]
#[async_trait::async_trait]
impl alex_runtime_ports::PromptPseudonymizationPort for BasemindPromptPseudonymizer {
    async fn pseudonymize(
        &self,
        prompt: &str,
        session_id: &str,
        workspace_root: &str,
    ) -> Result<
        alex_runtime_ports::PseudonymizedPrompt,
        alex_runtime_ports::PromptPseudonymizationError,
    > {
        let mut guard = self.inner.lock().await;
        PromptPseudonymizer::pseudonymize(&mut guard, prompt, session_id, workspace_root)
            .await
            .map(
                |pseudonymized| alex_runtime_ports::PseudonymizedPrompt {
                    text: pseudonymized.text,
                    rehydration_map: pseudonymized.rehydration_map,
                    session_id: pseudonymized.session_id,
                },
            )
            .map_err(|error| {
                alex_runtime_ports::PromptPseudonymizationError::new(
                    alex_runtime_ports::PromptPseudonymizationErrorKind::Failed,
                    error.to_string(),
                )
            })
    }
}
