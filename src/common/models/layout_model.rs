use std::cmp::Ordering;
use std::collections::VecDeque;

use crate::common::common_interface::{GraphMetrics, ScriptGraph};

pub const NODE_W: f32 = 200.0;
pub const NODE_H: f32 = 64.0;
const COL_W: f32 = 280.0;
const ROW_H: f32 = 92.0;
const MAX_PER_COLUMN: usize = 14; // taller columns wrap into a second column
const GROUP_PAD: f32 = 20.0;
const GROUP_TOP: f32 = 40.0; // room for the group's title
const GROUP_GAP: f32 = 60.0;
const CELL_W: f32 = NODE_W + 30.0;
const CELL_H: f32 = NODE_H + 26.0;

/// Lays scripts out left to right like a visual-scripting graph:
/// entry points on the left, the things they depend on further right.
pub struct LayoutModel;

impl LayoutModel {
    /// Scripts of the same folder, folders in path order, scripts in name order.
    pub fn folder_groups(graph: &ScriptGraph) -> Vec<(String, Vec<usize>)> {
        let mut groups: std::collections::BTreeMap<String, Vec<usize>> = std::collections::BTreeMap::new();

        for (i, node) in graph.nodes.iter().enumerate() {
            groups.entry(node.folder.clone()).or_default().push(i);
        }

        groups
            .into_iter()
            .map(|(folder, mut members)| {
                members.sort_by(|&a, &b| graph.nodes[a].name.cmp(&graph.nodes[b].name));
                (folder, members)
            })
            .collect()
    }

    /// No flow, no ranks: each folder becomes a block of scripts, and the blocks are packed in rows.
    pub fn arrange_grouped(graph: &ScriptGraph) -> Vec<[f32; 2]> {
        let groups = Self::folder_groups(graph);
        let mut positions = vec![[0.0f32; 2]; graph.nodes.len()];

        let shapes: Vec<(usize, f32, f32)> = groups
            .iter()
            .map(|(_, members)| {
                let cols = ((members.len() as f32).sqrt().ceil() as usize).max(1);
                let rows = members.len().div_ceil(cols);
                let w = cols as f32 * CELL_W - (CELL_W - NODE_W) + GROUP_PAD * 2.0;
                let h = GROUP_TOP + rows as f32 * CELL_H - (CELL_H - NODE_H) + GROUP_PAD;
                (cols, w, h)
            })
            .collect();

        let area: f32 = shapes.iter().map(|s| s.1 * s.2).sum();
        let widest = shapes.iter().map(|s| s.1).fold(0.0, f32::max);
        let row_width = (area.sqrt() * 1.7).max(widest);
        let (mut x, mut y, mut row_height) = (0.0f32, 0.0f32, 0.0f32);

        for ((_, members), &(cols, w, h)) in groups.iter().zip(&shapes) {
            if x > 0.0 && x + w > row_width {
                x = 0.0;
                y += row_height + GROUP_GAP;
                row_height = 0.0;
            }

            for (k, &node) in members.iter().enumerate() {
                positions[node] = [x + GROUP_PAD + (k % cols) as f32 * CELL_W, y + GROUP_TOP + (k / cols) as f32 * CELL_H];
            }

            x += w + GROUP_GAP;
            row_height = row_height.max(h);
        }

        positions
    }

    /// Returns the top-left corner of every node.
    pub fn arrange(graph: &ScriptGraph, metrics: &GraphMetrics) -> Vec<[f32; 2]> {
        let n = graph.nodes.len();

        if n == 0 {
            return Vec::new();
        }

        let rank = Self::ranks(graph, metrics);
        let max_rank = rank.iter().copied().max().unwrap_or(0);
        let mut columns: Vec<Vec<usize>> = vec![Vec::new(); max_rank + 1];

        let mut order: Vec<usize> = (0..n).collect();
        order.sort_by(|&a, &b| {
            (&graph.nodes[a].folder, &graph.nodes[a].name).cmp(&(&graph.nodes[b].folder, &graph.nodes[b].name))
        });

        for i in order {
            columns[rank[i]].push(i);
        }

        // Shuffle each column so connected scripts sit near each other and lines cross less.
        let mut ins: Vec<Vec<usize>> = vec![Vec::new(); n];
        let mut outs: Vec<Vec<usize>> = vec![Vec::new(); n];

        for edge in &graph.edges {
            outs[edge.from].push(edge.to);
            ins[edge.to].push(edge.from);
        }

        let mut slot = vec![0.0f32; n];
        Self::reindex(&columns, &mut slot);

        for _ in 0..2 {
            for r in 1..columns.len() {
                Self::sweep(&mut columns[r], &ins, &slot);
                Self::reindex(&columns, &mut slot);
            }

            for r in (0..columns.len().saturating_sub(1)).rev() {
                Self::sweep(&mut columns[r], &outs, &slot);
                Self::reindex(&columns, &mut slot);
            }
        }

        let mut positions = vec![[0.0f32; 2]; n];
        let mut cursor = 0.0;

        for column in &columns {
            if column.is_empty() {
                continue;
            }

            let parts = column.len().div_ceil(MAX_PER_COLUMN);

            for part in 0..parts {
                let chunk = &column[part * MAX_PER_COLUMN..((part + 1) * MAX_PER_COLUMN).min(column.len())];
                let height = chunk.len() as f32;

                for (k, &node) in chunk.iter().enumerate() {
                    positions[node] = [cursor + part as f32 * COL_W, (k as f32 - (height - 1.0) / 2.0) * ROW_H];
                }
            }

            cursor += parts as f32 * COL_W;
        }

        positions
    }

    /// Longest path from an entry point. Scripts in the same cycle share one column.
    fn ranks(graph: &ScriptGraph, metrics: &GraphMetrics) -> Vec<usize> {
        let n = graph.nodes.len();
        let total = n + metrics.cycles.len();
        let group: Vec<usize> = (0..n).map(|i| metrics.cycle_of[i].map(|c| n + c).unwrap_or(i)).collect();

        let mut next: Vec<Vec<usize>> = vec![Vec::new(); total];
        let mut waiting = vec![0usize; total];

        for edge in &graph.edges {
            let (a, b) = (group[edge.from], group[edge.to]);

            if a != b {
                next[a].push(b);
                waiting[b] += 1;
            }
        }

        let mut rank = vec![0usize; total];
        let mut queue: VecDeque<usize> = (0..total).filter(|&g| waiting[g] == 0).collect();

        while let Some(g) = queue.pop_front() {
            for &h in &next[g] {
                rank[h] = rank[h].max(rank[g] + 1);
                waiting[h] -= 1;

                if waiting[h] == 0 {
                    queue.push_back(h);
                }
            }
        }

        (0..n).map(|i| rank[group[i]]).collect()
    }

    fn reindex(columns: &[Vec<usize>], slot: &mut [f32]) {
        for column in columns {
            for (k, &node) in column.iter().enumerate() {
                slot[node] = k as f32;
            }
        }
    }

    /// Sorts one column by the average position of each node's neighbours.
    fn sweep(column: &mut Vec<usize>, neighbours: &[Vec<usize>], slot: &[f32]) {
        let mut keyed: Vec<(f32, usize)> = column
            .iter()
            .enumerate()
            .map(|(k, &node)| {
                let list = &neighbours[node];
                let key = if list.is_empty() { k as f32 } else { list.iter().map(|&m| slot[m]).sum::<f32>() / list.len() as f32 };
                (key, node)
            })
            .collect();

        keyed.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(Ordering::Equal));
        *column = keyed.into_iter().map(|(_, node)| node).collect();
    }
}
