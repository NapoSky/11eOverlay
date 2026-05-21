//! Shared HTTP client with mutex-guarded refresh interceptor.

use std::sync::Arc;
use tokio::sync::Mutex;

use reqwest::{header, Method, StatusCode};
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::api::tokens::TokenStore;
use crate::api::types::{ApiErrorEnvelope, TokenResponse};
use crate::api::{user_agent, API_BASE_URL};

/// Shared client.
///
/// Cloning is cheap (all fields are `Arc` or `Clone`).
#[derive(Clone)]
pub struct ApiClient {
    http:    reqwest::Client,
    pub tokens:  TokenStore,
    /// Held during refresh so concurrent expired-token responses don't all
    /// hit `/auth/refresh` in parallel.
    refresh_lock: Arc<Mutex<()>>,
}

#[derive(Debug)]
pub enum ApiError {
    /// Network / serialization / unexpected failure.
    Transport(String),
    /// API returned a structured error envelope.
    Api {
        #[allow(dead_code)]
        status: u16,
        code: String,
        message: String,
    },
    /// Authentication is required (refresh failed or no tokens).
    Unauthenticated,
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Transport(s) => write!(f, "transport: {s}"),
            Self::Api { code, message, .. } => write!(f, "{code}: {message}"),
            Self::Unauthenticated => write!(f, "unauthenticated"),
        }
    }
}

impl ApiClient {
    pub fn new(tokens: TokenStore) -> Result<Self, String> {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::USER_AGENT,
            header::HeaderValue::from_str(&user_agent())
                .map_err(|e| format!("invalid UA: {e}"))?,
        );

        let http = reqwest::Client::builder()
            .default_headers(headers)
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| format!("reqwest build: {e}"))?;

        Ok(Self {
            http,
            tokens,
            refresh_lock: Arc::new(Mutex::new(())),
        })
    }

    pub fn raw(&self) -> &reqwest::Client {
        &self.http
    }

    /// Authenticated GET. Auto-refreshes on `401 OVERLAY_TOKEN_EXPIRED`.
    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, ApiError> {
        self.request::<(), T>(Method::GET, path, None).await
    }

    /// Authenticated POST with JSON body.
    #[allow(dead_code)]
    pub async fn post<B: Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, ApiError> {
        self.request::<B, T>(Method::POST, path, Some(body)).await
    }

    /// Authenticated POST returning no body.
    #[allow(dead_code)]
    pub async fn post_no_content<B: Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<(), ApiError> {
        let token = self.access_token().await.ok_or(ApiError::Unauthenticated)?;
        let url = format!("{API_BASE_URL}{path}");

        let mut resp = self
            .http
            .post(&url)
            .bearer_auth(&token)
            .json(body)
            .send()
            .await
            .map_err(|e| ApiError::Transport(e.to_string()))?;

        if resp.status() == StatusCode::UNAUTHORIZED {
            if self.try_refresh_for(&token).await? {
                let t = self.access_token().await.ok_or(ApiError::Unauthenticated)?;
                resp = self
                    .http
                    .post(&url)
                    .bearer_auth(&t)
                    .json(body)
                    .send()
                    .await
                    .map_err(|e| ApiError::Transport(e.to_string()))?;
            }
        }

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(parse_error(resp).await)
        }
    }

    async fn request<B: Serialize, T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        body: Option<&B>,
    ) -> Result<T, ApiError> {
        let token = self.access_token().await.ok_or(ApiError::Unauthenticated)?;
        let url = format!("{API_BASE_URL}{path}");

        let send = |t: &str| {
            let mut req = self.http.request(method.clone(), &url).bearer_auth(t);
            if let Some(b) = body {
                req = req.json(b);
            }
            req.send()
        };

        let mut resp = send(&token)
            .await
            .map_err(|e| ApiError::Transport(e.to_string()))?;

        if resp.status() == StatusCode::UNAUTHORIZED {
            if self.try_refresh_for(&token).await? {
                let t = self.access_token().await.ok_or(ApiError::Unauthenticated)?;
                resp = send(&t)
                    .await
                    .map_err(|e| ApiError::Transport(e.to_string()))?;
            }
        }

        if resp.status().is_success() {
            let bytes = resp
                .bytes()
                .await
                .map_err(|e| ApiError::Transport(e.to_string()))?;
            serde_json::from_slice::<T>(&bytes).map_err(|e| {
                let preview: String = String::from_utf8_lossy(&bytes)
                    .chars()
                    .take(300)
                    .collect();
                ApiError::Transport(format!("json: {e} | body: {preview}"))
            })
        } else {
            Err(parse_error(resp).await)
        }
    }

    async fn access_token(&self) -> Option<String> {
        self.tokens.get().await.map(|b| b.access_token.clone())
    }

    /// Returns `Ok(true)` if a refresh succeeded and a retry is appropriate.
    /// Returns `Err(Unauthenticated)` if refresh definitively failed —
    /// tokens are wiped in that case.
    async fn try_refresh_for(&self, observed_access: &str) -> Result<bool, ApiError> {
        let _g = self.refresh_lock.lock().await;

        // Another concurrent task may have already refreshed.
        let current = self.tokens.get().await;
        if let Some(b) = &current {
            if b.access_token != observed_access {
                return Ok(true);
            }
        }

        let refresh = match current.as_ref() {
            Some(b) => b.refresh_token.clone(),
            None => return Err(ApiError::Unauthenticated),
        };

        let url = format!("{API_BASE_URL}/api/overlay/auth/refresh");
        let resp = self
            .http
            .post(&url)
            .json(&serde_json::json!({ "refresh_token": refresh }))
            .send()
            .await
            .map_err(|e| ApiError::Transport(e.to_string()))?;

        if !resp.status().is_success() {
            // Wipe everything: this token chain is dead.
            self.tokens.clear().await;
            return Err(ApiError::Unauthenticated);
        }

        let tr: TokenResponse = resp
            .json()
            .await
            .map_err(|e| ApiError::Transport(format!("refresh json: {e}")))?;
        let now = chrono_now_secs();
        let bundle = crate::api::types::TokenBundle {
            access_token:        tr.access_token,
            refresh_token:       tr.refresh_token,
            access_expires_at:   now + tr.access_token_expires_in,
            refresh_expires_at:  now + tr.refresh_token_expires_in,
        };
        self.tokens.store(bundle).await.map_err(ApiError::Transport)?;
        Ok(true)
    }
}

pub(crate) async fn parse_error(resp: reqwest::Response) -> ApiError {
    let status = resp.status().as_u16();
    if let Ok(env) = resp.json::<ApiErrorEnvelope>().await {
        ApiError::Api {
            status,
            code: env.error.code,
            message: env.error.message,
        }
    } else {
        ApiError::Api {
            status,
            code: "UNKNOWN".into(),
            message: format!("HTTP {status}"),
        }
    }
}

pub(crate) fn chrono_now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
