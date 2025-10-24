//! Search helpers for the HNSW graph.

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashSet};

use crate::datasource::DataSource;

use super::graph::Graph;
use super::types::{ExtendedSearchContext, Neighbour, SearchContext};
use super::{HnswError, HnswParams};

/// Performs a full HNSW search across all layers.
pub(crate) fn search<D: DataSource + Sync>(
    graph: &Graph,
    query: usize,
    k: usize,
    params: &HnswParams,
    source: &D,
) -> Result<Vec<Neighbour>, HnswError> {
    let Some(entry) = graph.entry() else {
        return Ok(Vec::new());
    };
    let mut current = entry.node;
    let mut current_dist = validate_distance(source, query, current)?;

    let mut level = entry.level;
    while level > 0 {
        let node = graph
            .node(current)
            .ok_or_else(|| HnswError::GraphInvariantViolation {
                message: format!("node {current} missing during descent"),
            })?;
        let neighbours = node.neighbours(level);
        if neighbours.is_empty() {
            level -= 1;
            continue;
        }
        let distances = validate_batch_distances(source, query, neighbours)?;
        let mut improved = false;
        for (&candidate, &distance) in neighbours.iter().zip(distances.iter()) {
            if distance < current_dist {
                current = candidate;
                current_dist = distance;
                improved = true;
            }
        }
        if !improved {
            level -= 1;
        }
    }

    let base = SearchContext {
        query,
        entry: current,
        level: 0,
    };
    let mut results = graph.search_layer(source, base.with_ef(params.ef_construction()))?;
    results.truncate(k);
    Ok(results)
}

/// Validates a single distance provided by the data source.
pub(crate) fn validate_distance<D: DataSource + Sync>(
    source: &D,
    query: usize,
    candidate: usize,
) -> Result<f32, HnswError> {
    let distance = source.distance(query, candidate)?;
    if !distance.is_finite() {
        return Err(HnswError::InvalidParameters {
            reason: format!(
                "non-finite distance returned for query {query} and candidate {candidate}"
            ),
        });
    }
    Ok(distance)
}

/// Validates a batch of distances provided by the data source.
pub(crate) fn validate_batch_distances<D: DataSource + Sync>(
    source: &D,
    query: usize,
    candidates: &[usize],
) -> Result<Vec<f32>, HnswError> {
    let distances = source.batch_distances(query, candidates)?;
    if distances.iter().any(|distance| !distance.is_finite()) {
        return Err(HnswError::InvalidParameters {
            reason: format!("non-finite distance returned in batch for query {query}"),
        });
    }
    Ok(distances)
}

#[derive(Clone, Debug)]
struct ReverseNeighbour {
    inner: Neighbour,
}

impl ReverseNeighbour {
    fn new(id: usize, distance: f32) -> Self {
        Self {
            inner: Neighbour { id, distance },
        }
    }
}

impl Eq for ReverseNeighbour {}

impl Ord for ReverseNeighbour {
    fn cmp(&self, other: &Self) -> Ordering {
        other.inner.distance.total_cmp(&self.inner.distance)
    }
}

impl PartialOrd for ReverseNeighbour {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for ReverseNeighbour {
    fn eq(&self, other: &Self) -> bool {
        self.inner.distance == other.inner.distance && self.inner.id == other.inner.id
    }
}

impl Graph {
    /// Searches a single layer using best-first exploration.
    pub(crate) fn search_layer<D: DataSource + Sync>(
        &self,
        source: &D,
        ctx: ExtendedSearchContext,
    ) -> Result<Vec<Neighbour>, HnswError> {
        let entry_dist = validate_distance(source, ctx.base.query, ctx.base.entry)?;
        let mut visited = HashSet::new();
        visited.insert(ctx.base.entry);

        let mut candidates = BinaryHeap::new();
        candidates.push(ReverseNeighbour::new(ctx.base.entry, entry_dist));

        let mut best = BinaryHeap::new();
        best.push(Neighbour {
            id: ctx.base.entry,
            distance: entry_dist,
        });

        while let Some(ReverseNeighbour { inner }) = candidates.pop() {
            if best.len() >= ctx.ef {
                if let Some(furthest) = best.peek() {
                    if inner.distance > furthest.distance {
                        break;
                    }
                } else {
                    continue;
                }
            }

            let node = self
                .node(inner.id)
                .ok_or_else(|| HnswError::GraphInvariantViolation {
                    message: format!("node {} missing during layer search", inner.id),
                })?;

            let fresh: Vec<usize> = node
                .neighbours(ctx.base.level)
                .iter()
                .copied()
                .filter(|id| visited.insert(*id))
                .collect();
            if fresh.is_empty() {
                continue;
            }

            let dists = validate_batch_distances(source, ctx.base.query, &fresh)?;
            for (&cand, &dist) in fresh.iter().zip(dists.iter()) {
                let should_add = if best.len() < ctx.ef {
                    true
                } else if let Some(furthest) = best.peek() {
                    dist < furthest.distance
                } else {
                    false
                };
                if !should_add {
                    continue;
                }
                candidates.push(ReverseNeighbour::new(cand, dist));
                best.push(Neighbour {
                    id: cand,
                    distance: dist,
                });
                if best.len() > ctx.ef {
                    best.pop();
                }
            }
        }

        let mut result = best.into_vec();
        result.sort_unstable_by(|a, b| a.distance.total_cmp(&b.distance));
        Ok(result)
    }
}
