//! Token-at-rest encryption (AES-256-GCM, machine-bound key).
//!
//! Tokens are stored in `11eOverlay.json` (next to the executable) as a
//! base64-encoded blob produced by [`encrypt`]. The encryption key is
//! derived from the machine's stable ID via HKDF-SHA256, so a copy of
//! `settings.json` on another machine cannot be decrypted.

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use base64::Engine;
use hkdf::Hkdf;
use rand::RngCore;
use sha2::Sha256;

/// Build-time salt — changing this rotates *all* on-disk tokens.
const APP_SALT: &[u8] = b"11eOverlay/v1/aes-gcm-key";
const KDF_INFO: &[u8] = b"11eOverlay/overlay-tokens/v1";

/// Returns a stable per-machine identifier.
///
/// Tries `machine-uid` first (Linux `/etc/machine-id`, Windows MachineGuid,
/// macOS IOPlatformUUID). Falls back to a hostname+username hash if the
/// platform call fails (rare: minimal containers, locked-down systems).
fn machine_id() -> Vec<u8> {
    if let Ok(uid) = machine_uid::get() {
        return uid.into_bytes();
    }

    // Degraded fallback. Not as stable, but better than failing outright.
    let hostname = hostname::get()
        .ok()
        .and_then(|s| s.into_string().ok())
        .unwrap_or_else(|| "unknown-host".into());
    let user = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "unknown-user".into());
    format!("fallback::{hostname}::{user}").into_bytes()
}

/// Derive a 32-byte AES-256 key from the machine ID.
fn derive_key() -> [u8; 32] {
    let ikm = machine_id();
    let hk = Hkdf::<Sha256>::new(Some(APP_SALT), &ikm);
    let mut okm = [0u8; 32];
    hk.expand(KDF_INFO, &mut okm)
        .expect("HKDF expand: 32 bytes always fits");
    okm
}

/// Encrypt arbitrary plaintext bytes. Output: base64(nonce_12 || ciphertext_with_tag).
pub fn encrypt(plaintext: &[u8]) -> Result<String, String> {
    let key_bytes = derive_key();
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key_bytes));

    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| format!("encrypt failed: {e}"))?;

    let mut out = Vec::with_capacity(12 + ciphertext.len());
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&ciphertext);
    Ok(base64::engine::general_purpose::STANDARD.encode(out))
}

/// Decrypt a blob produced by [`encrypt`]. Returns `Err` if the blob is
/// truncated, the key has changed, or the tag fails to verify.
pub fn decrypt(blob: &str) -> Result<Vec<u8>, String> {
    let raw = base64::engine::general_purpose::STANDARD
        .decode(blob.as_bytes())
        .map_err(|e| format!("base64 decode: {e}"))?;
    if raw.len() < 12 + 16 {
        return Err("ciphertext too short".into());
    }

    let (nonce_bytes, ciphertext) = raw.split_at(12);
    let key_bytes = derive_key();
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key_bytes));
    cipher
        .decrypt(Nonce::from_slice(nonce_bytes), ciphertext)
        .map_err(|e| format!("decrypt failed: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let pt = b"hello world";
        let blob = encrypt(pt).unwrap();
        let back = decrypt(&blob).unwrap();
        assert_eq!(back, pt);
    }

    #[test]
    fn tampered_fails() {
        let mut blob = encrypt(b"secret").unwrap();
        // flip a character somewhere in the body
        let bytes = unsafe { blob.as_bytes_mut() };
        if let Some(b) = bytes.get_mut(20) {
            *b = b'A';
        }
        assert!(decrypt(&blob).is_err());
    }
}
