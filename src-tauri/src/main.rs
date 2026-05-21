// Hide the console window in release builds on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod api;
mod content;
mod settings;

use api::auth as api_auth;
use api::sse::SseManager;
use api::tokens::TokenStore;
use api::types::{AuthStatus, Instance, Layer, OverlayUser, Snapshot};
use api::ApiClient;
use content::{load_content, ContentSection};
use settings::{load_settings, merge_settings, save_settings, AppSettings};

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{
    Emitter, Manager, PhysicalPosition, PhysicalSize, RunEvent, State, WebviewUrl,
    WebviewWindowBuilder, WindowEvent,
};
use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
use tokio::sync::Mutex as AsyncMutex;

struct AppState {
    settings:   Mutex<AppSettings>,
    api:        ApiClient,
    sse:        Arc<SseManager>,
    oauth_lock: Arc<AsyncMutex<()>>,
}

// ── Existing commands ─────────────────────────────────────────────────────────

#[tauri::command]
fn get_settings(state: State<'_, AppState>) -> AppSettings {
    state.settings.lock().unwrap().clone()
}

#[tauri::command]
fn get_content() -> Vec<ContentSection> {
    load_content()
}

#[tauri::command]
fn save_settings_cmd(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
    patch: serde_json::Value,
) -> Result<AppSettings, String> {
    let updated = {
        let mut s = state.settings.lock().map_err(|e| e.to_string())?;
        let prev_hotkey = s.hotkey.clone();
        *s = merge_settings(s.clone(), patch);
        save_settings(&s);

        if s.hotkey != prev_hotkey {
            app.global_shortcut()
                .unregister_all()
                .map_err(|e| e.to_string())?;
            register_hotkeys(&app, &s).map_err(|e| e.to_string())?;
        }

        s.clone()
    };

    if let Some(win) = app.get_webview_window("overlay") {
        let _ = win.emit("settings-updated", &updated);
    }

    Ok(updated)
}

#[tauri::command]
fn open_settings_window(app: tauri::AppHandle) {
    if let Some(win) = app.get_webview_window("settings") {
        let _ = win.set_focus();
        return;
    }
    let _ = WebviewWindowBuilder::new(
        &app,
        "settings",
        WebviewUrl::App("settings/index.html".into()),
    )
    .title("Paramètres — 11e Overlay")
    .inner_size(440.0, 360.0)
    .transparent(true)
    .decorations(false)
    .always_on_top(true)
    .resizable(false)
    .build();
}

#[tauri::command]
fn hide_overlay_cmd(app: tauri::AppHandle) {
    if let Some(win) = app.get_webview_window("overlay") {
        let _ = win.hide();
    }
    if let Some(tray) = app.tray_by_id("main-tray") {
        let _ = tray.set_visible(true);
    }
}

#[tauri::command]
fn quit_app(app: tauri::AppHandle) {
    app.exit(0);
}

// ── New: auth commands ───────────────────────────────────────────────────────

