use serde::{Deserialize, Serialize};
use std::env;
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentItem {
    pub key:   String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentSection {
    pub section: String,
    pub items:   Vec<ContentItem>,
}

/// Default content embedded at compile time.
/// Users can override by placing data/content.json next to the exe.
const EMBEDDED: &str = include_str!("../../data/content.json");

pub fn load_content() -> Vec<ContentSection> {
    // User override: data/content.json next to the exe
    if let Ok(exe) = env::current_exe() {
        let override_path = exe
            .parent()
            .map(|p| p.join("data").join("content.json"));

        if let Some(path) = override_path {
            if path.exists() {
                if let Ok(raw) = fs::read_to_string(&path) {
                    if let Ok(sections) = serde_json::from_str::<Vec<ContentSection>>(&raw) {
                        return sections;
                    }
                }
            }
        }
    }

    // Fall back to embedded content
    serde_json::from_str(EMBEDDED).unwrap_or_default()
}
