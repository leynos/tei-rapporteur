//! Shared type definitions for the HNSW graph implementation.

use std::cmp::Ordering;

use super::HnswParams;

/// Identifies a node alongside the highest layer it participates in.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct NodeContext {
    pub(crate) node: usize,
    pub(crate) level: usize,
}

/// Entry point for navigating the layered graph.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct EntryPoint {
    pub(crate) node: usize,
    pub(crate) level: usize,
}

/// Captures a query, entry node, and target level for search.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct SearchContext {
    pub(crate) query: usize,
    pub(crate) entry: usize,
    pub(crate) level: usize,
}

impl SearchContext {
    /// Extends the context with a search width parameter.
    pub(crate) fn with_ef(self, ef: usize) -> ExtendedSearchContext {
        ExtendedSearchContext { base: self, ef }
    }
}

/// Adds a search width to the base context.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ExtendedSearchContext {
    pub(crate) base: SearchContext,
    pub(crate) ef: usize,
}

/// Context for trimming edges to enforce maximum degree.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct EdgeContext {
    pub(crate) level: usize,
    pub(crate) max_connections: usize,
}

impl EdgeContext {
    /// Builds the context for a specific level.
    #[must_use]
    pub(crate) fn for_level(params: &HnswParams, level: usize) -> Self {
        Self {
            level,
            max_connections: params.max_connections(),
        }
    }
}

/// Planned neighbours for a layer.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct LayerPlan {
    pub(crate) level: usize,
    pub(crate) neighbours: Vec<Neighbour>,
}

impl LayerPlan {
    /// Ensures neighbours are sorted ascending by distance.
    pub(crate) fn sort_neighbours(&mut self) {
        self.neighbours
            .sort_unstable_by(|a, b| a.distance.total_cmp(&b.distance));
    }
}

/// Neighbour reference used during planning and search.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Neighbour {
    /// Node identifier referenced by the neighbour.
    pub id: usize,
    /// Metric distance between the query and the neighbour.
    pub distance: f32,
}

impl Eq for Neighbour {}

impl Ord for Neighbour {
    fn cmp(&self, other: &Self) -> Ordering {
        self.distance.total_cmp(&other.distance)
    }
}

impl PartialOrd for Neighbour {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Complete insertion plan for a node.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct InsertionPlan {
    pub(crate) layers: Vec<LayerPlan>,
}

impl InsertionPlan {
    /// Filters the plan to layers within the provided level.
    pub(crate) fn take_for_level(mut self, level: usize) -> Self {
        self.layers.retain(|layer| layer.level <= level);
        self
    }
}

/// Context captured when applying a prepared insertion.
#[derive(Debug)]
pub(crate) struct ApplyContext<'a> {
    pub(crate) params: &'a HnswParams,
    pub(crate) plan: InsertionPlan,
}

/// Prepared adjacency updates for a node insertion.
#[derive(Debug, PartialEq)]
pub(crate) struct PreparedInsertion {
    pub(crate) node: NodeContext,
    pub(crate) promote_entry: bool,
    pub(crate) new_node_neighbours: Vec<Vec<usize>>,
    pub(crate) updates: Vec<(usize, usize, Vec<usize>)>,
}

/// Result of trimming a node's candidate list.
#[derive(Debug, PartialEq)]
pub(crate) struct TrimResult {
    pub(crate) node: usize,
    pub(crate) level: usize,
    pub(crate) neighbours: Vec<usize>,
}
