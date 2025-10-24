//! Graph representation and insertion orchestration.

use std::collections::HashMap;

use rayon::prelude::*;

use crate::datasource::DataSource;

use super::node::{Node, TrimResultInternal};
use super::search::validate_batch_distances;
use super::types::{
    ApplyContext, EntryPoint, InsertionPlan, LayerPlan, Neighbour, NodeContext, PreparedInsertion,
    TrimResult,
};
use super::{HnswError, HnswParams};

/// Backing graph for the HNSW index.
#[derive(Debug)]
pub(crate) struct Graph {
    params: HnswParams,
    nodes: Vec<Option<Node>>,
    entry: Option<EntryPoint>,
}

impl Graph {
    /// Creates a new graph with optional preallocated capacity.
    #[must_use]
    pub(crate) fn new(params: HnswParams, capacity: usize) -> Self {
        let mut nodes = Vec::with_capacity(capacity);
        nodes.resize_with(capacity, || None);
        Self {
            params,
            nodes,
            entry: None,
        }
    }

    /// Returns the current entry point.
    #[must_use]
    pub(crate) fn entry(&self) -> Option<EntryPoint> {
        self.entry
    }

    /// Accesses a node immutably.
    pub(crate) fn node(&self, node: usize) -> Option<&Node> {
        self.nodes.get(node).and_then(Option::as_ref)
    }

    /// Accesses a node mutably.
    fn node_mut(&mut self, node: usize) -> Option<&mut Node> {
        self.nodes.get_mut(node).and_then(Option::as_mut)
    }

    fn ensure_capacity(&mut self, capacity: usize) {
        if self.nodes.len() < capacity {
            self.nodes.resize_with(capacity, || None);
        }
    }

    /// Allocates space for a node at `level`.
    pub(crate) fn attach_node(&mut self, node: usize, level: usize) -> Result<(), HnswError> {
        if level > self.params.max_level() {
            return Err(HnswError::InvalidParameters {
                reason: format!(
                    "level {level} exceeds max_level {}",
                    self.params.max_level()
                ),
            });
        }
        self.ensure_capacity(node + 1);
        let slot = self
            .nodes
            .get_mut(node)
            .ok_or_else(|| HnswError::InvalidParameters {
                reason: format!("node {node} is outside pre-allocated capacity"),
            })?;
        if slot.is_some() {
            return Err(HnswError::DuplicateNode { node });
        }
        *slot = Some(Node::new(level));
        Ok(())
    }

    /// Inserts the first node into an empty graph.
    pub(crate) fn insert_first(&mut self, ctx: NodeContext) -> Result<(), HnswError> {
        if self.entry.is_some() {
            return Err(HnswError::InvalidParameters {
                reason: "graph already has an entry point".into(),
            });
        }
        self.attach_node(ctx.node, ctx.level)?;
        self.entry = Some(EntryPoint {
            node: ctx.node,
            level: ctx.level,
        });
        Ok(())
    }

    /// Plans neighbours for the new node by scanning existing vertices.
    pub(crate) fn plan_insertion<D: DataSource + Sync>(
        &self,
        ctx: NodeContext,
        params: &HnswParams,
        source: &D,
    ) -> Result<InsertionPlan, HnswError> {
        if self.entry.is_none() {
            return Err(HnswError::InvalidParameters {
                reason: "cannot plan insertion without an entry point".into(),
            });
        }

        let mut candidate_ids = Vec::new();
        for (node_id, slot) in self.nodes.iter().enumerate() {
            if slot.is_some() && node_id != ctx.node {
                candidate_ids.push(node_id);
            }
        }

        if candidate_ids.is_empty() {
            return Ok(InsertionPlan { layers: Vec::new() });
        }

        let distances = validate_batch_distances(source, ctx.node, &candidate_ids)?;
        let mut scored: Vec<Neighbour> = candidate_ids
            .into_iter()
            .zip(distances)
            .map(|(id, distance)| Neighbour { id, distance })
            .collect();
        scored.sort_unstable_by(|a, b| a.distance.total_cmp(&b.distance));
        let limit = params.max_connections();

        let mut layers = Vec::new();
        for level in 0..=ctx.level {
            let mut layer = LayerPlan {
                level,
                neighbours: scored.iter().take(limit).copied().collect(),
            };
            layer.sort_neighbours();
            layers.push(layer);
        }
        Ok(InsertionPlan { layers })
    }

    /// Applies the insertion plan, computes trim results, and commits the update.
    pub(crate) fn insert_node<D: DataSource + Sync>(
        &mut self,
        ctx: NodeContext,
        params: &HnswParams,
        source: &D,
    ) -> Result<(), HnswError> {
        let plan = self
            .plan_insertion(ctx, params, source)?
            .take_for_level(ctx.level);
        let (prepared, trim_jobs) = self.apply_insertion(ctx, ApplyContext { params, plan })?;

        let trim_results = trim_jobs
            .into_par_iter()
            .map(|mut job| -> Result<TrimResultInternal, HnswError> {
                job.prioritise(ctx.node);
                let distances = validate_batch_distances(source, job.node, &job.candidates)?;
                let mut combined: Vec<_> = job
                    .candidates
                    .into_iter()
                    .zip(distances.into_iter())
                    .collect();
                combined.sort_unstable_by(|a, b| a.1.total_cmp(&b.1));
                combined.truncate(job.ctx.max_connections);
                Ok(TrimResultInternal {
                    node: job.node,
                    ctx: job.ctx.clone(),
                    neighbours: combined.into_iter().map(|(id, _)| id).collect(),
                })
            })
            .collect::<Result<Vec<_>, HnswError>>()?;

        let public = trim_results
            .into_iter()
            .map(TrimResultInternal::into_public)
            .collect();
        self.commit_insertion(prepared, public)
    }

    /// Commits prepared updates into the graph.
    pub(crate) fn commit_insertion(
        &mut self,
        prepared: PreparedInsertion,
        trims: Vec<TrimResult>,
    ) -> Result<(), HnswError> {
        let mut trim_map: HashMap<(usize, usize), Vec<usize>> = HashMap::new();
        for trim in trims {
            trim_map.insert((trim.node, trim.level), trim.neighbours);
        }

        let slot = self.node_mut(prepared.node.node).ok_or_else(|| {
            HnswError::GraphInvariantViolation {
                message: format!("node {} missing during commit", prepared.node.node),
            }
        })?;

        for (level, neighbours) in prepared.new_node_neighbours.iter().enumerate() {
            slot.neighbours_mut(level).clone_from(neighbours);
        }

        for (node, level, candidates) in prepared.updates {
            let neighbours = trim_map.remove(&(node, level)).unwrap_or(candidates);
            let existing =
                self.node_mut(node)
                    .ok_or_else(|| HnswError::GraphInvariantViolation {
                        message: format!("node {node} missing during neighbour update"),
                    })?;
            *existing.neighbours_mut(level) = neighbours;
        }

        if prepared.promote_entry {
            self.entry = Some(EntryPoint {
                node: prepared.node.node,
                level: prepared.node.level,
            });
        }

        Ok(())
    }
}
