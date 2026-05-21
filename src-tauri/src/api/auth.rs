//! Authentication flows: OAuth loopback (Discord), local credentials, logout.

use std::net::TcpListener;
use std::sync::Arc;
use std::time::Duration;

use rand::RngCore;
use tauri::{AppHandle, Emitter};
use tiny_http::{Header, Response, Server};
use tokio::sync::Mutex;

use crate::api::client::{chrono_now_secs, parse_error, ApiError, ApiClient};
use crate::api::types::{TokenBundle, TokenResponse};
use crate::api::{user_agent, API_BASE_URL};

const OAUTH_TIMEOUT_SECS: u64 = 300; // 5 min

/// Single-shot OAuth flow. Spawns a loopback server, opens the system
/// browser, and emits `auth-success` / `auth-cancelled` to the frontend.
///
/// Holds a global Mutex to prevent concurrent flows.
pub async fn discord_oauth_start(
    app: AppHandle,
    client: ApiClient,
    flow_lock: Arc<Mutex<()>>,
) -> Result<(), String> {
    // Refuse concurrent flows.
    let guard = match flow_lock.try_lock_owned() {
        Ok(g) => g,
        Err(_) => return Err("OAuth flow already in progress".into()),
    };

    // 1. Bind loopback on a random port.
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|e| format!("bind: {e}"))?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    let server = Server::from_listener(listener, None)
        .map_err(|e| format!("tiny_http: {e}"))?;

    // 2. Client state (CSPRNG, hex).
    let mut state_bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut state_bytes);
    let client_state = hex_encode(&state_bytes);

    let redirect_uri = format!("http://127.0.0.1:{port}/callback");

    // 3. Hit /api/overlay/auth/start ourselves so the UA gate sees the
    //    overlay's UA (not the browser's). We must NOT follow the redirect.
    let no_redirect = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .user_agent(user_agent())
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| format!("reqwest build: {e}"))?;

    let start_url = format!(
        "{API_BASE_URL}/api/overlay/auth/start?redirect_uri={}&state={}",
        url_encode(&redirect_uri),
        url_encode(&client_state),
    );
    let resp = no_redirect
        .get(&start_url)
        .send()
        .await
        .map_err(|e| format!("auth/start: {e}"))?;

    if resp.status().as_u16() != 302 {
        let status = resp.status().as_u16();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("auth/start returned {status}: {body}"));
    }

    let discord_url = resp
        .headers()
        .get(reqwest::header::LOCATION)
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| "auth/start: missing Location header".to_string())?
        .to_string();

    // 4. Open the Discord authorize URL in the system browser.
    open_in_browser(&app, &discord_url)?;

    // 5. Wait for the loopback callback.
    let result = tokio::task::spawn_blocking(move || -> Result<TokenBundle, String> {
        run_loopback(server, &client_state)
    })
    .await
    .map_err(|e| format!("join error: {e}"))?;

    drop(guard); // release the flow lock

    match result {
        Ok(bundle) => {
            client
                .tokens
                .store(bundle)
                .await
                .map_err(|e| format!("store tokens: {e}"))?;
            let _ = app.emit("auth-success", ());
            Ok(())
        }
        Err(e) => {
            let _ = app.emit("auth-cancelled", &e);
            Err(e)
        }
    }
}

/// Run the loopback server until the fragment is posted back. Synchronous
/// (kept in a `spawn_blocking` task by the caller).
fn run_loopback(server: Server, expected_state: &str) -> Result<TokenBundle, String> {
    let deadline = std::time::Instant::now() + Duration::from_secs(OAUTH_TIMEOUT_SECS);

    loop {
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        if remaining.is_zero() {
            return Err("OAuth timeout".into());
        }

        let req = match server.recv_timeout(remaining) {
            Ok(Some(r)) => r,
            Ok(None) => continue,
            Err(e) => return Err(format!("loopback recv: {e}")),
        };

        let path = req.url().split('?').next().unwrap_or("").to_string();
        let method = req.method().clone();

        match (method.as_str(), path.as_str()) {
            ("GET", "/callback") => {
                let html = CALLBACK_HTML.as_bytes();
                let _ = req.respond(
                    Response::from_data(html).with_header(
                        Header::from_bytes(&b"Content-Type"[..], &b"text/html; charset=utf-8"[..])
                            .unwrap(),
                    ),
                );
            }
            ("POST", "/finish") => {
                let mut body = String::new();
                {
                    let mut r = req;
                    let _ = r.as_reader().read_to_string(&mut body);
                    let _ = r.respond(
                        Response::from_string("ok").with_header(
                            Header::from_bytes(&b"Content-Type"[..], &b"text/plain"[..]).unwrap(),
                        ),
                    );
                }

                // Body is the URL fragment without the leading '#'.
                let bundle = parse_fragment(&body, expected_state)?;
                return Ok(bundle);
            }
            _ => {
                let _ = req.respond(Response::from_string("not found").with_status_code(404));
            }
        }
    }
}

