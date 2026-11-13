use std::collections::BTreeMap;
use std::fmt::Write;

use crate::common::common_interface::ScanReport;
use crate::common::models::graph_metrics_model::{GraphMetricsModel, LARGE_LINES};

/// Turns a scan into a Markdown report you can share or commit.
pub struct ReportExportModel;

impl ReportExportModel {
    pub fn markdown(report: &ScanReport) -> String {
        let (graph, metrics) = (&report.graph, &report.metrics);
        let name = report.root.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "project".to_string());
        let kinds: Vec<&str> = report.kinds.iter().map(|k| k.label()).collect();
        let mut out = String::new();

        let _ = writeln!(out, "# Quark-Sextant report: {name}\n");
        let _ = writeln!(out, "- Project type: {}", kinds.join(", "));
        let _ = writeln!(out, "- Scripts: {}", graph.nodes.len());
        let _ = writeln!(out, "- Links: {}", graph.edges.len());
        let _ = writeln!(out, "- Spaghetti score: {:.0} / 100 ({})\n", metrics.score, metrics.verdict());

        if graph.truncated {
            let _ = writeln!(out, "> Only the first {} scripts were scanned ({} skipped).\n", graph.nodes.len(), graph.skipped);
        }

        let _ = writeln!(out, "## Circular dependencies\n");

        if metrics.cycles.is_empty() {
            let _ = writeln!(out, "None.\n");
        }

        for (index, cycle) in metrics.cycles.iter().enumerate() {
            let _ = writeln!(out, "### Cycle {} ({} scripts)\n", index + 1, cycle.len());

            for &node in cycle.iter().take(25) {
                let _ = writeln!(out, "- `{}`", graph.nodes[node].rel_path);
            }

            if cycle.len() > 25 {
                let _ = writeln!(out, "- ...and {} more", cycle.len() - 25);
            }

            out.push('\n');
        }

        let _ = writeln!(out, "## Hubs (touch too many scripts)\n");

        if metrics.hubs.is_empty() {
            let _ = writeln!(out, "None.\n");
        } else {
            let _ = writeln!(out, "| Script | Used by | Uses |\n|---|---|---|");

            for &hub in metrics.hubs.iter().take(25) {
                let _ = writeln!(out, "| `{}` | {} | {} |", graph.nodes[hub].rel_path, metrics.fan_in[hub], metrics.fan_out[hub]);
            }

            out.push('\n');
        }

        let _ = writeln!(out, "## Isolated scripts (no links, maybe dead code)\n");

        if metrics.orphans.is_empty() {
            let _ = writeln!(out, "None.\n");
        }

        for &node in metrics.orphans.iter().take(50) {
            let _ = writeln!(out, "- `{}`", graph.nodes[node].rel_path);
        }

        let _ = writeln!(out, "\n## Largest scripts ({LARGE_LINES}+ lines)\n");
        let large = GraphMetricsModel::large_scripts(graph, 15);

        if large.is_empty() {
            let _ = writeln!(out, "None.\n");
        }

        for node in large {
            let _ = writeln!(out, "- `{}`: {} lines", graph.nodes[node].rel_path, graph.nodes[node].lines);
        }

        let mut languages: BTreeMap<&str, usize> = BTreeMap::new();

        for node in &graph.nodes {
            *languages.entry(node.language.label()).or_default() += 1;
        }

        let _ = writeln!(out, "\n## Languages\n");

        for (language, count) in languages {
            let _ = writeln!(out, "- {language}: {count}");
        }

        out
    }
}
