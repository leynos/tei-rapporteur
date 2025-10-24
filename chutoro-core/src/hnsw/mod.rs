//! CPU-backed Hierarchical Navigable Small World (HNSW) index.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, RwLock};

use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use thiserror::Error;

use crate::datasource::{DataSource, DataSourceError};

mod graph;
mod insert;
mod node;
mod search;
pub(crate) mod types;

use graph::Graph;
pub use types::Neighbour;
use types::NodeContext;

/// Parameters controlling the HNSW topology.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HnswParams {
    max_level: usize,
    max_connections: usize,
    ef_construction: usize,
}

impl HnswParams {
    /// Creates new parameters after validating invariants.
    ///
    /// # Panics
    ///
    /// Panics if any of the provided values are zero, as HNSW requires strictly
    /// positive configuration parameters.
    #[must_use]
    pub fn new(max_level: usize, max_connections: usize, ef_construction: usize) -> Self {
        assert!(max_level > 0, "max_level must be positive");
        assert!(max_connections > 0, "max_connections must be positive");
        assert!(ef_construction > 0, "ef_construction must be positive");
        Self {
            max_level,
            max_connections,
            ef_construction,
        }
    }

    /// Maximum layer index.
    #[must_use]
    pub fn max_level(&self) -> usize {
        self.max_level
    }

    /// Maximum number of neighbours per layer.
    #[must_use]
    pub fn max_connections(&self) -> usize {
        self.max_connections
    }

    /// Search width used during construction.
    #[must_use]
    pub fn ef_construction(&self) -> usize {
        self.ef_construction
    }
}

/// Errors surfaced by the HNSW index.
#[derive(Debug, Error)]
pub enum HnswError {
    /// Parameters supplied by the caller were invalid.
    #[error("invalid parameters: {reason}")]
    InvalidParameters {
        /// Explanation of the invalid parameter combination.
        reason: String,
    },
    /// The caller attempted to insert a node that already exists.
    #[error("node {node} already exists in the graph")]
    DuplicateNode {
        /// Identifier of the node supplied more than once.
        node: usize,
    },
    /// Internal graph invariants were violated.
    #[error("graph invariant violated: {message}")]
    GraphInvariantViolation {
        /// Description of the violated invariant.
        message: String,
    },
    /// A data source reported an error while computing distances.
    #[error(transparent)]
    DataSource(#[from] DataSourceError),
}

/// CPU-backed HNSW index with thread-safe access.
#[derive(Debug)]
pub struct CpuHnsw {
    params: HnswParams,
    graph: RwLock<graph::Graph>,
    len: AtomicUsize,
    rng: Mutex<SmallRng>,
}

impl CpuHnsw {
    /// Creates a new index with the supplied `params` and initial capacity.
    #[must_use]
    pub fn new(params: HnswParams, capacity: usize, seed: u64) -> Self {
        let graph = Graph::new(params.clone(), capacity);
        Self {
            params,
            graph: RwLock::new(graph),
            len: AtomicUsize::new(0),
            rng: Mutex::new(SmallRng::seed_from_u64(seed)),
        }
    }

    /// Number of nodes currently stored in the index.
    #[must_use]
    pub fn len(&self) -> usize {
        self.len.load(Ordering::Relaxed)
    }

    /// Returns `true` when the index contains no nodes.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn read_graph<R>(&self, f: impl FnOnce(&graph::Graph) -> R) -> R {
        let guard = self
            .graph
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        f(&guard)
    }

    fn write_graph<R>(&self, f: impl FnOnce(&mut graph::Graph) -> R) -> R {
        let mut guard = self
            .graph
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        f(&mut guard)
    }

    fn sample_level(&self) -> usize {
        let mut rng = self
            .rng
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut level = 0;
        while level < self.params.max_level && rng.gen_bool(0.5) {
            level += 1;
        }
        level
    }

    /// Inserts a node into the graph using the provided [`DataSource`].
    ///
    /// # Errors
    ///
    /// Returns [`HnswError`] when the node already exists, when its sampled
    /// level exceeds [`HnswParams::max_level`], or when distance validation
    /// fails during insertion.
    pub fn insert<D: DataSource + Sync>(&self, node: usize, source: &D) -> Result<(), HnswError> {
        let ctx = NodeContext {
            node,
            level: self.sample_level(),
        };

        if !self.read_graph(|graph| graph.entry().is_some()) {
            search::validate_distance(source, node, node)?;
            let inserted = self.write_graph(|graph| {
                if graph.entry().is_some() {
                    return Ok::<bool, HnswError>(false);
                }
                graph.insert_first(ctx)?;
                Ok(true)
            })?;
            if inserted {
                self.len.store(1, Ordering::Relaxed);
                return Ok(());
            }
        }

        self.write_graph(|graph| graph.insert_node(ctx, &self.params, source))?;
        self.len.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    /// Searches for the `k` nearest neighbours to `query`.
    ///
    /// # Errors
    ///
    /// Returns [`HnswError`] if the data source reports an error while
    /// computing distances or if graph invariants are violated during search.
    pub fn search<D: DataSource + Sync>(
        &self,
        query: usize,
        k: usize,
        source: &D,
    ) -> Result<Vec<Neighbour>, HnswError> {
        self.read_graph(|graph| search::search(graph, query, k, &self.params, source))
    }
}
