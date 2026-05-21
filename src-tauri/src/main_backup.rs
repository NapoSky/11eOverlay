// Hide the console window in release builds on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod content;
mod settings;

use content::{load_content, ContentSection};
use settings::{load_settings, merge_settings, save_settings, AppSettings};

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{Emitter, Manager, PhysicalPosition, PhysicalSize, State, WebviewUrl, WebviewWindowBuilder, WindowEvent};
use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

struct AppState {
    settings: Mutex<AppSettings>,
}

// ── Tauri commands ────────────────────────────────────────────────────────────

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

    // Notify the overlay window so it can update its opacity / font size live.
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

// ── Hotkey helpers ────────────────────────────────────────────────────────────

fn register_hotkeys(
    app: &tauri::AppHandle,
    settings: &AppSettings,
) -> Result<(), Box<dyn std::error::Error>> {
    // Toggle visibility hotkey
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

    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(AppState {
            settings: Mutex::new(settings),
        })
        .setup(move |app| {
            // Build tray menu and icon (hidden at startup since overlay is visible).
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

            // Start hidden — overlay is visible at launch.
            if let Some(tray) = app.tray_by_id("main-tray") {
                let _ = tray.set_visible(false);
            }

            let overlay = app
                .get_webview_window("overlay")
                .expect("overlay window not found");

            // Apply saved position and size before the window is visible.
            let _ = overlay.set_position(PhysicalPosition::new(
                settings_for_setup.x,
                settings_for_setup.y,
            ));
            let _ = overlay.set_size(PhysicalSize::new(
                settings_for_setup.width,
                settings_for_setup.height,
            ));

            // Save geometry to settings.json when the window is moved or resized.
            // Handled entirely in Rust so it works with the custom titlebar drag.
            let app_geo = app.handle().clone();
            let last_save = Arc::new(Mutex::new(
                Instant::now() - Duration::from_secs(10),
            ));
            overlay.on_window_event(move |event| {
                match event {
                    WindowEvent::Moved(_) | WindowEvent::Resized(_) => {}
                    _ => return,
                }

                // Throttle: write at most once per 400 ms.
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
