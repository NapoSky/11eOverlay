//! In-memory + encrypted-at-rest token store.
//!
//! Tokens are persisted as a single AES-GCM blob in `AppSettings.encrypted_tokens`
//! (string field in `11eOverlay.json` next to the executable). No keyring, no
//! separate file — the application binary stays portable.
//!
//! On every successful read, `TokenBundle` is dropped via `ZeroizeOnDrop` to
//! wipe the plaintext from RAM.

use std::sync::Arc;
use tokio::sync::RwLock;

use crate::api::crypto;
use crate::api::types::TokenBundle;
use crate::settings::{load_settings, save_settings};

/// Thread-safe in-memory cache. The disk source of truth is
/// `AppSettings.encrypted_tokens`.
#[derive(Clone, Default)]
pub struct TokenStore {
    inner: Arc<RwLock<Option<TokenBundle>>>,
}

impl TokenStore {
    /// Build a store and pre-populate it by decrypting the on-disk blob
    /// (if any). Decryption errors are silently treated as "no tokens".
    pub fn load_from_disk() -> Self {
        let store = Self::default();
        let settings = load_settings();
        if let Some(blob) = settings.encrypted_tokens.as_deref() {
            if let Ok(plain) = crypto::decrypt(blob) {
                if let Ok(bundle) = serde_json::from_slice::<TokenBundle>(&plain) {
                    *store.inner.blocking_write() = Some(bundle);
                }
            }
        }
        store
    }

    pub async fn get(&self) -> Option<TokenBundle> {
        self.inner.read().await.clone()
    }

    pub async fn is_authenticated(&self) -> bool {
        self.inner.read().await.is_some()
    }

    /// Persist the bundle: encrypts it and writes the blob into
    /// `AppSettings.encrypted_tokens`. Also updates the in-memory cache.
    pub async fn store(&self, bundle: TokenBundle) -> Result<(), String> {
        let plain = serde_json::to_vec(&bundle).map_err(|e| e.to_string())?;
        let blob = crypto::encrypt(&plain)?;

        // Persist to settings.json
        let mut settings = load_settings();
        settings.encrypted_tokens = Some(blob);
        save_settings(&settings);

        *self.inner.write().await = Some(bundle);
        Ok(())
    }

    /// Wipe both memory and the on-disk encrypted blob.
    pub async fn clear(&self) {
        *self.inner.write().await = None;
        let mut settings = load_settings();
        if settings.encrypted_tokens.is_some() {
            settings.encrypted_tokens = None;
            save_settings(&settings);
        }
    }

    /// Synchronous variant used by the exit hook (must not block on async
    /// runtime). Wipes the disk blob only — memory will be freed by Drop.
    pub fn clear_disk_sync() {
        let mut settings = load_settings();
        if settings.encrypted_tokens.is_some() {
            settings.encrypted_tokens = None;
            save_settings(&settings);
        }
    }
}