#[tauri::command]
async fn auth_status(state: State<'_, AppState>) -> Result<AuthStatus, String> {
    if !state.api.tokens.is_authenticated().await {
        return Ok(AuthStatus::default());
    }
    match state.api.get::<OverlayUser>("/api/overlay/me").await {
        Ok(u) => Ok(AuthStatus {
            authenticated: true,
            pseudo: Some(u.pseudo),
            role:   Some(u.role),
        }),
        Err(api::client::ApiError::Unauthenticated) => Ok(AuthStatus::default()),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
async fn auth_discord_start(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let api = state.api.clone();
    let lock = state.oauth_lock.clone();
    drop(state);
    api_auth::discord_oauth_start(app, api, lock).await
}

#[tauri::command]
async fn auth_local_login(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    pseudo: String,
    password: String,
) -> Result<(), String> {
    let api = state.api.clone();
    drop(state);
    api_auth::local_login(app, api, pseudo, password).await
}

#[tauri::command]
async fn auth_logout(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let api = state.api.clone();
    let sse = state.sse.clone();
    drop(state);
    sse.disconnect().await;
    api_auth::logout(app, api).await
}

// ── New: artillery read commands ──────────────────────────────────────────────

#[tauri::command]
async fn arty_list_instances(state: State<'_, AppState>) -> Result<Vec<Instance>, String> {
    state
        .api
        .get::<Vec<Instance>>("/api/overlay/instances")
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn arty_list_layers(
    state: State<'_, AppState>,
    instance_id: String,
) -> Result<Vec<Layer>, String> {
    state
        .api
        .get::<Vec<Layer>>(&format!("/api/overlay/instances/{instance_id}/layers"))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn arty_get_snapshot(
    state: State<'_, AppState>,
    instance_id: String,
    layer_id: String,
) -> Result<Snapshot, String> {
    state
        .api
        .get::<Snapshot>(&format!(
            "/api/overlay/instances/{instance_id}/layers/{layer_id}/snapshot"
        ))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn arty_sse_connect(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    instance_id: String,
) -> Result<(), String> {
    let api = state.api.clone();
    let sse = state.sse.clone();
    drop(state);
    sse.connect(app, api, instance_id).await;
    Ok(())
}

#[tauri::command]
async fn arty_sse_disconnect(state: State<'_, AppState>) -> Result<(), String> {
    let sse = state.sse.clone();
    drop(state);
    sse.disconnect().await;
    Ok(())
}

// ── Hotkey helpers ────────────────────────────────────────────────────────────

fn register_hotkeys(
    app: &tauri::AppHandle,
    settings: &AppSettings,
) -> Result<(), Box<dyn std::error::Error>> {
    let app_toggle = app.clone();
    app.global_shortcut()
        .on_shortcut(settings.hotkey.as_str(), move |_app, _shortcut, event| {
            if event.state() == ShortcutState::Pressed {
                if let Some(win) = app_toggle.get_webview_window("overlay") {
                    let visible = win.is_visible().unwrap_or(false);
                    if visible {
                        let _ = win.hide();
                        if let Some(tray) = app_toggle.tray_by_id("main-tray") {
                            let _ = tray.set_visible(true);
                        }
                    } else {
                        let _ = win.show();
                        let _ = win.set_focus();
                        if let Some(tray) = app_toggle.tray_by_id("main-tray") {
                            let _ = tray.set_visible(false);
                        }
                    }
                }
            }
        })?;

    Ok(())
}

// ── Entry point ───────────────────────────────────────────────────────────────

fn main() {
    let settings = load_settings();
    let settings_for_setup = settings.clone();

    let token_store = TokenStore::load_from_disk();
    let api_client = ApiClient::new(token_store).expect("failed to build ApiClient");
    let sse_manager = SseManager::new();

    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .manage(AppState {
            settings:   Mutex::new(settings),
            api:        api_client,
            sse:        sse_manager,
            oauth_lock: Arc::new(AsyncMutex::new(())),
        })
        .setup(move |app| {
            let show_item = MenuItemBuilder::with_id("show", "Afficher l'overlay").build(app)?;
            let quit_item = MenuItemBuilder::with_id("quit", "Quitter").build(app)?;
            let menu = MenuBuilder::new(app).items(&[&show_item, &quit_item]).build()?;

            TrayIconBuilder::with_id("main-tray")
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("11e Overlay (masqué)")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(win) = app.get_webview_window("overlay") {
                            let _ = win.show();
                            let _ = win.set_focus();
                        }
                        if let Some(tray) = app.tray_by_id("main-tray") {
                            let _ = tray.set_visible(false);
                        }
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(win) = app.get_webview_window("overlay") {
                            let _ = win.show();
                            let _ = win.set_focus();
                        }
                        let _ = tray.set_visible(false);
                    }
                })
                .build(app)?;

            if let Some(tray) = app.tray_by_id("main-tray") {
                let _ = tray.set_visible(false);
            }

            let overlay = app
                .get_webview_window("overlay")
                .expect("overlay window not found");

            let _ = overlay.set_position(PhysicalPosition::new(
                settings_for_setup.x,
                settings_for_setup.y,
            ));
            let _ = overlay.set_size(PhysicalSize::new(
                settings_for_setup.width,
                settings_for_setup.height,
            ));

            let app_geo = app.handle().clone();
            let last_save = Arc::new(Mutex::new(Instant::now() - Duration::from_secs(10)));
            overlay.on_window_event(move |event| {
                match event {
                    WindowEvent::Moved(_) | WindowEvent::Resized(_) => {}
                    _ => return,
                }

                let now = Instant::now();
                {
                    let mut last = last_save.lock().unwrap();
                    if now.duration_since(*last) < Duration::from_millis(400) {
                        return;
                    }
                    *last = now;
                }

                if let Some(win) = app_geo.get_webview_window("overlay") {
                    if let (Ok(pos), Ok(size)) = (win.outer_position(), win.inner_size()) {
                        let state = app_geo.state::<AppState>();
                        if let Ok(mut s) = state.settings.lock() {
                            s.x = pos.x;
                            s.y = pos.y;
                            s.width = size.width;
                            s.height = size.height;
                            save_settings(&s);
                        };
                    }
                }
            });

            register_hotkeys(app.handle(), &settings_for_setup)
                .map_err(|e| format!("hotkey error: {e}"))?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_settings,
            get_content,
            save_settings_cmd,
            open_settings_window,
            hide_overlay_cmd,
            quit_app,
            // Auth
            auth_status,
            auth_discord_start,
            auth_local_login,
            auth_logout,
            // Artillery
            arty_list_instances,
            arty_list_layers,
            arty_get_snapshot,
            arty_sse_connect,
            arty_sse_disconnect,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            if let RunEvent::ExitRequested { .. } = event {
                // Strategy A: wipe tokens (encrypted blob + memory) on every
                // clean exit. Best-effort server logout with a short timeout.
                let app_handle = app_handle.clone();
                tauri::async_runtime::block_on(async move {
                    let state = app_handle.state::<AppState>();
                    state.sse.disconnect().await;
                    if state.api.tokens.is_authenticated().await {
                        let _ = tokio::time::timeout(
                            Duration::from_secs(1),
                            api_auth::logout(app_handle.clone(), state.api.clone()),
                        )
                        .await;
                    }
                    TokenStore::clear_disk_sync();
                });
            }
        });
}
