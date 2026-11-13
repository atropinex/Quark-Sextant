use std::fs;
use std::path::PathBuf;

use crate::common::common_interface::AppSettings;
use crate::common::utils::path_helper::app_data_dir;

fn defaults() -> AppSettings {
    AppSettings { theme: "unity-dark".to_string(), last_project: None, grouped: true }
}

/// The app's settings, saved on this computer. Shared by the launcher and the viewer.
pub struct SettingsModel {
    file: PathBuf,
    settings: AppSettings,
}

impl SettingsModel {
    pub fn new() -> Self {
        let file = app_data_dir().join("settings.json");
        let settings = fs::read_to_string(&file)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_else(defaults);

        Self { file, settings }
    }

    pub fn get(&self) -> AppSettings {
        self.settings.clone()
    }

    pub fn update(&mut self, change: impl FnOnce(&mut AppSettings)) {
        change(&mut self.settings);

        if let Ok(text) = serde_json::to_string_pretty(&self.settings) {
            let _ = fs::write(&self.file, text);
        }
    }
}
