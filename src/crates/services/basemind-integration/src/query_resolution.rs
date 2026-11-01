//! Query resolution — reverses echoed session tokens through the session vault
//! before RAG search, and re-tokenizes search hits back into session tokens for
//! the LLM (D6 amended: the index stores raw text, so the vault is the only
//! token inversion and the LLM boundary is the single chokepoint, I1).

#[cfg(feature = "basemind")]
use anyhow::Result;
#[cfg(feature = "basemind")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "basemind")]
use std::cmp::Reverse;
#[cfg(feature = "basemind")]
use std::collections::BTreeMap;
#[cfg(feature = "basemind")]
use std::sync::{Arc, RwLock};

#[cfg(feature = "basemind")]
pub struct QueryResolver {
    client: crate::ipc::BasemindClient,
    registry: crate::identity_registry::IdentityRegistry,
    vault: Arc<RwLock<crate::session_vault::SessionVault>>,
}

/// Synchronous, session-vault-only access for DOC references.
///
/// RAG searches hold their tool mutex across awaited sidecar calls, but Read
/// can resolve a DOC token concurrently with that work. This handle shares the
/// resolver's vault lock without acquiring the search mutex or cloning an
/// independently synchronized vault.
#[cfg(feature = "basemind")]
#[derive(Clone)]
pub struct DocumentTokenResolver {
    vault: Arc<RwLock<crate::session_vault::SessionVault>>,
}

#[cfg(feature = "basemind")]
impl QueryResolver {
    pub fn new(
        client: crate::ipc::BasemindClient,
        registry: crate::identity_registry::IdentityRegistry,
        vault: crate::session_vault::SessionVault,
    ) -> Self {
        Self {
            client,
            registry,
            vault: Arc::new(RwLock::new(vault)),
        }
    }

    /// Return a synchronized handle that resolves only document references.
    pub(crate) fn document_token_resolver(&self) -> DocumentTokenResolver {
        DocumentTokenResolver {
            vault: Arc::clone(&self.vault),
        }
    }

    fn load_session(&self, session_id: &str) -> Result<crate::session_vault::SessionPayload> {
        let vault = self.vault.read().unwrap_or_else(|error| error.into_inner());
        vault
            .load_session(session_id)
            .map_err(|error| anyhow::anyhow!("session payload unavailable: {error}"))
    }

    fn store_session(
        &self,
        session_id: &str,
        payload: crate::session_vault::SessionPayload,
    ) -> Result<()> {
        let vault = self
            .vault
            .write()
            .unwrap_or_else(|error| error.into_inner());
        vault
            .store_session(session_id, payload)
            .map_err(|error| anyhow::anyhow!("session payload unavailable: {error}"))
    }

    /// Rewrite a user's query for the **raw** index (D6 amended).
    ///
    /// Model context only ever contains session tokens (SPEC §11) while the
    /// index stores raw text, so every token the model echoed back is reversed
    /// to its raw value through the session vault map: the token is an HMAC
    /// (D3), it has no recoverable suffix, and the vault is the only inversion.
    /// Tokens with no vault entry pass through unchanged (already opaque,
    /// nothing leaks); ordinary search terms pass through unchanged.
    ///
    /// The resolved query stays local — loopback MCP to the sidecar — the same
    /// trust boundary as the index itself. A missing or unreadable session
    /// payload hard-fails the call (D7).
    pub fn rewrite_query(&self, raw_query: &str, session_id: &str) -> Result<String> {
        let session = self.load_session(session_id)?;

        // Tokens are fixed-shape so none can contain another; longest-first
        // keeps replacement order irrelevant anyway.
        let mut tokens: Vec<(&String, &String)> = session.map.iter().collect();
        tokens.sort_by_key(|(token, _)| Reverse(token.len()));

        let mut rewritten = raw_query.to_string();
        for (token, value) in tokens {
            if rewritten.contains(token.as_str()) {
                rewritten = rewritten.replace(token.as_str(), value);
            }
        }
        Ok(rewritten)
    }

    /// Resolve a document token for an internal file operation. Only the DOC
    /// namespace is accepted here so this seam cannot be used to expose the
    /// session's ordinary PII rehydration map.
    pub fn resolve_document_token(&self, token: &str, session_id: &str) -> Result<Option<String>> {
        self.document_token_resolver()
            .resolve_document_token(token, session_id)
    }
}

#[cfg(feature = "basemind")]
impl DocumentTokenResolver {
    pub fn resolve_document_token(&self, token: &str, session_id: &str) -> Result<Option<String>> {
        if !(token.starts_with("[DOC_") && token.ends_with(']')) {
            return Ok(None);
        }
        let vault = self.vault.read().unwrap_or_else(|error| error.into_inner());
        let session = vault
            .load_session(session_id)
            .map_err(|error| anyhow::anyhow!("session payload unavailable: {error}"))?;
        Ok(session.map.get(token).cloned())
    }
}

