use regex::Regex;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use crate::common::common_interface::{Language, ScanReport, ScriptEdge, ScriptGraph, ScriptNode};
use crate::common::models::graph_metrics_model::GraphMetricsModel;
use crate::common::models::project_detector_model::ProjectDetectorModel;

// Packages, vendored libraries and build output. Lower case: folder names are compared in lower case.
const IGNORED_DIRS: &[&str] = &[
    // packages and third-party code
    "node_modules", "bower_components", "jspm_packages", "vendor", "third_party", "thirdparty", "3rdparty",
    "external", "externals", "extern", "deps", "packagecache", "pods", "carthage", "site-packages", "venv",
    "textmesh pro",
    // build output and engine caches
    "target", "dist", "build", "bin", "obj", "library", "temp", "logs", "intermediate", "binaries", "saved",
    "deriveddatacache", "__pycache__", "testdata", "coverage",
];
// Machine-written files are not "scripts created by the devs" either.
const GENERATED_SUFFIXES: &[&str] = &[".d.ts", ".min.js", ".g.cs", ".generated.cs", ".designer.cs", ".pb.go"];
const MAX_SCRIPTS: usize = 900;          // more nodes than this makes the graph unreadable anyway
const MAX_FILE_BYTES: u64 = 400 * 1024;  // bigger files are bundles or generated code

fn re(pattern: &str) -> Regex {
    Regex::new(pattern).expect("built-in pattern is valid")
}

