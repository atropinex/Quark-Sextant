# 🧭 Quark-Sextant

Quark-Sextant is a visual-only script flow viewer for spotting spaghetti code. Point it at a project folder and it turns every script the developers wrote into a draggable node graph, so you can see what uses what, where the circular dependencies hide, and which files carry too much weight. It never edits your code. It only shows you the shape of it.

## ✨ Features

* 🗺️ **Visual Script Graph:** Every script becomes a node with in and out ports, joined by smooth curves, in the style of a visual-scripting editor. Drag nodes, pan, zoom, and double-click to glide to a script.
* 🧩 **Project Type Detection:** Recognizes Node.js, API, WebSocket, Unity, Cocos, Unreal, Godot, Rust, Go, Python and .NET projects and shows them as tags.
* 🍝 **Spaghetti Detection:** Finds circular dependencies (drawn in red), hub scripts that touch too many others, isolated scripts (possible dead code) and oversized files, then rolls them into a 0 to 100 health score.
* 🗂️ **Grouped or Flow Layout:** Box scripts by folder, or arrange them by dependency flow from entry points to leaves. Switch any time and the nodes glide to their new places.
* 📦 **Dev Scripts Only:** Package and vendor folders (`node_modules`, `vendor`, `Library`, `Pods`, `third_party` and more), build output and generated files are skipped.
* 🛠️ **Handy Tools:** Search, Problems list, rescan, Markdown report export, open a script in your editor, show its folder, and copy its path.
* 🎨 **Themes:** Unity-style dark and light skins plus Midnight and Ember, all plain YAML files you can edit and reload.
* 🚀 **Separate Launcher:** A Unity-Hub-style launcher keeps your recent projects and starts the viewer in its own window.
* 🌊 **Buttery Motion:** Camera, nodes, buttons, rows and the health bar all glide instead of snapping.

## 🚀 Tech Stack

* Language: Rust
* Desktop UI: eframe / egui (immediate-mode, GPU drawn)
* Scanning: `std::fs` walker + `regex` import parsing, Tarjan's algorithm for cycles
* Saved data: `serde`, `serde_json`, `serde_yaml`
* Native dialogs: `rfd`

## 🛠️ Getting Started

```
cargo build --release
target/release/quark-sextant-launcher
```

On Linux, install the GTK3 dev files first (`sudo apt install libgtk-3-dev`). The full walkthrough is in [USAGE.md](USAGE.md).

## 📄 License
This project is licensed under the Apache License, Version 2.0. See the [LICENSE](https://github.com/atropinex/Quark-Sextant/blob/main/LICENSE) file for details.
