# Identity registry: per-workspace value→alias store

`Type: task`
`Status: resolved`

## Question

Build the persistent, vault-encrypted registry that maps a hash of the real PII value to a stable per-workspace alias.

## Deliverable

**File:** `basemind-integration/src/identity_registry.rs`

```rust
pub struct IdentityRegistry { ... }

impl IdentityRegistry {
    pub fn new(workspace_data_dir: PathBuf) -> Result<Self> { ... }  // fetches passphrase from keychain
    pub fn compute_value_hash(value: &str) -> String { ... }  // SHA-256 hex
    pub fn get_or_create_alias(&mut self, value_hash: &str, category: &str) -> String { ... }
    pub fn get_or_create_alias_for_value(&mut self, value: &str, category: &str) -> String { ... }
    pub fn resolve_alias(&self, alias: &str) -> Option<String> { ... }
    pub fn flush(&self) -> Result<()> { ... }
}
```

**Storage format:** JSON `{ "version": 1, "entries": { "value_hash": "alias", ... } }` encrypted via `vault_encrypt` (XPII: AES-256-GCM + scrypt).

**Key management:** Passphrase from OS keychain via `crate::keychain::get_or_create_vault_passphrase()`.

**Scope:** Per-workspace — `workspace_data_dir/identity_registry.enc`. Different workspaces → different files → no cross-workspace correlation.

## Answer

**Implemented** in `src/crates/services/basemind-integration/src/identity_registry.rs`:

- `IdentityRegistry::new(workspace_data_dir)` — fetches passphrase from OS keychain (tickets 07, 08)
- `compute_value_hash(value)` — SHA-256 hex of raw PII value
- `get_or_create_alias(value_hash, category)` — returns `ENT_<CAT>_<hash8>`, creates if new
- `get_or_create_alias_for_value(value, category)` — convenience: hashes value then gets alias
- `resolve_alias(alias)` → `Option<value_hash>` for rehydration
- `flush()` — persists encrypted store to `workspace_data_dir/identity_registry.enc`

**Alias format:** `ENT_PERSON_a7f3` (category + first 8 hex of value_hash). Per-workspace isolation: different workspaces have different registry files.

**Dependencies**: Uses `crate::vault` (XPII) and `crate::keychain` (passphrase). Compiles with `keychain-macos` feature.