// Written to the themes folder the first time the app runs. After that the files belong to you.
pub const DEFAULT_THEMES: &[(&str, &str)] = &[
    ("unity-dark.yaml", r##"# Themes are plain YAML files. Edit this one, or copy it to make your own,
# then press "Reload" in the launcher's Themes page. Leave a color out to keep the default.
name: Unity Dark
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
"##),
    ("unity-light.yaml", r##"name: Unity Light
colors:
  bg: "#BDBDBD"
  panel: "#C8C8C8"
  field: "#E4E4E4"
  border: "#8A8A8A"
  text: "#111111"
  dim: "#555555"
  accent: "#1C5FAF"
  select: "#3A72B0"
  grid: "#B0B0B0"
  node: "#DCDCDC"
  danger: "#C62828"
"##),
    ("midnight.yaml", r##"name: Midnight
colors:
  bg: "#12111C"
  panel: "#1D1B2B"
  field: "#171523"
  border: "#0C0B14"
  text: "#E6E3F5"
  dim: "#8F8BAE"
  accent: "#B69CFF"
  select: "#4B3BB5"
  grid: "#1A1828"
  node: "#1F1D30"
  danger: "#FF6B7A"
"##),
    ("ember.yaml", r##"name: Ember
colors:
  bg: "#1C1411"
  panel: "#2B1F1A"
  field: "#231913"
  border: "#120C09"
  text: "#F2E6DF"
  dim: "#A68E80"
  accent: "#FFB86B"
  select: "#8A4A1F"
  grid: "#261B16"
  node: "#2E2019"
  danger: "#FF5A4D"
"##),
];
