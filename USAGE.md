# 🧭 Quark-Sextant: How to Use

This guide walks through everything Quark-Sextant can do, from the first build to reading a messy graph.

## 1. Build and start

```
cargo build --release
```

This produces two programs in `target/release/`:

| Program | What it is |
|---|---|
| `quark-sextant-launcher` | Your project list and themes. Start here. |
| `quark-sextant` | The viewer. The launcher opens it for you. |

Keep them in the same folder, because the launcher looks for the viewer next to itself.

You can also run the viewer alone: `quark-sextant --project "C:\path\to\project"`, or run it with no argument and pick a folder inside.

**Linux:** the folder picker needs GTK3: `sudo apt install libgtk-3-dev`.

## 2. The launcher

- **Projects page:** press **Add folder** (or drop a folder on the window). Each project shows its name, path, type tags and when you last opened it.
- Click a project row to open it in the viewer. The small **x** removes it from the list (your files are untouched).
- **Themes page:** click a card to switch theme everywhere. **Reload** re-reads the YAML files. **Open themes folder** shows where they live.

## 3. The viewer at a glance

```
+--------------------------------------------------------------+
| Open Folder  project  [tags]      search  Fit Layout Rescan.. |  toolbar
+-----------+------------------------------------+-------------+
| Scripts   |                                    | Inspector   |
| Problems  |           the graph                |             |
|           |                                    |             |
+-----------+------------------------------------+-------------+
| 123 scripts   456 links   3 cycles   Tangled        Zoom 80%  |  status
+--------------------------------------------------------------+
```

- **Left panel:** the Scripts list (click to jump to a node) and the Problems list.
- **Center:** the graph.
- **Right panel (Inspector):** project health when nothing is selected, details of the script when one is.

## 4. Reading the graph

