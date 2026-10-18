//! Basemind vault encryption/decryption — AES-GCM for rehydration maps.

#[cfg(feature = "basemind")]
use aes_gcm::{Aes256Gcm, Key, Nonce, KeyInit};
#[cfg(feature = "basemind")]
use aes_gcm::aead::Aead;
#[cfg(feature = "basemind")]
use rand::RngCore;
#[cfg(feature = "basemind")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "basemind")]
use anyhow::{anyhow, Result};
#[cfg(feature = "basemind")]
use std::collections::HashMap;

/// Encrypted vault blob structure.
#[cfg(feature = "basemind")]
#[derive(Debug, Serialize, Deserialize)]
pub struct EncryptedVaultBlob {
    pub nonce: String,  // base64 encoded
    pub ciphertext: String, // base64 encoded
}

/// Encrypt a rehydration map using AES-256-GCM.
/// Key is derived from the Basemind configuration.
#[cfg(feature = "basemind")]
pub fn vault_encrypt(map: &HashMap<String, String>) -> Result<String> {
    let plaintext = serde_json::to_vec(map)
        .map_err(|e| anyhow!("Failed to serialize rehydration map: {}", e))?;
    
    let key: [u8; 32] = get_vault_key()?;
    let cipher = Aes256Gcm::new(aes_gcm::Key::<Aes256Gcm>::from_slice(&key));
    
    // Generate random nonce
    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    
    let ciphertext = cipher.encrypt(nonce, plaintext.as_ref())
        .map_err(|e| anyhow!("Failed to encrypt vault data: {:?}", e))?;
    
    let blob = EncryptedVaultBlob {
        nonce: base64::encode(&nonce_bytes),
        ciphertext: base64::encode(&ciphertext),
    };
    
    let json = serde_json::to_string(&blob)
        .map_err(|e| anyhow!("Failed to serialize encrypted vault: {}", e))?;
    
    Ok(json)
}

/// Decrypt a rehydration map from an encrypted vault blob.
#[cfg(feature = "basemind")]
pub fn vault_decrypt(encrypted_json: &str) -> Result<HashMap<String, String>> {
    let blob: EncryptedVaultBlob = serde_json::from_str(encrypted_json)
        .map_err(|e| anyhow!("Failed to parse encrypted vault blob: {}", e))?;
    
    let nonce_bytes = base64::decode(&blob.nonce)
        .map_err(|e| anyhow!("Failed to decode nonce: {}", e))?;
    let ciphertext = base64::decode(&blob.ciphertext)
        .map_err(|e| anyhow!("Failed to decode ciphertext: {}", e))?;
    
    let key: [u8; 32] = get_vault_key()?;
    let cipher = Aes256Gcm::new(aes_gcm::Key::<Aes256Gcm>::from_slice(&key));
    let nonce = Nonce::from_slice(&nonce_bytes);
    
    let plaintext = cipher.decrypt(nonce, ciphertext.as_ref())
        .map_err(|e| anyhow!("Failed to decrypt vault data: {:?}", e))?;
    
    let map = serde_json::from_slice(&plaintext)
        .map_err(|e| anyhow!("Failed to deserialize rehydration map: {}", e))?;
    
    Ok(map)
}

/// Get the vault encryption key from configuration.
/// In production, this comes from Basemind's config (derived from master key).
#[cfg(feature = "basemind")]
fn get_vault_key() -> Result<[u8; 32]> {
    // TODO: Get key from Basemind config / environment
    // For now, use a fixed key (REPLACE IN PRODUCTION)
    let key_str = std::env::var("BASEMIND_VAULT_KEY")
        .unwrap_or_else(|_| "0000000000000000000000000000000000000000000000000000000000000000".to_string());
    
    let mut key = [0u8; 32];
    if key_str.len() >= 64 {
        // Hex decode
        for i in 0..32 {
            let byte_str = &key_str[i*2..i*2+2];
            key[i] = u8::from_str_radix(byte_str, 16)
                .map_err(|e| anyhow!("Invalid hex in vault key: {}", e))?;
        }
    } else {
        // Use as-is (padded/truncated)
        key[..key_str.len().min(32)].copy_from_slice(&key_str.as_bytes()[..key_str.len().min(32)]);
    }
    
    Ok(key)
}

/// Base64 encoding helper.
#[cfg(feature = "basemind")]
mod base64 {
    pub fn encode(bytes: &[u8]) -> String {
        const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut result = String::with_capacity(bytes.len() * 4 / 3 + 4);
        let mut i = 0;
        while i + 2 < bytes.len() {
            let b1 = bytes[i];
            let b2 = bytes[i + 1];
            let b3 = bytes[i + 2];
            result.push(TABLE[(b1 >> 2) as usize] as char);
            result.push(TABLE[((b1 & 0x03) << 4 | b2 >> 4) as usize] as char);
            result.push(TABLE[((b2 & 0x0F) << 2 | b3 >> 6) as usize] as char);
            result.push(TABLE[(b3 & 0x3F) as usize] as char);
            i += 3;
        }
        match bytes.len() - i {
            1 => {
                let b1 = bytes[i];
                result.push(TABLE[(b1 >> 2) as usize] as char);
                result.push(TABLE[((b1 & 0x03) << 4) as usize] as char);
                result.push('=');
                result.push('=');
            }
            2 => {
                let b1 = bytes[i];
                let b2 = bytes[i + 1];
                result.push(TABLE[(b1 >> 2) as usize] as char);
                result.push(TABLE[((b1 & 0x03) << 4 | b2 >> 4) as usize] as char);
                result.push(TABLE[((b2 & 0x0F) << 2) as usize] as char);
                result.push('=');
            }
            _ => {}
        }
        result
    }
    
    pub fn decode(s: &str) -> Result<Vec<u8>, String> {
        let s = s.trim_end_matches('=');
        let mut result = Vec::with_capacity(s.len() * 3 / 4);
        let mut buf = 0u32;
        let mut bits = 0;
        
        for ch in s.chars() {
            let val = match ch {
                'A'..='Z' => ch as u32 - 'A' as u32,
                'a'..='z' => ch as u32 - 'a' as u32 + 26,
                '0'..='9' => ch as u32 - '0' as u32 + 52,
                '+' => 62,
                '/' => 63,
                _ => continue,
            };
            buf = (buf << 6) | val;
            bits += 6;
            if bits >= 8 {
                bits -= 8;
                result.push((buf >> bits) as u8);
            }
        }
        Ok(result)
    }
}