use std::path::{Path, PathBuf};
use std::process::Command;

/// Where settings, recent projects and themes live.
pub fn app_data_dir() -> PathBuf {
    let dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from(".")).join("Quark-Sextant");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

/// The other Quark-Sextant program that sits next to this one (the viewer and the launcher ship together).
pub fn sibling_executable(name: &str) -> PathBuf {
    let file = format!("{name}{}", std::env::consts::EXE_SUFFIX);

    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join(&file)))
        .unwrap_or_else(|| PathBuf::from(file))
}

/// Opens a folder in the system file manager.
pub fn reveal(path: &Path) {
    let program = if cfg!(target_os = "windows") {
        "explorer"
    } else if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };

    let _ = Command::new(program).arg(path).spawn();
}

/// Opens a file with whatever the system uses for it (usually your code editor).
pub fn open_path(path: &Path) {
    if cfg!(target_os = "windows") {
        let _ = Command::new("cmd").args(["/C", "start", ""]).arg(path).spawn();
    } else {
        reveal(path); // "open" and "xdg-open" work for files too
    }
}
