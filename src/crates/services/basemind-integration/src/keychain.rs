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

    #[cfg(target_os = "windows")]
    {
        // Use DPAPI via CryptUnprotectData - simplified, would need proper implementation
        Err(KeychainError::Unavailable(
            "Windows DPAPI not fully implemented".into(),
        ))
    }

    #[cfg(target_os = "linux")]
    {
        // Simplified - would need async SecretService
        Err(KeychainError::Unavailable(
            "Linux secret-service not fully implemented".into(),
        ))
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        Err(KeychainError::Unavailable(
            "keychain not available on this platform".into(),
        ))
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

    #[cfg(target_os = "windows")]
    {
        Err(KeychainError::Unavailable(
            "Windows DPAPI not fully implemented".into(),
        ))
    }

    #[cfg(target_os = "linux")]
    {
        Err(KeychainError::Unavailable(
            "Linux secret-service not fully implemented".into(),
        ))
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        Err(KeychainError::Unavailable(
            "keychain not available on this platform".into(),
        ))
    }
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
