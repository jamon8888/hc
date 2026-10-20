//! Session token minting — opaque per-session PII tokens (D3 clarified).
//!
//! `token = [<CATEGORY>_<H16>]` where `H16` is the first 16 hex characters of
//! `HMAC-SHA256(key = session salt, message = workspace alias)`. The token is
//! stable within a session (same entity, same token) and different in every
//! other session, and it reveals neither the alias nor the salt: it can be
//! reversed only through the session vault's `token -> value` map — there is
//! no strip-the-salt arithmetic (D3/D4, SPEC §11.5).

/// Mint the session token for a workspace alias under the session salt.
#[cfg(feature = "basemind")]
pub fn mint_session_token(category: &str, alias: &str, session_salt: &str) -> String {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    let mut mac = Hmac::<Sha256>::new_from_slice(session_salt.as_bytes())
        .expect("HMAC-SHA256 accepts keys of any length");
    mac.update(alias.as_bytes());
    let digest = hex::encode(mac.finalize().into_bytes());
    format!("[{}_{}]", category.to_uppercase(), &digest[..16])
}
