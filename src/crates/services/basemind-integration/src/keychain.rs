//! OS keychain integration for vault passphrase.
//!
//! Retrieves/stores the passphrase in the platform keychain. Never argv, never disk.

#[cfg(feature = "basemind")]
use anyhow::Result;
#[cfg(feature = "basemind")]
use base64::Engine;
#[cfg(feature = "basemind")]
use rand::RngCore;
#[cfg(feature = "basemind")]
use thiserror::Error;

#[cfg(feature = "basemind")]
#[derive(Debug, Error)]
pub enum KeychainError {
    #[error("keychain unavailable: {0}")]
    Unavailable(String),
    #[error("item not found")]
    NotFound,
    #[error("keychain operation failed: {0}")]
    Failed(String),
}

#[cfg(feature = "basemind")]
const SERVICE: &str = "openbitfun";
#[cfg(feature = "basemind")]
const ACCOUNT: &str = "basemind-vault";

#[cfg(feature = "basemind")]
pub fn get_vault_passphrase() -> Result<String, KeychainError> {
    #[cfg(target_os = "macos")]
    {
        use security_framework::passwords::get_generic_password;
        use security_framework_sys::base::errSecItemNotFound;

        match get_generic_password(SERVICE, ACCOUNT) {
            Ok(password) => Ok(String::from_utf8(password)
                .map_err(|_| KeychainError::Failed("invalid UTF-8".into()))?),
            Err(e) if e.code() == errSecItemNotFound => Err(KeychainError::NotFound),
            Err(e) => Err(KeychainError::Failed(e.to_string())),
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        match open_entry()?.get_secret() {
            Ok(secret) => {
                String::from_utf8(secret).map_err(|_| KeychainError::Failed("invalid UTF-8".into()))
            }
            Err(keyring_core::Error::NoEntry) => Err(KeychainError::NotFound),
            Err(error) => Err(KeychainError::Failed(error.to_string())),
        }
    }
}

#[cfg(feature = "basemind")]
pub fn set_vault_passphrase(passphrase: &str) -> Result<(), KeychainError> {
    #[cfg(target_os = "macos")]
    {
        use security_framework::passwords::set_generic_password;

        set_generic_password(SERVICE, ACCOUNT, passphrase.as_bytes())
            .map_err(|e| KeychainError::Failed(e.to_string()))?;
        Ok(())
    }

    #[cfg(not(target_os = "macos"))]
    {
        open_entry()?
            .set_secret(passphrase.as_bytes())
            .map_err(|error| KeychainError::Failed(error.to_string()))
    }
}

/// The credential-store entry the vault passphrase lives in.
///
/// macOS uses the Keychain directly. The other platforms go through the
/// credential store the rest of the product already uses: the Windows
/// Credential Manager and the Linux Secret Service, through `keyring-core`.
#[cfg(all(feature = "basemind", not(target_os = "macos")))]
fn open_entry() -> Result<keyring_core::Entry, KeychainError> {
    use std::sync::{Mutex, OnceLock};

    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    let _guard = LOCK.get_or_init(|| Mutex::new(())).lock();

    if keyring_core::get_default_store().is_none() {
        #[cfg(target_os = "windows")]
        let store = windows_native_keyring_store::Store::new();
        #[cfg(all(
            unix,
            not(any(target_os = "macos", target_os = "ios", target_os = "android"))
        ))]
        let store = zbus_secret_service_keyring_store::Store::new();
        #[cfg(not(any(
            target_os = "windows",
            all(
                unix,
                not(any(target_os = "macos", target_os = "ios", target_os = "android"))
            )
        )))]
        let store: keyring_core::Result<std::sync::Arc<keyring_core::CredentialStore>> =
            Err(keyring_core::Error::NoDefaultStore);

        let store = store.map_err(|error| {
            KeychainError::Unavailable(format!("initialize system credential store: {error}"))
        })?;
        keyring_core::set_default_store(store);
    }

    keyring_core::Entry::new(SERVICE, ACCOUNT)
        .map_err(|error| KeychainError::Unavailable(format!("open credential entry: {error}")))
}

#[cfg(feature = "basemind")]
pub fn get_or_create_vault_passphrase() -> Result<String, KeychainError> {
    match get_vault_passphrase() {
        Ok(p) => Ok(p),
        Err(KeychainError::NotFound) => {
            // Generate new 256-bit passphrase
            let mut bytes = [0u8; 32];
            let mut rng = rand::rngs::OsRng;
            rng.fill_bytes(&mut bytes);
            let passphrase = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes);
            set_vault_passphrase(&passphrase)?;
            Ok(passphrase)
        }
        Err(e) => Err(e),
    }
}
