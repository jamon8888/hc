//! Vault encryption/decryption — XPII wire format (magic + scrypt + AES-256-GCM).
//!
//! Delegates to basemind's vault implementation via MCP when available, or provides
//! a local implementation for offline use.

#[cfg(feature = "basemind")]
use anyhow::Result;
#[cfg(feature = "basemind")]
use base64::Engine;
#[cfg(feature = "basemind")]
use rand::RngCore;
#[cfg(feature = "basemind")]
use serde::{Deserialize, Serialize, de::DeserializeOwned};
#[cfg(feature = "basemind")]
use thiserror::Error;

#[cfg(feature = "basemind")]
#[derive(Debug, Error)]
pub enum VaultError {
    #[error("encryption failed: {0}")]
    Encrypt(String),
    #[error("decryption failed: {0}")]
    Decrypt(String),
    #[error("serialization failed: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("base64 decode failed: {0}")]
    Base64(#[from] base64::DecodeError),
}

#[cfg(feature = "basemind")]
const XPII_MAGIC: &[u8] = b"XPII";

#[cfg(feature = "basemind")]
/// Encrypt a serializable value with a passphrase (XPII wire format).
pub fn vault_encrypt<T: Serialize>(value: &T, passphrase: &str) -> Result<String, VaultError> {
    let plaintext = serde_json::to_vec(value)?;
    let encrypted = xpcc_encrypt(&plaintext, passphrase)?;
    Ok(base64::engine::general_purpose::STANDARD.encode(encrypted))
}

#[cfg(feature = "basemind")]
/// Decrypt an XPII-encrypted blob with a passphrase.
pub fn vault_decrypt<T: DeserializeOwned>(
    encrypted_b64: &str,
    passphrase: &str,
) -> Result<T, VaultError> {
    let encrypted = base64::engine::general_purpose::STANDARD.decode(encrypted_b64)?;
    let plaintext = xpcc_decrypt(&encrypted, passphrase)?;
    Ok(serde_json::from_slice(&plaintext)?)
}

#[cfg(feature = "basemind")]
fn xpcc_encrypt(plaintext: &[u8], passphrase: &str) -> Result<Vec<u8>, VaultError> {
    use aes_gcm::aead::{Aead, KeyInit};
    use aes_gcm::{Aes256Gcm, Key, Nonce};
    use rand::RngCore;
    use scrypt::{Params, scrypt};

    // Derive key via scrypt
    let mut key = [0u8; 32];
    let params = Params::RECOMMENDED;
    scrypt(passphrase.as_bytes(), b"openbitfun-salt", &params, &mut key)
        .map_err(|e| VaultError::Encrypt(format!("scrypt: {e}")))?;

    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key));

    // Generate random nonce
    let mut nonce_bytes = [0u8; 12];
    let mut rng = rand::rngs::OsRng;
    rng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    // Encrypt
    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| VaultError::Encrypt(format!("aes-gcm: {e}")))?;

    // XPII wire format: magic(4) + nonce(12) + ciphertext
    let mut out = Vec::with_capacity(4 + 12 + ciphertext.len());
    out.extend_from_slice(XPII_MAGIC);
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

#[cfg(feature = "basemind")]
fn xpcc_decrypt(encrypted: &[u8], passphrase: &str) -> Result<Vec<u8>, VaultError> {
    use aes_gcm::aead::{Aead, KeyInit};
    use aes_gcm::{Aes256Gcm, Key, Nonce};
    use scrypt::{Params, scrypt};

    if encrypted.len() < 4 + 12 {
        return Err(VaultError::Decrypt("too short".into()));
    }
    if &encrypted[..4] != XPII_MAGIC {
        return Err(VaultError::Decrypt("bad magic".into()));
    }

    let nonce = Nonce::from_slice(&encrypted[4..16]);
    let ciphertext = &encrypted[16..];

    let mut key = [0u8; 32];
    let params = Params::RECOMMENDED;
    scrypt(passphrase.as_bytes(), b"openbitfun-salt", &params, &mut key)
        .map_err(|e| VaultError::Decrypt(format!("scrypt: {e}")))?;

    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key));

    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| VaultError::Decrypt(format!("aes-gcm: {e}")))?;

    Ok(plaintext)
}
