use eframe::egui::Color32;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

// === What kind of project is it? ================================================ //

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum ProjectKind {
    NodeJs,
    Api,
    WebSocket,
    Unity,
    Cocos,
    Unreal,
    Godot,
    Rust,
    Go,
    Python,
    DotNet,
    Scripts, // nothing recognised, just a folder of scripts
}

impl ProjectKind {
    pub fn label(&self) -> &'static str {
        match self {
            Self::NodeJs => "Node.js",
            Self::Api => "API",
            Self::WebSocket => "WebSocket",
            Self::Unity => "Unity Game",
            Self::Cocos => "Cocos Game",
            Self::Unreal => "Unreal Game",
            Self::Godot => "Godot Game",
            Self::Rust => "Rust",
            Self::Go => "Go",
            Self::Python => "Python",
            Self::DotNet => ".NET",
            Self::Scripts => "Scripts",
        }
    }

    pub fn color(&self) -> Color32 {
        let (r, g, b) = match self {
            Self::NodeJs => (0x6C, 0xC2, 0x4A),
            Self::Api => (0x4D, 0x9D, 0xE0),
            Self::WebSocket => (0xB6, 0x9C, 0xFF),
            Self::Unity => (0xE6, 0xE6, 0xE6),
            Self::Cocos => (0x4C, 0xC9, 0xB0),
            Self::Unreal => (0x7F, 0xA6, 0xFF),
            Self::Godot => (0x6F, 0xB5, 0xE8),
            Self::Rust => (0xF0, 0x8A, 0x4B),
            Self::Go => (0x5B, 0xC8, 0xE8),
            Self::Python => (0xF2, 0xD0, 0x5C),
            Self::DotNet => (0x9B, 0x7B, 0xE8),
            Self::Scripts => (0x9A, 0x9A, 0x9A),
        };
        Color32::from_rgb(r, g, b)
    }
}

// === Scripts ===================================================================== //

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Language {
    TypeScript,
    JavaScript,
    Rust,
    Go,
    CSharp,
    Cpp,
    C,
    Java,
    Kotlin,
    Python,
    Lua,
    GdScript,
}

impl Language {
    pub fn from_extension(ext: &str) -> Option<Self> {
        Some(match ext {
            "ts" | "tsx" | "mts" | "cts" => Self::TypeScript,
            "js" | "jsx" | "mjs" | "cjs" => Self::JavaScript,
            "rs" => Self::Rust,
            "go" => Self::Go,
            "cs" => Self::CSharp,
            "cpp" | "cc" | "cxx" | "hpp" | "hh" => Self::Cpp,
            "c" | "h" => Self::C,
            "java" => Self::Java,
            "kt" | "kts" => Self::Kotlin,
            "py" => Self::Python,
            "lua" => Self::Lua,
            "gd" => Self::GdScript,
            _ => return None,
        })
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::TypeScript => "TypeScript",
            Self::JavaScript => "JavaScript",
            Self::Rust => "Rust",
            Self::Go => "Go",
            Self::CSharp => "C#",
            Self::Cpp => "C++",
            Self::C => "C",
            Self::Java => "Java",
            Self::Kotlin => "Kotlin",
            Self::Python => "Python",
            Self::Lua => "Lua",
            Self::GdScript => "GDScript",
        }
    }

    pub fn color(&self) -> Color32 {
        let (r, g, b) = match self {
            Self::TypeScript => (0x31, 0x78, 0xC6),
            Self::JavaScript => (0xC9, 0xA8, 0x1E),
            Self::Rust => (0xC2, 0x62, 0x2E),
            Self::Go => (0x1F, 0x9F, 0xBF),
            Self::CSharp => (0x68, 0x4B, 0xB5),
            Self::Cpp => (0x3F, 0x6F, 0xA8),
            Self::C => (0x5A, 0x6B, 0x86),
            Self::Java => (0xB0, 0x52, 0x2A),
            Self::Kotlin => (0x8A, 0x4F, 0xC4),
            Self::Python => (0x3A, 0x7B, 0xA8),
            Self::Lua => (0x2F, 0x4F, 0x9E),
            Self::GdScript => (0x3F, 0x8F, 0xB8),
        };
        Color32::from_rgb(r, g, b)
    }

    /// These languages do not import files, they just use each other's classes by name.
    pub fn uses_type_references(&self) -> bool {
        matches!(self, Self::CSharp | Self::Java | Self::Kotlin | Self::GdScript)
    }
}

pub struct ScriptNode {
    pub rel_path: String, // always "/" separated
    pub name: String,
    pub folder: String,
    pub language: Language,
    pub lines: usize,
}

/// `from` uses / imports `to`.
#[derive(Clone, Copy)]
pub struct ScriptEdge {
    pub from: usize,
    pub to: usize,
}

