//! Persistent SSE task with exponential backoff and refresh-on-401.

use std::sync::Arc;
use std::time::{Duration, Instant};

use futures_util::StreamExt;
use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use eventsource_client::{Client as _, ClientBuilder, SSE};

use crate::api::client::ApiClient;
use crate::api::types::ArtyEvent;
use crate::api::{user_agent, API_BASE_URL};

const MAX_BACKOFF_SECS: u64 = 30;
const STABLE_RESET_SECS: u64 = 60;

#[derive(Default)]
pub struct SseManager {
    inner: Mutex<Option<JoinHandle<()>>>,
}

impl SseManager {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// Start (or restart) the SSE task for the given instance.
    pub async fn connect(
        self: Arc<Self>,
        app: AppHandle,
        client: ApiClient,
        instance_id: String,
    ) {
        // Stop any existing task.
        self.disconnect().await;

        let app_for_task = app.clone();
        let handle = tokio::spawn(async move {
            run(app_for_task, client, instance_id).await;
        });

        *self.inner.lock().await = Some(handle);
    }

    pub async fn disconnect(&self) {
        if let Some(h) = self.inner.lock().await.take() {
            h.abort();
        }
    }
}

async fn run(app: AppHandle, client: ApiClient, instance_id: String) {
    let mut backoff = 1u64;

    loop {
        let token = match client.tokens.get().await {
            Some(b) => b.access_token.clone(),
            None => {
                let _ = app.emit("auth-required", ());
                return;
            }
        };

        let url = format!(
            "{API_BASE_URL}/api/overlay/instances/{instance_id}/events"
        );

        let sse = ClientBuilder::for_url(&url)
            .and_then(|b| Ok(b
                .header("Authorization", &format!("Bearer {token}"))?
                .header("User-Agent", &user_agent())?
                .build()));

        let sse = match sse {
            Ok(c) => c,
            Err(e) => {
                let _ = app.emit("arty-sse-error", &format!("build: {e}"));
                return;
            }
        };

        let mut stream = sse.stream();
        let connected_at = Instant::now();
        let mut got_anything = false;

        let _ = app.emit("arty-sse-status", "connecting");

        let mut auth_failed = false;
        while let Some(item) = stream.next().await {
            match item {
                Ok(SSE::Event(ev)) => {
                    got_anything = true;
                    let _ = app.emit(
                        "arty-sse",
                        &ArtyEvent {
                            kind: ev.event_type.clone(),
                            data: serde_json::from_str(&ev.data)
                                .unwrap_or(serde_json::Value::String(ev.data)),
                        },
                    );

                    // Reset backoff if the connection has been stable.
                    if connected_at.elapsed().as_secs() >= STABLE_RESET_SECS {
                        backoff = 1;
                    }
                }
                Ok(SSE::Comment(_)) => {
                    got_anything = true;
                }
                Ok(SSE::Connected(_)) => {
                    let _ = app.emit("arty-sse-status", "connected");
                }
                Err(eventsource_client::Error::UnexpectedResponse(resp, _)) => {
                    if resp.status() == 401 {
                        auth_failed = true;
                    }
                    break;
                }
                Err(_) => break,
            }
        }

        // If we ran some time without errors, give backoff a head start.
        if got_anything && connected_at.elapsed().as_secs() >= STABLE_RESET_SECS {
            backoff = 1;
        }

        let _ = app.emit("arty-sse-status", "disconnected");

        if auth_failed {
            // Try refresh once by issuing any authenticated request.
            // Easiest: re-check tokens; the next `me` will trigger refresh.
            match client.get::<serde_json::Value>("/api/overlay/me").await {
                Ok(_) => {
                    backoff = 1;
                    continue;
                }
                Err(_) => {
                    let _ = app.emit("auth-required", ());
                    return;
                }
            }
        }

        let delay = Duration::from_secs(backoff);
        tokio::time::sleep(delay).await;
        backoff = (backoff * 2).min(MAX_BACKOFF_SECS);
    }
}
