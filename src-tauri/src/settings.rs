use serde::{Deserialize, Serialize};
use std::fs;
use tauri::{AppHandle, Manager};

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Settings {
    pub output_dir: String,
    pub concurrency: usize,
    pub country: String,
    pub quality: String,
    pub organize: bool,
    pub cookies_browser: String,
    pub extra_args: String,
    pub retries: u32,
    pub skip_existing: bool,
    pub close_to_tray: bool,
    pub onboarded: bool,
    pub language: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            output_dir: String::new(),
            concurrency: 3,
            country: "US".into(),
            quality: "best".into(),
            organize: true,
            cookies_browser: String::new(),
            extra_args: String::new(),
            retries: 3,
            skip_existing: true,
            close_to_tray: false,
            onboarded: false,
            language: String::new(),
        }
    }
}

fn path(app: &AppHandle) -> Option<std::path::PathBuf> {
    app.path().app_config_dir().ok().map(|p| p.join("settings.json"))
}

pub fn load(app: &AppHandle) -> Settings {
    path(app)
        .and_then(|p| fs::read_to_string(p).ok())
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

pub fn save(app: &AppHandle, s: &Settings) {
    if let Some(p) = path(app) {
        if let Some(d) = p.parent() {
            let _ = fs::create_dir_all(d);
        }
        if let Ok(t) = serde_json::to_string_pretty(s) {
            let _ = fs::write(p, t);
        }
    }
}