pub struct ScriptGraph {
    pub nodes: Vec<ScriptNode>,
    pub edges: Vec<ScriptEdge>,
    pub truncated: bool, // true when the project had more scripts than we draw
    pub skipped: usize,
}

pub struct GraphMetrics {
    pub fan_in: Vec<usize>,
    pub fan_out: Vec<usize>,
    pub cycle_of: Vec<Option<usize>>, // index into `cycles`
    pub cycles: Vec<Vec<usize>>,      // every group of scripts that depend on each other
    pub edge_in_cycle: Vec<bool>,
    pub hubs: Vec<usize>, // scripts that touch too many others
    pub orphans: Vec<usize>, // scripts nobody uses that use nobody: dead code candidates
    pub score: f32,       // 0 = clean, 100 = spaghetti
}

impl GraphMetrics {
    pub fn verdict(&self) -> &'static str {
        match self.score {
            s if s < 20.0 => "Clean",
            s if s < 40.0 => "Healthy",
            s if s < 65.0 => "Tangled",
            _ => "Spaghetti",
        }
    }
}

pub struct ScanReport {
    pub root: PathBuf,
    pub kinds: Vec<ProjectKind>,
    pub graph: ScriptGraph,
    pub metrics: GraphMetrics,
}

/// Sent from the scanning thread to the UI.
pub enum ScanMessage {
    Progress(usize),
    Finished(Box<ScanReport>),
}

// === Saved data ================================================================== //

#[derive(Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub theme: String,
    pub last_project: Option<String>,
    #[serde(default = "yes")]
    pub grouped: bool, // true = scripts boxed by folder, false = laid out by flow
}

fn yes() -> bool {
    true
}

#[derive(Clone, Serialize, Deserialize)]
pub struct RecentProject {
    pub path: String,
    pub name: String,
    pub kinds: Vec<ProjectKind>,
    pub last_opened: u64, // seconds since 1970
}

// === Themes ====================================================================== //

pub const THEME_COLORS: [ThemeColor; 11] = [
    ThemeColor::Bg,
    ThemeColor::Panel,
    ThemeColor::Field,
    ThemeColor::Border,
    ThemeColor::Text,
    ThemeColor::Dim,
    ThemeColor::Accent,
    ThemeColor::Select,
    ThemeColor::Grid,
    ThemeColor::Node,
    ThemeColor::Danger,
];

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ThemeColor {
    Bg,     // the graph background
    Panel,  // toolbars and side panels
    Field,  // inputs, buttons, list backgrounds
    Border, // thin lines
    Text,   // main text
    Dim,    // secondary text
    Accent, // highlights
    Select, // selected rows and primary buttons
    Grid,   // grid lines on the graph
    Node,   // node body
    Danger, // cycles and warnings
}

impl ThemeColor {
    pub fn key(&self) -> &'static str {
        match self {
            Self::Bg => "bg",
            Self::Panel => "panel",
            Self::Field => "field",
            Self::Border => "border",
            Self::Text => "text",
            Self::Dim => "dim",
            Self::Accent => "accent",
            Self::Select => "select",
            Self::Grid => "grid",
            Self::Node => "node",
            Self::Danger => "danger",
        }
    }

    /// Unity's dark editor skin. Used when a theme leaves a color out.
    pub fn fallback(&self) -> Color32 {
        let (r, g, b) = match self {
            Self::Bg => (0x23, 0x23, 0x23),
            Self::Panel => (0x38, 0x38, 0x38),
            Self::Field => (0x2A, 0x2A, 0x2A),
            Self::Border => (0x1A, 0x1A, 0x1A),
            Self::Text => (0xD2, 0xD2, 0xD2),
            Self::Dim => (0x8C, 0x8C, 0x8C),
            Self::Accent => (0x5A, 0xA0, 0xE6),
            Self::Select => (0x2C, 0x5D, 0x87),
            Self::Grid => (0x2E, 0x2E, 0x2E),
            Self::Node => (0x2E, 0x2E, 0x2E),
            Self::Danger => (0xE5, 0x53, 0x4B),
        };
        Color32::from_rgb(r, g, b)
    }
}

pub struct Theme {
    pub id: String,
    pub name: String,
    pub colors: HashMap<ThemeColor, Color32>,
}

impl Theme {
    pub fn unity_dark() -> Self {
        Self { id: "unity-dark".into(), name: "Unity Dark".into(), colors: HashMap::new() }
    }

    pub fn color(&self, color: ThemeColor) -> Color32 {
        self.colors.get(&color).copied().unwrap_or_else(|| color.fallback())
    }

    pub fn is_light(&self) -> bool {
        let bg = self.color(ThemeColor::Bg);
        (bg.r() as u32 + bg.g() as u32 + bg.b() as u32) / 3 > 128
    }
}

pub struct ThemeList {
    pub themes: Vec<Theme>,
    pub errors: Vec<String>,
}