static JS_IMPORT: LazyLock<Regex> = LazyLock::new(|| re(r#"(?:from|import|require)\s*\(?\s*['"]([^'"\n]+)['"]"#));
static RUST_MOD: LazyLock<Regex> = LazyLock::new(|| re(r"(?m)^\s*(?:pub(?:\([^)]*\))?\s+)?mod\s+(\w+)\s*;"));
static RUST_USE: LazyLock<Regex> = LazyLock::new(|| re(r"(?m)^\s*(?:pub(?:\([^)]*\))?\s+)?use\s+([\w:]+)"));
static GO_SINGLE: LazyLock<Regex> = LazyLock::new(|| re(r#"(?m)^\s*import\s+(?:[\w.]+\s+)?"([^"]+)""#));
static GO_BLOCK: LazyLock<Regex> = LazyLock::new(|| re(r"(?s)import\s*\((.*?)\)"));
static QUOTED: LazyLock<Regex> = LazyLock::new(|| re(r#""([^"]+)""#));
static PY_FROM: LazyLock<Regex> = LazyLock::new(|| re(r"(?m)^\s*from\s+([\w.]+)\s+import"));
static PY_IMPORT: LazyLock<Regex> = LazyLock::new(|| re(r"(?m)^\s*import\s+([\w.]+)"));
static C_INCLUDE: LazyLock<Regex> = LazyLock::new(|| re(r#"(?m)^\s*#\s*include\s*"([^"]+)""#));
static LUA_REQUIRE: LazyLock<Regex> = LazyLock::new(|| re(r#"require\s*\(?\s*['"]([^'"]+)['"]"#));
static GD_PATH: LazyLock<Regex> = LazyLock::new(|| re(r#"(?:preload|load)\(\s*"res://([^"]+)""#));
static TYPE_DECL: LazyLock<Regex> = LazyLock::new(|| re(r"\b(?:class|struct|interface|enum|record)\s+([A-Za-z_]\w*)"));
static GD_CLASS: LazyLock<Regex> = LazyLock::new(|| re(r"(?m)^\s*class_name\s+(\w+)"));
static IDENT: LazyLock<Regex> = LazyLock::new(|| re(r"[A-Za-z_]\w*"));

/// Walks a project folder, finds every script and works out which scripts use which.
pub struct ScriptScannerModel;

impl ScriptScannerModel {
    pub fn scan(root: &Path, on_progress: &dyn Fn(usize)) -> ScanReport {
        let (files, truncated, skipped) = Self::collect(root, on_progress);
        let rels: Vec<String> = files.iter().map(|f| f.0.clone()).collect();
        let index = PathIndex::build(&rels);

        let mut nodes = Vec::with_capacity(files.len());
        let mut edges: Vec<ScriptEdge> = Vec::new();
        let mut seen: HashSet<(usize, usize)> = HashSet::new();
        let mut declared: HashMap<String, usize> = HashMap::new();
        let mut words: Vec<HashSet<String>> = vec![HashSet::new(); files.len()];

        for (i, (rel, abs, language)) in files.iter().enumerate() {
            let text = fs::read(abs).map(|bytes| String::from_utf8_lossy(&bytes).into_owned()).unwrap_or_default();
            for spec in Self::extract_specs(*language, &text) {
                match resolve(&index, rel, *language, &spec) {
                    Some(target) if target != i => {
                        if seen.insert((i, target)) {
                            edges.push(ScriptEdge { from: i, to: target });
                        }
                    }
                    Some(_) => {}
                    None => {} // a package, not part of this project
                }
            }

            if language.uses_type_references() {
                for name in Self::declared_types(*language, &text) {
                    declared.entry(name).or_insert(i);
                }

                words[i] = IDENT.find_iter(&text).map(|m| m.as_str().to_string()).collect();
            }

            nodes.push(ScriptNode {
                rel_path: rel.clone(),
                name: rel.rsplit('/').next().unwrap_or(rel).to_string(),
                folder: parent(rel).unwrap_or("").to_string(),
                language: *language,
                lines: text.lines().count(),
            });
        }

        // C#, Java, Kotlin and GDScript: a script "uses" another when it mentions a class declared there.
        for i in 0..nodes.len() {
            if !nodes[i].language.uses_type_references() {
                continue;
            }

            for word in &words[i] {
                let Some(&target) = declared.get(word) else { continue };

                if target != i && word.len() >= 3 && nodes[target].language == nodes[i].language && seen.insert((i, target)) {
                    edges.push(ScriptEdge { from: i, to: target });
                }
            }
        }

        edges.sort_by_key(|e| (e.from, e.to));

        let metrics = GraphMetricsModel::compute(nodes.len(), &edges);

        ScanReport {
            root: root.to_path_buf(),
            kinds: ProjectDetectorModel::detect(root),
            graph: ScriptGraph { nodes, edges, truncated, skipped },
            metrics,
        }
    }

    /// Breadth first, so when a project is too big the shallow (important) scripts win.
    fn collect(root: &Path, on_progress: &dyn Fn(usize)) -> (Vec<(String, PathBuf, Language)>, bool, usize) {
        let mut queue = VecDeque::from([root.to_path_buf()]);
        let mut found: Vec<(String, PathBuf, Language)> = Vec::new();
        let mut truncated = false;
        let mut skipped = 0;

        while let Some(dir) = queue.pop_front() {
            let Ok(entries) = fs::read_dir(&dir) else { continue };
            let mut entries: Vec<_> = entries.flatten().collect();
            entries.sort_by_key(|e| e.file_name());

            for entry in entries {
                let path = entry.path();
                let name = entry.file_name().to_string_lossy().to_string();
                let Ok(kind) = entry.file_type() else { continue };

                if kind.is_dir() {
                    if !name.starts_with('.') && !IGNORED_DIRS.contains(&name.to_lowercase().as_str()) {
                        queue.push_back(path);
                    }

                    continue;
                }

                let lower = name.to_lowercase();

                if !kind.is_file() || GENERATED_SUFFIXES.iter().any(|s| lower.ends_with(s)) {
                    continue;
                }

                let language = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .and_then(|e| Language::from_extension(&e.to_lowercase()));
                let Some(language) = language else { continue };

                if entry.metadata().map(|m| m.len()).unwrap_or(0) > MAX_FILE_BYTES {
                    skipped += 1;
                } else if found.len() >= MAX_SCRIPTS {
                    truncated = true;
                    skipped += 1;
                } else {
                    let rel = path.strip_prefix(root).unwrap_or(&path).to_string_lossy().replace('\\', "/");
                    found.push((rel, path, language));
                }
            }

            on_progress(found.len());
        }

        (found, truncated, skipped)
    }

    fn extract_specs(language: Language, text: &str) -> Vec<String> {
        match language {
            Language::TypeScript | Language::JavaScript => captures(&JS_IMPORT, text),
            Language::Rust => {
                let mut specs: Vec<String> = captures(&RUST_MOD, text).into_iter().map(|m| format!("mod::{m}")).collect();
                specs.extend(captures(&RUST_USE, text));
                specs
            }
            Language::Go => {
                let mut specs = captures(&GO_SINGLE, text);

                for block in GO_BLOCK.captures_iter(text) {
                    specs.extend(captures(&QUOTED, &block[1]));
                }

                specs
            }
            Language::Python => {
                let mut specs = captures(&PY_FROM, text);
                specs.extend(captures(&PY_IMPORT, text));
                specs
            }
            Language::C | Language::Cpp => captures(&C_INCLUDE, text),
            Language::Lua => captures(&LUA_REQUIRE, text),
            Language::GdScript => captures(&GD_PATH, text),
            Language::CSharp | Language::Java | Language::Kotlin => Vec::new(),
        }
    }

    fn declared_types(language: Language, text: &str) -> Vec<String> {
        match language {
            Language::CSharp | Language::Java | Language::Kotlin => captures(&TYPE_DECL, text),
            Language::GdScript => captures(&GD_CLASS, text),
            _ => Vec::new(),
        }
    }
}

fn captures(pattern: &Regex, text: &str) -> Vec<String> {
    pattern.captures_iter(text).map(|c| c[1].to_string()).collect()
}

// === Turning an import into a file ============================================== //

fn parent(path: &str) -> Option<&str> {
    path.rfind('/').map(|i| &path[..i])
}

fn strip_ext(path: &str) -> &str {
    match path.rfind('.') {
        Some(i) if path.rfind('/').map_or(true, |slash| i > slash) => &path[..i],
        _ => path,
    }
}

/// Joins `rel` onto `base` and resolves "." and "..".
fn normalize(base: &str, rel: &str) -> String {
    let mut parts: Vec<&str> = if rel.starts_with('/') { Vec::new() } else { base.split('/').filter(|s| !s.is_empty()).collect() };

    for segment in rel.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            other => parts.push(other),
        }
    }

    parts.join("/")
}

struct PathIndex {
    by_path: HashMap<String, usize>,   // "src/app/main.ts"
    by_key: HashMap<String, usize>,    // "src/app/main" and the folder "src/app" (index / mod files)
    by_suffix: HashMap<String, usize>, // every tail of a key: "app/main", "main"
}

impl PathIndex {
    fn build(rels: &[String]) -> Self {
        let mut by_path = HashMap::new();
        let mut by_key: HashMap<String, usize> = HashMap::new();

        for (i, rel) in rels.iter().enumerate() {
            by_path.insert(rel.clone(), i);
            by_key.entry(strip_ext(rel).to_string()).or_insert(i);
        }

        // A folder stands for its index-like file first, then for any file inside it.
        for pass in 0..2 {
            for (i, rel) in rels.iter().enumerate() {
                let stem = strip_ext(rel.rsplit('/').next().unwrap_or(rel));
                let index_like = matches!(stem, "index" | "mod" | "__init__" | "main" | "lib");

                if let Some(dir) = parent(rel) {
                    if (pass == 0) == index_like {
                        by_key.entry(dir.to_string()).or_insert(i);
                    }
                }
            }
        }

        let mut keys: Vec<(&String, &usize)> = by_key.iter().collect();
        keys.sort();
        let mut by_suffix = HashMap::new();

        for (key, &i) in keys {
            by_suffix.entry(key.clone()).or_insert(i);

            for (pos, _) in key.match_indices('/') {
                by_suffix.entry(key[pos + 1..].to_string()).or_insert(i);
            }
        }

        Self { by_path, by_key, by_suffix }
    }

    fn exact(&self, key: &str) -> Option<usize> {
        self.by_path
            .get(key)
            .or_else(|| self.by_key.get(key))
            .or_else(|| self.by_key.get(strip_ext(key)))
            .copied()
    }

    fn suffix(&self, key: &str) -> Option<usize> {
        self.by_key.get(key).or_else(|| self.by_suffix.get(key)).copied()
    }

    /// Tries the whole path, then keeps dropping the last part ("a/b/Type" -> "a/b" -> "a").
    fn shrinking(&self, path: &str) -> Option<usize> {
        let mut parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

        while !parts.is_empty() {
            if let Some(found) = self.suffix(&parts.join("/")) {
                return Some(found);
            }

            parts.pop();
        }

        None
    }
}

/// Heuristic: good enough to show the real flow, not a compiler.
fn resolve(index: &PathIndex, from: &str, language: Language, spec: &str) -> Option<usize> {
    let dir = parent(from).unwrap_or("");

    match language {
        Language::TypeScript | Language::JavaScript => {
            if spec.starts_with('.') {
                index.exact(&normalize(dir, spec))
            } else if let Some(rest) = spec.strip_prefix("@/").or_else(|| spec.strip_prefix("~/")) {
                index.suffix(strip_ext(rest))
            } else if spec.starts_with("src/") {
                index.suffix(strip_ext(spec))
            } else {
                None // a package from node_modules
            }
        }
        Language::C | Language::Cpp => index.exact(&normalize(dir, spec)).or_else(|| index.suffix(strip_ext(spec))),
        Language::Rust => {
            if let Some(name) = spec.strip_prefix("mod::") {
                let stem = strip_ext(from.rsplit('/').next().unwrap_or(from));
                let base = if matches!(stem, "mod" | "lib" | "main") { dir.to_string() } else { normalize(dir, stem) };
                return index.exact(&normalize(&base, name));
            }

            let path = spec.replace("::", "/");
            let path = path.strip_prefix("crate/").unwrap_or(&path);

            if path.starts_with("self") || path.starts_with("super") {
                return None;
            }

            index.shrinking(path)
        }
        Language::Go => {
            let found = index.suffix(spec);

            if found.is_some() || !spec.split('/').next().unwrap_or("").contains('.') {
                return found;
            }

            // github.com/me/app/pkg/util -> pkg/util -> util
            let parts: Vec<&str> = spec.split('/').collect();
            (1..parts.len()).find_map(|skip| index.suffix(&parts[skip..].join("/")))
        }
        Language::Python => {
            let dots = spec.chars().take_while(|&c| c == '.').count();
            let rest = spec[dots..].replace('.', "/");

            if dots == 0 {
                return index.shrinking(&rest);
            }

            let mut base = dir;

            for _ in 1..dots {
                base = parent(base).unwrap_or("");
            }

            index.exact(&normalize(base, &rest))
        }
        Language::Lua => index.shrinking(&spec.replace('.', "/")),
        Language::GdScript => index.by_path.get(spec).copied(),
        Language::CSharp | Language::Java | Language::Kotlin => None,
    }
}
