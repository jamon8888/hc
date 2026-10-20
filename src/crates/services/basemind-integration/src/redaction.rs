//! Document redaction session — the product-owned half of a redaction.
//!
//! The basemind fork redacts a document and answers with the redacted text,
//! a `rehydration_map` (token -> original value) and the `detections` it
//! found. It has no reveal tool, and it must not grow one: ADR 0001 puts the
//! PII identity layer in the product, because the fork's own `entity_id` is a
//! per-file span id and carries no cross-document identity.
//!
//! So this module keeps what the fork does not:
//!
//! - the token -> value map, encrypted at rest through the session vault, so
//!   a reveal can resolve a token back to the original value;
//! - the entity id, minted from a hash of the real value so the same value
//!   carries the same id across documents and workspaces;
//! - the span markup the web UI needs to render a click-to-reveal badge.
//!
//! The redacted markdown the fork returns carries bare tokens such as
//! `[EMAIL_1]`. The UI reveals an entity by id, so the tokens are rewritten
//! into spans that carry the entity id.

#[cfg(feature = "basemind")]
use crate::identity_registry::IdentityRegistry;
#[cfg(feature = "basemind")]
use anyhow::Result;
#[cfg(feature = "basemind")]
use serde_json::Value;
#[cfg(feature = "basemind")]
use std::collections::BTreeMap;
#[cfg(feature = "basemind")]
use std::path::PathBuf;
#[cfg(feature = "basemind")]
use std::sync::Mutex;

/// A redacted document plus the values needed to reveal its entities.
#[cfg(feature = "basemind")]
#[derive(Debug, Clone)]
pub struct RedactedDocument {
    /// The redacted markdown, with entity spans substituted for the fork's
    /// bare tokens.
    pub markdown: String,
    /// The entities found, with the product-minted entity id.
    pub entities: Vec<crate::models::RedactedEntity>,
    /// The source format the fork extracted (`pdf`, `docx`, ...).
    pub source_format: String,
}

/// The product's rehydration state for one workspace.
///
/// Holds the token -> value map for the documents redacted in this
/// workspace. The map is persisted encrypted through the session vault, so a
/// reveal still works after a restart, and the entity id for a value is
/// stable because it is derived from a hash of the value itself.
#[cfg(feature = "basemind")]
pub struct RedactionStore {
    /// token -> original value, for the documents redacted in this workspace.
    values: Mutex<BTreeMap<String, String>>,
    /// entity id -> original value, for reveal.
    by_entity: Mutex<BTreeMap<String, String>>,
    /// Mints entity ids from a hash of the real value.
    registry: Mutex<IdentityRegistry>,
    /// Encrypted at-rest store for the token -> value map.
    vault: crate::session_vault::SessionVault,
    /// The vault key this workspace's map is stored under.
    scope: String,
}

#[cfg(feature = "basemind")]
impl RedactionStore {
    /// Open the store for a workspace.
    ///
    /// `workspace_data_dir` holds the encrypted identity registry,
    /// `vault_dir` the encrypted rehydration map. `scope` names this
    /// workspace's entry in the vault.
    pub fn new(workspace_data_dir: PathBuf, vault_dir: PathBuf, scope: String) -> Result<Self> {
        let registry = IdentityRegistry::new(workspace_data_dir)?;
        let vault = crate::session_vault::SessionVault::new(vault_dir)?;
        let store = Self {
            values: Mutex::new(BTreeMap::new()),
            by_entity: Mutex::new(BTreeMap::new()),
            registry: Mutex::new(registry),
            vault,
            scope,
        };
        store.load_persisted()?;
        Ok(store)
    }

    /// Open the store for a workspace with an explicit passphrase.
    ///
    /// The passphrase is the one the OS keychain would otherwise supply; the
    /// tests use it so they never touch the keychain.
    pub fn with_passphrase(
        workspace_data_dir: PathBuf,
        vault_dir: PathBuf,
        scope: String,
        passphrase: String,
    ) -> Result<Self> {
        let registry = IdentityRegistry::with_passphrase(workspace_data_dir, passphrase.clone())?;
        let vault = crate::session_vault::SessionVault::with_passphrase(vault_dir, passphrase);
        let store = Self {
            values: Mutex::new(BTreeMap::new()),
            by_entity: Mutex::new(BTreeMap::new()),
            registry: Mutex::new(registry),
            vault,
            scope,
        };
        store.load_persisted()?;
        Ok(store)
    }

    /// Restore the persisted token -> value map.
    fn load_persisted(&self) -> Result<()> {
        let payload = match self.vault.load_session(&self.scope) {
            Ok(payload) => payload,
            // A workspace that has never redacted a document has no vault
            // entry yet; that is not an error.
            Err(_) => return Ok(()),
        };
        let mut values = self.values.lock().unwrap_or_else(|e| e.into_inner());
        values.extend(payload.map);
        Ok(())
    }

    /// Persist the token -> value map.
    fn persist(&self) -> Result<()> {
        let values = self
            .values
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        self.vault.store_session(
            &self.scope,
            crate::session_vault::SessionPayload {
                map: values,
                salt: String::new(),
            },
        )
    }