- **Node header color** is the language (TypeScript blue, Rust orange, C# purple, and so on). The header shows the file name.
- **Under the name:** the folder, then `in N  out M  L lines`.
  - `in` = how many scripts use this one.
  - `out` = how many scripts this one uses.
- **Ports:** the dot on the left is "used by", the dot on the right is "uses". A lit dot means there is at least one link.
- **Curves** go from the script that uses to the script that is used.
- **Red** outlines and curves are a **circular dependency**: scripts that depend on each other in a loop. This is the strongest spaghetti signal.
- **Selecting** a script lights it and its direct neighbours and fades the rest. Its curves turn bright.

## 5. Controls

| Action | How |
|---|---|
| Select a script | Click it |
| Zoom to a script | Double-click it, or click it in the left list |
| Move a node | Drag it |
| Pan | Drag the background |
| Zoom | Scroll wheel or pinch |
| Fit everything | `F` or **Fit** |
| Clear selection | `Esc` or click the background |
| Rescan the project | `F5` or **Rescan** |
| Re-arrange | `R` or **Re-layout** |
| Grouped / Flow | `G` or the layout button |
| Open a folder | Drop it on the window, or **Open Folder** |

Keys are ignored while you are typing in the search box.

## 6. The tools

### Search
Type in the toolbar search box. The Scripts list filters, and nodes that do not match fade out on the graph. Matching is on the whole path, so `models/user` works.

### Grouped and Flow layouts
- **Grouped:** one labelled box per folder, with the scripts in a grid inside. Good for seeing which folders talk to which. Each folder keeps its own color. Boxes follow their nodes if you drag them.
- **Flow:** scripts are placed in columns by dependency depth. Entry points are on the left and the things they rely on go to the right. Scripts in the same cycle share a column. Good for seeing the direction of the flow.
- **Re-layout** discards your manual dragging and re-arranges in the current mode. Your choice of mode is remembered.

### Problems list (left panel)
| Entry | Meaning |
|---|---|
| Circular dependency | A group of scripts that depend on each other in a loop. Click to jump to it. |
| Hub | A script that uses 12 or more scripts, or is used by 15 or more. Often a "god" file. |
| Isolated | Nothing uses it and it uses nothing. Possible dead code, or an entry point such as a standalone tool. |
| Large | 500 or more lines. |

### Inspector actions
Select a script and use:
- **Open** to open it with your default editor.
- **Show folder** to reveal it in the file manager.
- **Copy path** to put its full path on the clipboard.
- The **Uses** and **Used by** lists jump to the linked script when clicked.

### Export report
**Export** saves a Markdown report: project type, score, every cycle with its files, hubs, isolated scripts, the largest scripts and a language count. Good for code reviews, tickets, or tracking the score over time.

### Rescan
Edited code? Press `F5`. The graph rebuilds and blooms in again.

## 7. The spaghetti score

The score runs from 0 (clean) to 100 (spaghetti):

- Up to **60 points** for scripts caught in circular dependencies (40% of scripts in cycles reaches the full 60).
- Up to **25 points** for hubs (about 12% of scripts being hubs reaches the full 25).
- Up to **15 points** for general busyness (an average of 8 or more links per script).

| Score | Verdict |
|---|---|
| under 20 | Clean |
| 20 to 39 | Healthy |
| 40 to 64 | Tangled |
| 65 and up | Spaghetti |

It is a guide, not a grade. A small tool with one tidy cycle can score higher than a huge calm codebase, so always open the Problems list to see why.

## 8. What gets scanned

**Scanned:** TypeScript, JavaScript, Rust, Go, C#, C++, C, Java, Kotlin, Python, Lua and GDScript.

**Skipped on purpose:**
- Package and vendor code: `node_modules`, `bower_components`, `vendor`, `third_party`, `external`, `deps`, `Pods`, `site-packages`, `venv`, Unity `PackageCache` and `TextMesh Pro`.
- Build output and caches: `target`, `dist`, `build`, `bin`, `obj`, `Library`, `Temp`, `Logs`, `Intermediate`, `Binaries`, `Saved`.
- Hidden folders (starting with `.`), symlinks, files over 400 KB.
- Generated files: `.d.ts`, `.min.js`, `.g.cs`, `.generated.cs`, `.designer.cs`, `.pb.go`.

Edit `IGNORED_DIRS` in `src/common/models/script_scanner_model.rs` to change the list. Folder names are compared in lower case.

**Size limit:** up to 900 scripts are drawn, shallowest folders first. If your project is bigger, the status bar says how many were skipped.

## 9. How links are found

| Language | How a "uses" link is detected |
|---|---|
| TypeScript, JavaScript | `import`, `export ... from`, `require()`, dynamic `import()`, relative paths and `@/`, `~/`, `src/` aliases |
| Rust | `mod x;` and `use crate::...` |
| Go | `import` lines, matched to the package folder |
| Python | `import` and `from ... import`, including relative imports |
| C, C++ | `#include "..."` |
| Lua | `require` |
| GDScript | `preload` / `load` of `res://` paths |
| C#, Java, Kotlin | A script mentions a class, struct, interface or enum declared in another script |

It is a heuristic, not a compiler. Links to packages are ignored. In Unity, links through prefabs, scenes or the Inspector are not visible, only what the code itself mentions.

## 10. Themes

Themes are YAML files in the `themes` folder inside the app data folder (see section 11). Example:

```yaml
name: My Theme
colors:
  bg: "#232323"      # graph background
  panel: "#383838"   # toolbars and side panels
  field: "#2A2A2A"   # inputs, buttons, lists
  border: "#1A1A1A"  # thin lines
  text: "#D2D2D2"    # main text
  dim: "#8C8C8C"     # secondary text
  accent: "#5AA0E6"  # highlights
  select: "#2C5D87"  # selected rows, primary buttons
  grid: "#2E2E2E"    # graph grid
  node: "#2E2E2E"    # node body
  danger: "#E5534B"  # cycles and warnings
```

1. Copy an existing file, change the `name` and colors.
2. Open the launcher, go to **Themes**, press **Reload**.
3. Click your card. The choice is remembered.

Colors are hex: `#rrggbb` (or `#rrggbbaa`). A missing color falls back to Unity Dark. A broken file is reported in the status bar and the other themes keep working. Dark or light is chosen automatically from the `bg` color.

## 11. Where things are saved

| OS | Folder |
|---|---|
| Windows | `%APPDATA%\Quark-Sextant` |
| macOS | `~/Library/Application Support/Quark-Sextant` |
| Linux | `~/.config/Quark-Sextant` |

Inside: `settings.json` (theme, layout mode, last project), `recent.json` (the project list) and `themes/`. Delete a file to reset it.

## 12. Tips for different projects

- **Node.js / API / WebSocket:** try Flow. Entry files such as `server` or `index` sit on the left. A route file that every other file imports shows up as a hub.
- **Unity:** try Grouped. Folders such as `Player`, `UI` and `Enemies` become boxes, and the lines between boxes show cross-feature coupling. Managers and singletons appear as hubs.
- **Cocos / Godot:** same as Unity. Godot `preload` paths are followed.
- **Go:** links go to a package, so a node stands for the package's main file. Big standard-library-style trees give many small boxes in Grouped mode.
- **Clean up plan:** open Problems, fix the biggest cycle first, press `F5`, and watch the score drop. Export a report before and after.

## 13. Troubleshooting

| Problem | What to try |
|---|---|
| Graph is empty | The folder may only hold skipped folders or unsupported languages. Select the folder that contains the source. |
| "Showing the first 900 scripts" | Open a sub-folder of the project instead. |
| Too many links to read | Select a script to fade the rest, or search for a folder name. |
| Theme not applied | Check the status bar for a theme problem (usually a color that is not hex). |
| The launcher says it could not start the viewer | Make sure `quark-sextant` is in the same folder as `quark-sextant-launcher`. |
| Folder dialog missing on Linux | Install `libgtk-3-dev`, then rebuild. |

## 14. Good to know

- Quark-Sextant only reads your files. It never changes your project.
- Moving a node does not save anything. Rescan or Re-layout resets positions.
- Everything runs on your machine, with no network use.
