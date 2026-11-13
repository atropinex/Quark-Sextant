use crate::common::common_interface::{GraphMetrics, ScriptEdge, ScriptGraph};

const HUB_FAN_OUT: usize = 12; // uses this many other scripts
const HUB_FAN_IN: usize = 15;  // used by this many other scripts
pub const LARGE_LINES: usize = 500; // a script this long is worth a look

/// Finds the things that make code spaghetti: scripts that depend on each other in circles,
/// and scripts that touch too many others.
pub struct GraphMetricsModel;

impl GraphMetricsModel {
    pub fn compute(count: usize, edges: &[ScriptEdge]) -> GraphMetrics {
        let mut fan_in = vec![0; count];
        let mut fan_out = vec![0; count];
        let mut adjacency: Vec<Vec<usize>> = vec![Vec::new(); count];

        for edge in edges {
            fan_out[edge.from] += 1;
            fan_in[edge.to] += 1;
            adjacency[edge.from].push(edge.to);
        }

        let mut cycles: Vec<Vec<usize>> = Self::strongly_connected(&adjacency).into_iter().filter(|c| c.len() > 1).collect();
        cycles.sort_by_key(|c| std::cmp::Reverse(c.len()));

        let mut cycle_of = vec![None; count];

        for (index, cycle) in cycles.iter().enumerate() {
            for &node in cycle {
                cycle_of[node] = Some(index);
            }
        }

        let edge_in_cycle: Vec<bool> = edges
            .iter()
            .map(|e| cycle_of[e.from].is_some() && cycle_of[e.from] == cycle_of[e.to])
            .collect();

        let mut hubs: Vec<usize> = (0..count).filter(|&i| fan_out[i] >= HUB_FAN_OUT || fan_in[i] >= HUB_FAN_IN).collect();
        hubs.sort_by_key(|&i| std::cmp::Reverse(fan_in[i] + fan_out[i]));

        let score = Self::score(count, edges.len(), cycle_of.iter().flatten().count(), hubs.len());

        let orphans: Vec<usize> = (0..count).filter(|&i| fan_in[i] == 0 && fan_out[i] == 0).collect();

        GraphMetrics { fan_in, fan_out, cycle_of, cycles, edge_in_cycle, hubs, orphans, score }
    }

    /// The longest scripts, longest first.
    pub fn large_scripts(graph: &ScriptGraph, limit: usize) -> Vec<usize> {
        let mut list: Vec<usize> = (0..graph.nodes.len()).filter(|&i| graph.nodes[i].lines >= LARGE_LINES).collect();
        list.sort_by_key(|&i| std::cmp::Reverse(graph.nodes[i].lines));
        list.truncate(limit);
        list
    }

    /// 60 points for scripts stuck in cycles, 25 for hubs, 15 for general busyness.
    fn score(nodes: usize, edges: usize, in_cycles: usize, hubs: usize) -> f32 {
        if nodes == 0 {
            return 0.0;
        }

        let n = nodes as f32;
        let cycle_part = (in_cycles as f32 / n * 2.5).min(1.0) * 60.0;
        let hub_part = (hubs as f32 / n * 8.0).min(1.0) * 25.0;
        let busy_part = (edges as f32 / n / 8.0).min(1.0) * 15.0;

        (cycle_part + hub_part + busy_part).clamp(0.0, 100.0)
    }

    /// Tarjan's algorithm, written with a work list so huge projects cannot overflow the stack.
    fn strongly_connected(adjacency: &[Vec<usize>]) -> Vec<Vec<usize>> {
        let n = adjacency.len();
        let mut index = vec![usize::MAX; n];
        let mut low = vec![0usize; n];
        let mut on_stack = vec![false; n];
        let mut stack: Vec<usize> = Vec::new();
        let mut result = Vec::new();
        let mut counter = 0;

        for root in 0..n {
            if index[root] != usize::MAX {
                continue;
            }

            index[root] = counter;
            low[root] = counter;
            counter += 1;
            stack.push(root);
            on_stack[root] = true;

            let mut work: Vec<(usize, usize)> = vec![(root, 0)]; // (node, next neighbour to look at)

            while let Some(&(v, i)) = work.last() {
                if i < adjacency[v].len() {
                    if let Some(top) = work.last_mut() {
                        top.1 += 1;
                    }

                    let w = adjacency[v][i];

                    if index[w] == usize::MAX {
                        index[w] = counter;
                        low[w] = counter;
                        counter += 1;
                        stack.push(w);
                        on_stack[w] = true;
                        work.push((w, 0));
                    } else if on_stack[w] {
                        low[v] = low[v].min(index[w]);
                    }
                } else {
                    work.pop();

                    if let Some(&(parent, _)) = work.last() {
                        low[parent] = low[parent].min(low[v]);
                    }

                    if low[v] == index[v] {
                        let mut group = Vec::new();

                        while let Some(w) = stack.pop() {
                            on_stack[w] = false;
                            group.push(w);

                            if w == v {
                                break;
                            }
                        }

                        result.push(group);
                    }
                }
            }
        }

        result
    }
}
