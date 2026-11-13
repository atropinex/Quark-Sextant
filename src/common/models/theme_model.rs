use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::common::common_interface::{Theme, ThemeList, THEME_COLORS};
use crate::common::models::default_theme_model::DEFAULT_THEMES;
use crate::common::utils::color_helper;
use crate::common::utils::path_helper::app_data_dir;

#[derive(Deserialize)]
struct RawTheme {
    name: Option<String>,
    colors: Option<HashMap<String, String>>,
}

/// Themes are YAML files in a folder. This reads and checks them.
pub struct ThemeModel {
    pub folder: PathBuf,
}

impl ThemeModel {
    pub fn new() -> Self {
        let folder = app_data_dir().join("themes");
        let is_fresh = fs::read_dir(&folder).map(|mut dir| dir.next().is_none()).unwrap_or(true);

        let _ = fs::create_dir_all(&folder);

        if is_fresh {
            for (file, content) in DEFAULT_THEMES {
                let _ = fs::write(folder.join(file), content);
            }
        }

        Self { folder }
    }

    /// Reads every theme file. A broken file is reported without stopping the others.
    pub fn load(&self) -> ThemeList {
        let mut themes = Vec::new();
        let mut errors = Vec::new();

        let mut files: Vec<PathBuf> = fs::read_dir(&self.folder)
            .map(|dir| dir.flatten().map(|entry| entry.path()).collect::<Vec<_>>())
            .unwrap_or_default();
        files.sort();

        for path in files {
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();

            if ext != "yaml" && ext != "yml" {
                continue;
            }

            let id = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();

            match self.parse(&id, &path) {
                Ok(theme) => themes.push(theme),
                Err(error) => errors.push(format!("{id}: {error}")),
            }
        }

        if themes.is_empty() {
            themes.push(Theme::unity_dark());
        }

        ThemeList { themes, errors }
    }

    fn parse(&self, id: &str, path: &Path) -> Result<Theme, String> {
        let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
        let raw: RawTheme = serde_yaml::from_str(&text).map_err(|e| e.to_string())?;
        let mut colors = HashMap::new();

        for key in THEME_COLORS {
            let Some(value) = raw.colors.as_ref().and_then(|c| c.get(key.key())) else {
                continue;
            };

            match color_helper::hex(value) {
                Some(color) => {
                    colors.insert(key, color);
                }
                None => return Err(format!("colors.{} must be a hex color like \"#1a2b3c\"", key.key())),
            }
        }

        Ok(Theme { id: id.to_string(), name: raw.name.unwrap_or_else(|| id.to_string()), colors })
    }
}
