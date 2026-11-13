use std::collections::HashSet;
use std::fs;
use std::path::Path;

use crate::common::common_interface::ProjectKind;

const MANIFESTS: [&str; 5] = ["package.json", "go.mod", "Cargo.toml", "requirements.txt", "pyproject.toml"];
const API_HINTS: [&str; 14] = [
    "express", "fastify", "koa", "@nestjs", "hapi", "restify", "flask", "fastapi", "django", "gin-gonic",
    "gofiber", "labstack/echo", "axum", "actix-web",
];
const SOCKET_HINTS: [&str; 6] = ["socket.io", "\"ws\"", "websocket", "tungstenite", "uwebsockets", "signalr"];

/// Looks at the top of a folder and works out what kind of project it is.
pub struct ProjectDetectorModel;

impl ProjectDetectorModel {
    pub fn detect(root: &Path) -> Vec<ProjectKind> {
        let names: HashSet<String> = fs::read_dir(root)
            .map(|dir| dir.flatten().map(|e| e.file_name().to_string_lossy().to_string()).collect::<HashSet<String>>())
            .unwrap_or_default();

        let has = |name: &str| names.contains(name);
        let ends_with = |suffix: &str| names.iter().any(|name| name.ends_with(suffix));
        let manifest = Self::read_manifests(root, &names);
        let mentions = |hints: &[&str]| hints.iter().any(|hint| manifest.contains(hint));

        let mut kinds = Vec::new();
        let is_unity = has("Assets") && has("ProjectSettings");

        if is_unity {
            kinds.push(ProjectKind::Unity);
        }

        if ends_with(".uproject") {
            kinds.push(ProjectKind::Unreal);
        }

        if has("project.godot") {
            kinds.push(ProjectKind::Godot);
        }

        if has(".creator") || has("cc.config.json") || manifest.contains("cocos") || (has("assets") && has("settings") && has("project.json")) {
            kinds.push(ProjectKind::Cocos);
        }

        if has("package.json") {
            kinds.push(ProjectKind::NodeJs);
        }

        if mentions(&API_HINTS) {
            kinds.push(ProjectKind::Api);
        }

        if mentions(&SOCKET_HINTS) {
            kinds.push(ProjectKind::WebSocket);
        }

        if has("Cargo.toml") {
            kinds.push(ProjectKind::Rust);
        }

        if has("go.mod") {
            kinds.push(ProjectKind::Go);
        }

        if has("pyproject.toml") || has("requirements.txt") || has("setup.py") {
            kinds.push(ProjectKind::Python);
        }

        if !is_unity && (ends_with(".csproj") || ends_with(".sln")) {
            kinds.push(ProjectKind::DotNet);
        }

        if kinds.is_empty() {
            kinds.push(ProjectKind::Scripts);
        }

        kinds
    }

    /// All package files glued together in lower case, so we can search them for library names.
    fn read_manifests(root: &Path, names: &HashSet<String>) -> String {
        let mut text = String::new();

        for file in MANIFESTS {
            if names.contains(file) {
                text.push_str(&fs::read_to_string(root.join(file)).unwrap_or_default().to_lowercase());
                text.push('\n');
            }
        }

        for name in names.iter().filter(|n| n.ends_with(".csproj")) {
            text.push_str(&fs::read_to_string(root.join(name)).unwrap_or_default().to_lowercase());
        }

        text
    }
}