    /// Record a redaction result and return the document to show the user.
    ///
    /// `payload` is the fork's `redact_text` result: `redacted_text`,
    /// `rehydration_map` and `detections`. Each token in the rehydration map
    /// is given an entity id minted from a hash of its value, so the same
    /// value keeps the same id everywhere it appears.
    pub fn record(&self, payload: &Value, source_format: &str) -> RedactedDocument {
        let redacted_text = payload["redacted_text"].as_str().unwrap_or("").to_string();
        let rehydration = payload["rehydration_map"]
            .as_object()
            .cloned()
            .unwrap_or_default();

        // Count the occurrences per entity id so the UI can show "3 PII" per
        // type, and collect the token each id was minted from.
        let mut counts: BTreeMap<String, (String, u32)> = BTreeMap::new();
        let mut token_for_entity: BTreeMap<String, String> = BTreeMap::new();

        {
            let mut values = self.values.lock().unwrap_or_else(|e| e.into_inner());
            let mut by_entity = self.by_entity.lock().unwrap_or_else(|e| e.into_inner());
            let mut registry = self.registry.lock().unwrap_or_else(|e| e.into_inner());

            for (token, value) in &rehydration {
                let original = value.as_str().unwrap_or_default();
                if original.is_empty() {
                    continue;
                }
                let category = category_of(token);
                let entity_id = registry.get_or_create_alias_for_value(original, &category);
                values.insert(token.clone(), original.to_string());
                by_entity.insert(entity_id.clone(), original.to_string());
                token_for_entity.insert(entity_id.clone(), token.clone());
                let entry = counts.entry(entity_id.clone()).or_insert((category, 0));
                entry.1 += 1;
            }

            // Persist the identity registry so the ids stay stable across
            // runs. The rehydration map is persisted below, outside the
            // locks, because persisting it takes the same lock.
            let _ = registry.flush();
        }

        // Persist the rehydration map so a later reveal resolves, including
        // after a restart. This must happen without the value lock held.
        let _ = self.persist();

        let entities = counts
            .into_iter()
            .map(
                |(entity_id, (entity_type, occurrence_count))| crate::models::RedactedEntity {
                    entity_id,
                    entity_type,
                    occurrence_count,
                },
            )
            .collect();

        let markdown = rewrite_tokens_as_spans(&redacted_text, &token_for_entity);

        RedactedDocument {
            markdown,
            entities,
            source_format: source_format.to_string(),
        }
    }

    /// Reveal the original value behind an entity id.
    ///
    /// Returns `None` when the id is unknown to this workspace, so an id
    /// from another workspace cannot be resolved here.
    pub fn reveal(&self, entity_id: &str) -> Option<String> {
        let by_entity = self.by_entity.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(value) = by_entity.get(entity_id) {
            return Some(value.clone());
        }
        // The id may have been minted in an earlier run: the registry knows
        // the value hash, but not the value, so the value must come from the
        // persisted map, which is keyed by token.
        drop(by_entity);
        let token = self.token_for_entity(entity_id)?;
        let values = self.values.lock().unwrap_or_else(|e| e.into_inner());
        values.get(&token).cloned()
    }

    /// Find the token an entity id was minted from.
    fn token_for_entity(&self, entity_id: &str) -> Option<String> {
        let values = self.values.lock().unwrap_or_else(|e| e.into_inner());
        // The persisted map is token -> value, and the registry maps a value
        // hash to the alias, so an alias resolves to a token by matching the
        // hash of the stored value.
        let registry = self.registry.lock().unwrap_or_else(|e| e.into_inner());
        values
            .iter()
            .find(|(_, value)| {
                registry
                    .resolve_alias(entity_id)
                    .is_some_and(|hash| hash == IdentityRegistry::compute_value_hash(value))
            })
            .map(|(token, _)| token.clone())
    }
}

/// The pure helpers the crate's own tests exercise.
#[cfg(feature = "basemind")]
pub mod testable {
    pub use super::{category_of, rewrite_tokens_as_spans};
}

/// The category of a token, e.g. `[EMAIL_1]` -> `email`.
#[cfg(feature = "basemind")]
pub fn category_of(token: &str) -> String {
    token
        .trim_start_matches('[')
        .trim_end_matches(']')
        .rsplit_once('_')
        .map(|(category, _)| category.to_lowercase())
        .unwrap_or_else(|| "unknown".to_string())
}

/// Rewrite the fork's bare tokens into spans the UI can reveal.
///
/// The fork's redacted text carries tokens such as `[EMAIL_1]`. The web UI
/// renders a click-to-reveal badge for a span that carries `data-entity-type`
/// and `data-entity-id`, so each token becomes such a span. The token text
/// stays the visible label, so the redacted document still reads as redacted.
#[cfg(feature = "basemind")]
pub fn rewrite_tokens_as_spans(text: &str, token_for_entity: &BTreeMap<String, String>) -> String {
    let mut out = text.to_string();
    for (entity_id, token) in token_for_entity {
        let category = category_of(token);
        out = out.replace(
            token,
            &format!(
                "<span data-entity-type=\"{category}\" data-entity-id=\"{entity_id}\">{token}</span>"
            ),
        );
    }
    out
}
