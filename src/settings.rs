use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

type AppError = Box<dyn std::error::Error + Send + Sync>;

const DEFAULT_AGILE_WINDOW_SLOTS: usize = 24;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub agile_window_slots: usize,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            agile_window_slots: DEFAULT_AGILE_WINDOW_SLOTS,
        }
    }
}

fn settings_path() -> PathBuf {
    PathBuf::from("data/settings.json")
}

fn sanitise_agile_window_slots(value: usize) -> usize {
    match value {
        24 | 36 | 48 => value,
        _ => DEFAULT_AGILE_WINDOW_SLOTS,
    }
}

pub fn load_settings() -> Result<AppSettings, AppError> {
    let path = settings_path();

    if !path.exists() {
        return Ok(AppSettings::default());
    }

    let json = fs::read_to_string(path)?;
    let mut settings = serde_json::from_str::<AppSettings>(&json)?;
    settings.agile_window_slots = sanitise_agile_window_slots(settings.agile_window_slots);

    Ok(settings)
}

pub fn save_settings(settings: &AppSettings) -> Result<AppSettings, AppError> {
    let path = settings_path();

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let clean = AppSettings {
        agile_window_slots: sanitise_agile_window_slots(settings.agile_window_slots),
    };

    let json = serde_json::to_string_pretty(&clean)?;
    fs::write(path, json)?;

    Ok(clean)
}

pub fn ensure_settings_file() -> Result<AppSettings, AppError> {
    let path = settings_path();

    if path.exists() {
        return load_settings();
    }

    let defaults = AppSettings::default();
    save_settings(&defaults)
}

pub fn get_agile_window_slots() -> usize {
    load_settings()
        .map(|s| s.agile_window_slots)
        .unwrap_or(DEFAULT_AGILE_WINDOW_SLOTS)
}
