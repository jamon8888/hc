# Session vault: per-session rehydration map + salt storage

`Type: task`
`Status: resolved`

## Question

Build the vault that stores per-session rehydration maps (token → original value) and the session salt, encrypted with a passphrase from the OS keychain.

## Deliverable

**File:** `basemind-integration/src/session_vault.rs`

```rust
pub struct SessionVault {
    vault_dir: PathBuf,
    passphrase: String, // from OS keychain
}

impl SessionVault {
    pub fn new(vault_dir: PathBuf) -> Result<Self> { ... }  // fetches passphrase from keychain
    pub fn store_session(&self, session_id: &str, payload: SessionPayload) -> Result<()> { ... }
    pub fn load_session(&self, session_id: &str) -> Result<SessionPayload> { ... }
    pub fn delete_session(&self, session_id: &str) -> Result<()> { ... }
    pub fn generate_salt() -> String { ... }  // fresh 128-bit CSPRNG
}

struct SessionPayload {
    map: BTreeMap<String, String>, // token → original value (D3: keyed by the full HMAC token)
    salt: String,                  // hex-encoded per-session salt
}
```

**Encryption:** Delegates to `crate::vault::vault_encrypt/decrypt` (XPII: AES-256-GCM + scrypt). Passphrase from OS keychain via `crate::keychain::get_or_create_vault_passphrase()`.

**Storage:** `workspace_data_dir/sessions/{session_id}.vault` — one file per session.

**Rehydration at UI boundary:** `load_session(session_id)` → returns `map` + `salt`. Renderer:
1. Replace each session token in the displayed text with `map[token]` — the map is keyed by the **full** token `[CAT_<H16>]`; there is no suffix-stripping and no reverse alias lookup (the token is an HMAC and is not decomposable, D3 clarified)
2. Longest token first, so adjacent or nested tokens replace cleanly
3. Tokens absent from the map are left unchanged (nothing to reveal)

## Answer

**Implemented** in `src/crates/services/basemind-integration/src/session_vault.rs`:

- `SessionVault::new(vault_dir)` — fetches passphrase from OS keychain (ticket 08)
- `store_session()` / `load_session()` / `delete_session()` — encrypted file per session
- `generate_salt()` — 128-bit CSPRNG via `rand::OsRng`, hex-encoded
- `SessionPayload` — `map` (token → original) + `salt` (hex), both `Serialize`/`Deserialize`

**Dependencies**: Uses `crate::vault` (XPII wire format) and `crate::keychain` (passphrase). Compiles with `keychain-macos` feature.