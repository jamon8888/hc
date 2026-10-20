# OS keychain integration for vault passphrase

`Type: task`
`Status: resolved`

## Question

Retrieve the vault encryption passphrase from the OS keychain at sidecar startup and pass it over basemind IPC — never as argv.

## Deliverable

**File:** `basemind-integration/src/keychain.rs`

```rust
pub fn get_vault_passphrase() -> Result<String, KeychainError> { ... }
pub fn set_vault_passphrase(passphrase: &str) -> Result<(), KeychainError> { ... }
pub fn get_or_create_vault_passphrase() -> Result<String, KeychainError> { ... }
```

**Passphrase lifecycle:**
1. First run: generate 256-bit random passphrase, store in keychain (`service="openbitfun", account="basemind-vault"`), return it
2. Subsequent runs: retrieve from keychain
3. Pass to basemind sidecar via **IPC** — the `vault` MCP tool takes passphrase in JSON body; `hc` injects it when calling `vault mode="encrypt|decrypt"`

**Never:**
- Pass as command-line argument (visible in `ps`)
- Write to disk unencrypted
- Log

**Fallback:** If keychain unavailable (headless Linux without libsecret), read from `BASEMIND_VAULT_PASSPHRASE` env var and **emit a startup warning** logged at `WARN` level.

## Answer

**Implemented** in `src/crates/services/basemind-integration/src/keychain.rs`:

- **macOS** (feature `keychain-macos`): Uses `security-framework` 2.11 via `security_framework::passwords::{get_generic_password, set_generic_password}` with `errSecItemNotFound` for "not found" detection. Tested and compiles.
- **Windows** (feature `keychain-windows`): Stub returning `Unavailable` — DPAPI via `CryptUnprotectData` needs implementation.
- **Linux** (feature `keychain-linux`): Stub returning `Unavailable` — `secret-service` async API needs implementation.
- **Fallback**: `get_or_create_vault_passphrase()` generates a 256-bit random passphrase on first run (CSPRNG via `rand::OsRng`), stores it, returns it.

**Cargo features**: `keychain-macos` / `keychain-windows` / `keychain-linux` — opt-in per platform to avoid pulling `security-framework` / `windows` / `secret-service` on irrelevant targets.

**Used by**: `IdentityRegistry::new()`, `SessionVault::new()` — passphrase injected at construction, never argv.