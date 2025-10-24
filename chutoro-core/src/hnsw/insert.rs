//! Insertion helpers for the HNSW graph.

use super::HnswError;
use super::graph::Graph;
use super::node::{CandidateMap, TrimJob};
use super::types::{ApplyContext, EdgeContext, NodeContext, PreparedInsertion};

impl Graph {
    /// Applies a planned insertion and schedules trimming jobs in a single pass.
    pub(crate) fn apply_insertion(
        &mut self,
        node: NodeContext,
        ctx: ApplyContext<'_>,
    ) -> Result<(PreparedInsertion, Vec<TrimJob>), HnswError> {
        let ApplyContext { params, mut plan } = ctx;
        self.attach_node(node.node, node.level)?;
        let mut new_node_neighbours = vec![Vec::new(); node.level + 1];
        let mut staged = CandidateMap::default();
        let mut trim_jobs = Vec::new();

        for layer in plan.layers.drain(..) {
            if layer.level > node.level {
                continue;
            }
            let edge_ctx = EdgeContext::for_level(params, layer.level);
            for neighbour in layer.neighbours.into_iter().take(edge_ctx.max_connections) {
                if neighbour.id == node.node {
                    continue;
                }
                let layer_vec = new_node_neighbours.get_mut(layer.level).ok_or_else(|| {
                    HnswError::GraphInvariantViolation {
                        message: format!("layer {} missing when staging insertion", layer.level),
                    }
                })?;
                if !layer_vec.contains(&neighbour.id) {
                    layer_vec.push(neighbour.id);
                }

                let key = (neighbour.id, layer.level);
                let candidates = staged.entry_mut(key);
                if candidates.is_empty() {
                    let existing = self.node(neighbour.id).ok_or_else(|| {
                        HnswError::GraphInvariantViolation {
                            message: format!(
                                "node {} missing while staging insertion",
                                neighbour.id
                            ),
                        }
                    })?;
                    candidates.extend_from_slice(existing.neighbours(layer.level));
                }
                if !candidates.contains(&node.node) {
                    candidates.push(node.node);
                }

                if candidates.len() > edge_ctx.max_connections {
                    let mut job = TrimJob {
                        node: neighbour.id,
                        ctx: edge_ctx.clone(),
                        candidates: candidates.clone(),
                    };
                    job.prioritise(node.node);
                    trim_jobs.push(job);
                }
            }
        }

        let promote_entry = self.entry().is_none_or(|entry| node.level > entry.level);

        Ok((
            PreparedInsertion {
                node,
                promote_entry,
                new_node_neighbours,
                updates: staged.into_vec(),
            },
            trim_jobs,
        ))
    }
}