fn parse_fragment(fragment: &str, expected_state: &str) -> Result<TokenBundle, String> {
    let frag = fragment.trim_start_matches('#');

    let mut access = None;
    let mut refresh = None;
    let mut access_exp: i64 = 900;
    let mut refresh_exp: i64 = 2_592_000;
    let mut state = None;

    for pair in frag.split('&') {
        let mut it = pair.splitn(2, '=');
        let k = it.next().unwrap_or("");
        let v = it.next().unwrap_or("");
        let v = url_decode(v);
        match k {
            "access_token"               => access = Some(v),
            "refresh_token"              => refresh = Some(v),
            "access_token_expires_in"    => access_exp = v.parse().unwrap_or(900),
            "refresh_token_expires_in"   => refresh_exp = v.parse().unwrap_or(2_592_000),
            "state"                      => state = Some(v),
            _ => {}
        }
    }

    let state = state.ok_or_else(|| "missing state".to_string())?;
    if state != expected_state {
        return Err("state mismatch".into());
    }

    let now = chrono_now_secs();
    Ok(TokenBundle {
        access_token:       access.ok_or_else(|| "missing access_token".to_string())?,
        refresh_token:      refresh.ok_or_else(|| "missing refresh_token".to_string())?,
        access_expires_at:  now + access_exp,
        refresh_expires_at: now + refresh_exp,
    })
}

/// Local credentials login.
pub async fn local_login(
    app: AppHandle,
    client: ApiClient,
    pseudo: String,
    password: String,
) -> Result<(), String> {
    let url = format!("{API_BASE_URL}/api/overlay/auth/login");
    let resp = client
        .raw()
        .post(&url)
        .json(&serde_json::json!({ "pseudo": pseudo, "password": password }))
        .send()
        .await
        .map_err(|e| format!("network: {e}"))?;

    if !resp.status().is_success() {
        let err = parse_error(resp).await;
        return Err(format_api_error(&err));
    }

    let tr: TokenResponse = resp
        .json()
        .await
        .map_err(|e| format!("json: {e}"))?;
    let now = chrono_now_secs();
    let bundle = TokenBundle {
        access_token:       tr.access_token,
        refresh_token:      tr.refresh_token,
        access_expires_at:  now + tr.access_token_expires_in,
        refresh_expires_at: now + tr.refresh_token_expires_in,
    };
    client.tokens.store(bundle).await?;
    let _ = app.emit("auth-success", ());
    Ok(())
}

/// Best-effort server logout + local wipe.
pub async fn logout(app: AppHandle, client: ApiClient) -> Result<(), String> {
    if let Some(bundle) = client.tokens.get().await {
        let url = format!("{API_BASE_URL}/api/overlay/auth/logout");
        let _ = tokio::time::timeout(
            Duration::from_secs(2),
            client
                .raw()
                .post(&url)
                .bearer_auth(&bundle.access_token)
                .send(),
        )
        .await;
    }
    client.tokens.clear().await;
    let _ = app.emit("auth-cleared", ());
    Ok(())
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn url_encode(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}

fn url_decode(s: &str) -> String {
    url::form_urlencoded::parse(s.as_bytes())
        .next()
        .map(|(k, _)| k.into_owned())
        .unwrap_or_else(|| s.to_string())
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0xf) as usize] as char);
    }
    out
}

fn format_api_error(err: &ApiError) -> String {
    match err {
        ApiError::Api { code, message, .. } => match code.as_str() {
            "OVERLAY_LOGIN_FAILED"        => "Identifiants invalides.".into(),
            "OVERLAY_ACCOUNT_DISABLED"    => "Compte désactivé.".into(),
            "OVERLAY_USER_AGENT_REJECTED" => "Cette version de l'overlay n'est pas autorisée par le serveur.".into(),
            _ => format!("{code}: {message}"),
        },
        ApiError::Transport(s)  => format!("Erreur réseau: {s}"),
        ApiError::Unauthenticated => "Authentification requise.".into(),
    }
}

fn open_in_browser(app: &AppHandle, url: &str) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|e| format!("open browser: {e}"))
}

const CALLBACK_HTML: &str = r#"<!doctype html>
<html lang="fr">
<head>
<meta charset="utf-8">
<title>11e Overlay — Authentification</title>
<style>
  body { font-family: system-ui, sans-serif; background: #15161b; color: #dcdcdc;
         display: flex; align-items: center; justify-content: center;
         height: 100vh; margin: 0; }
  .card { padding: 24px 32px; border: 1px solid #2a2c36; border-radius: 8px;
          background: #1c1d24; text-align: center; max-width: 380px; }
  h1 { font-size: 16px; margin: 0 0 8px; color: #c8a84b; }
  p  { font-size: 13px; margin: 6px 0; }
</style>
</head>
<body>
<div class="card">
  <h1>Authentification</h1>
  <p id="msg">Finalisation en cours…</p>
</div>
<script>
  (async () => {
    try {
      const frag = location.hash.slice(1);
      await fetch('/finish', { method: 'POST', body: frag });
      document.getElementById('msg').textContent =
        'Authentification réussie. Vous pouvez fermer cet onglet.';
    } catch (e) {
      document.getElementById('msg').textContent = 'Erreur: ' + e;
    }
  })();
</script>
</body>
</html>"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_fragment_ok() {
        let frag = "access_token=AT&refresh_token=RT&access_token_expires_in=600\
                    &refresh_token_expires_in=1000&state=abc";
        let b = parse_fragment(frag, "abc").unwrap();
        assert_eq!(b.access_token, "AT");
        assert_eq!(b.refresh_token, "RT");
    }

    #[test]
    fn parse_fragment_state_mismatch() {
        let frag = "access_token=AT&refresh_token=RT&state=xxx";
        assert!(parse_fragment(frag, "abc").is_err());
    }
}
