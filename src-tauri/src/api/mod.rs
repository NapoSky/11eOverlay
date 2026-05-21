//! ArtyCon overlay API integration.
//!
//! Public surface:
//! - [`ApiClient`] — shared HTTP client (built once, holds tokens & UA).
//! - [`auth`] — OAuth loopback + local login + logout + status.
//! - [`sse`] — persistent Server-Sent Events task.
//! - [`tokens`] — encrypted token storage (settings.json field).
//! - [`types`] — wire types mirrored from the API spec.

pub mod auth;
pub mod client;
pub mod crypto;
pub mod sse;
pub mod tokens;
pub mod types;

pub use client::ApiClient;

/// Base URL of the ArtyCon API.
///
/// Hardcoded for production; can be overridden at compile time via the
/// `ARTYCON_API_BASE` environment variable (handy for dev/staging).
pub const API_BASE_URL: &str = match option_env!("ARTYCON_API_BASE") {
    Some(v) => v,
    None => "https://artycon.11e-foxhole.com",
};

/// Build the stable User-Agent string required by the API.
/// Format: `ArtyConOverlay/<semver> (<platform>)`.
pub fn user_agent() -> String {
    format!(
        "ArtyConOverlay/{} ({}-{})",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
        std::env::consts::ARCH,
    )
}
