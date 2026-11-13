use std::fs;
use std::path::{Path, PathBuf};

use crate::common::common_interface::{ProjectKind, RecentProject};
use crate::common::utils::format_helper::now_secs;
use crate::common::utils::path_helper::app_data_dir;

const MAX_RECENT: usize = 30;

/// The projects you opened before, newest first.
pub struct RecentProjectsModel {
    file: PathBuf,
    items: Vec<RecentProject>,
}

impl RecentProjectsModel {
    pub fn new() -> Self {
        let file = app_data_dir().join("recent.json");
        let items = fs::read_to_string(&file)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default();

        Self { file, items }
    }

    pub fn list(&self) -> &[RecentProject] {
        &self.items
    }

    pub fn add(&mut self, path: &Path, kinds: Vec<ProjectKind>) {
        let path_text = path.to_string_lossy().to_string();
        let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| path_text.clone());

        self.items.retain(|item| item.path != path_text);
        self.items.insert(0, RecentProject { path: path_text, name, kinds, last_opened: now_secs() });
        self.items.truncate(MAX_RECENT);
        self.save();
    }

    pub fn remove(&mut self, path: &str) {
        self.items.retain(|item| item.path != path);
        self.save();
    }

    fn save(&self) {
        if let Ok(text) = serde_json::to_string_pretty(&self.items) {
            let _ = fs::write(&self.file, text);
        }
    }
}
