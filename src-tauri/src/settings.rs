use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::PathBuf;

fn default_font_size() -> u32 { 12 }
fn default_active_section() -> Option<String> { None }
fn default_active_mode() -> String { "lexique".to_string() }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub hotkey: String,
    /// Background opacity 0–1
    pub opacity: f64,
    pub width:   u32,
    pub height:  u32,
    pub x:       i32,
    pub y:       i32,
    /// Content font size in px
    #[serde(rename = "fontSize", default = "default_font_size")]
    pub font_size: u32,
    /// Last active section chip (None = "Tout")
    #[serde(rename = "activeSection", default = "default_active_section")]
    pub active_section: Option<String>,

    /// Active top-level mode: "lexique" | "artillery".
    #[serde(rename = "activeMode", default = "default_active_mode")]
    pub active_mode: String,

    /// Last selections in artillery mode (persisted across restarts).
    #[serde(rename = "artyInstanceId", default)]
    pub arty_instance_id: Option<String>,
    #[serde(rename = "artyLayerId", default)]
    pub arty_layer_id:    Option<String>,
    #[serde(rename = "artyGroupId", default)]
    pub arty_group_id:    Option<String>,
    #[serde(rename = "artyBatteryId", default)]
    pub arty_battery_id:  Option<String>,
    /// When true, the artillery panel shows solutions for all batteries of the group.
    #[serde(rename = "artyShowAllBatteries", default)]
    pub arty_show_all_batteries: bool,

    /// AES-GCM encrypted token bundle (base64). `None` = logged out.
    /// Wiped on app exit (strategy A).
    #[serde(rename = "encryptedTokens", default)]
    pub encrypted_tokens: Option<String>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            hotkey:          "F1".to_string(),
            opacity: 0.88,
            width:   320,
            height:  520,
            x:       100,
            y:       100,
            font_size: 12,
            active_section: None,
            active_mode: default_active_mode(),
            arty_instance_id: None,
            arty_layer_id:    None,
            arty_group_id:    None,
            arty_battery_id:  None,
            arty_show_all_batteries: false,
            encrypted_tokens: None,
        }
    }
}

pub fn settings_path() -> PathBuf {
    env::current_exe()
        .expect("cannot get exe path")
        .parent()
        .expect("exe has no parent dir")
        .join("11eOverlay.json")
}

pub fn load_settings() -> AppSettings {
    let path = settings_path();
    if path.exists() {
        if let Ok(raw) = fs::read_to_string(&path) {
            if let Ok(s) = serde_json::from_str::<AppSettings>(&raw) {
                return s;
            }
        }
    }
    AppSettings::default()
}

pub fn save_settings(s: &AppSettings) {
    if let Ok(json) = serde_json::to_string_pretty(s) {
        let _ = fs::write(settings_path(), json);
    }
}

/// Non-destructive merge: only fields present in `patch` override `current`.
pub fn merge_settings(current: AppSettings, patch: serde_json::Value) -> AppSettings {
    let mut base = serde_json::to_value(&current).unwrap_or_default();
    if let (Some(b), Some(p)) = (base.as_object_mut(), patch.as_object()) {
        for (k, v) in p {
            b.insert(k.clone(), v.clone());
        }
    }
    serde_json::from_value(base).unwrap_or(current)
}