#[cfg(feature = "basemind")]
impl QueryResolver {
    /// Re-tokenize raw RAG hits into session tokens before they can reach the LLM.
    ///
    /// Candidate values per hit are `hit.entities[].text` (scan-time detection)
    /// **union** the `detections[]` of a `redact_text` safety pass over the raw
    /// chunk (catches regex-class PII and scans where NER was off — I1 must not
    /// depend on index configuration), processed longest-first so a value
    /// nested in another (email inside a URL) is replaced outermost-first.
    ///
    /// For each value: mint or reuse the stable workspace alias in the identity
    /// registry (exactly like the prompt flow), mint the session token
    /// `[CAT_<HMAC16(salt, alias)>]` (D3), replace the raw value inside the
    /// chunk, and upsert `token -> value` into the session vault so the UI
    /// render boundary can rehydrate it (D4). The workspace-stable alias is
    /// deliberately absent from the returned payload: hits are model context,
    /// and a visible alias would link this session to every other session of
    /// the workspace (D3).
    ///
    /// Fails hard when the session payload cannot be loaded, the `redact_text`
    /// safety pass fails, or a store cannot be persisted (D7); raw entity text
    /// never leaves this function.
    pub async fn retokenize_hits(
        &mut self,
        hits: Vec<crate::ipc::DocumentSearchHit>,
        session_id: &str,
        workspace_root: &str,
    ) -> Result<Vec<RetokenizedHit>> {
        let mut session = self.load_session(session_id)?;
        let salt = session.salt.clone();

        let mut registry_changed = false;
        let mut vault_changed = false;
        let mut retokenized = Vec::new();

        for hit in hits {
            // Safety pass over the raw chunk; failure hard-fails the call (D7).
            // The same D7 rule covers a truncated NER pass: a hit whose text
            // was only partly read must not reach the model, because the PII
            // past the cut would enter the redacted answer unchecked.
            let redact_result = self.client.redact_text(&hit.text, workspace_root).await?;
            if let Some(message) = crate::redaction::ner_coverage_refusal(&redact_result) {
                anyhow::bail!("redact_text safety pass refused a hit: {message}");
            }
            let detections = redact_result["detections"]
                .as_array()
                .ok_or_else(|| anyhow::anyhow!("missing detections in redact_text safety pass"))?;

            // Candidate values: safety-pass detections and scan-time entities,
            // deduplicated by value (detection order first, then entity order).
            let mut candidates: BTreeMap<String, String> = BTreeMap::new();
            for detection in detections {
                let value = detection["text"].as_str().unwrap_or("");
                let category = detection["category"].as_str().unwrap_or("UNKNOWN");
                if !value.is_empty() {
                    candidates.insert(value.to_string(), category.to_string());
                }
            }
            for entity in &hit.entities {
                if !entity.text.is_empty() {
                    candidates.insert(entity.text.clone(), entity.category.clone());
                }
            }
            let mut candidates: Vec<(String, String)> = candidates.into_iter().collect();
            // Longest first: a value nested inside another (email inside a URL)
            // is replaced outermost-first so later matches still line up.
            candidates.sort_by_key(|(value, _)| Reverse(value.len()));

            let mut hit_text = hit.text.clone();
            let mut hit_entities = Vec::new();

            for (value, category) in candidates {
                let before = self.registry.entry_count();
                let alias = self
                    .registry
                    .get_or_create_alias_for_value(&value, &category);
                registry_changed |= self.registry.entry_count() > before;
                let token = crate::token::mint_session_token(&category, &alias, &salt);

                // Replace the raw value inside the chunk (I1 chokepoint)
                hit_text = hit_text.replace(value.as_str(), &token);

                if session.map.get(&token) != Some(&value) {
                    session.map.insert(token.clone(), value.clone());
                    vault_changed = true;
                }

                hit_entities.push(RetokenizedEntity { category, token });
            }

            // File paths are also model-visible content. Keep the source
            // reference useful for correlation within this session, but do
            // not expose a user's directory or filename to the model. The
            // original path remains available only through the encrypted
            // session vault, alongside ordinary PII rehydration values.
            let source_doc = if hit.path.is_empty() {
                "[DOC_UNKNOWN]".to_string()
            } else {
                let before = self.registry.entry_count();
                self.registry
                    .get_or_create_alias_for_value(&hit.path, "DOC");
                registry_changed |= self.registry.entry_count() > before;
                let token = crate::token::mint_session_token("DOC", &hit.path, &salt);
                if session.map.get(&token) != Some(&hit.path) {
                    session.map.insert(token.clone(), hit.path.clone());
                    vault_changed = true;
                }
                token
            };

            retokenized.push(RetokenizedHit {
                text: hit_text,
                source_doc,
                score: 1.0 - hit.distance,
                entities: hit_entities,
            });
        }

        if registry_changed {
            self.registry.flush()?;
        }
        if vault_changed {
            self.store_session(session_id, session)?;
        }

        Ok(retokenized)
    }
}

#[cfg(feature = "basemind")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetokenizedHit {
    pub text: String, // chunk text with every raw value replaced by a session token
    /// Session-scoped opaque document token; the raw path stays in the
    /// encrypted session vault and never reaches the model.
    pub source_doc: String,
    pub score: f32,
    pub entities: Vec<RetokenizedEntity>,
}

#[cfg(feature = "basemind")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetokenizedEntity {
    pub category: String,
    /// Session token `[CAT_<HMAC16(salt, alias)>]`; the alias is deliberately
    /// absent from the payload (D3).
    pub token: String,
}
