//! Behavioural and unit tests for the HNSW index.

use super::graph::Graph;
use super::types::NodeContext;
use super::{search, CpuHnsw, HnswError, HnswParams};
use crate::datasource::{DataSource, DataSourceError};
use rstest::{fixture, rstest};

#[derive(Clone, Debug)]
struct LineSource {
    values: Vec<f32>,
}

impl LineSource {
    fn new(values: Vec<f32>) -> Self {
        Self { values }
    }

    fn point(&self, index: usize) -> Result<f32, DataSourceError> {
        self.values
            .get(index)
            .copied()
            .ok_or(DataSourceError::OutOfBounds { index })
    }
}

impl DataSource for LineSource {
    fn distance(&self, query: usize, candidate: usize) -> Result<f32, DataSourceError> {
        let query = self.point(query)?;
        let candidate = self.point(candidate)?;
        #[allow(clippy::float_arithmetic)] // Euclidean distance requires float subtraction.
        {
            Ok((query - candidate).abs())
        }
    }

    fn batch_distances(
        &self,
        query: usize,
        candidates: &[usize],
    ) -> Result<Vec<f32>, DataSourceError> {
        let origin = self.point(query)?;
        let mut distances = Vec::with_capacity(candidates.len());
        for &candidate in candidates {
            let target = self.point(candidate)?;
            #[allow(clippy::float_arithmetic)] // Euclidean distance requires float subtraction.
            {
                distances.push((origin - target).abs());
            }
        }
        Ok(distances)
    }
}

#[fixture]
fn params() -> HnswParams {
    HnswParams::new(2, 2, 2)
}

#[fixture]
fn source() -> LineSource {
    LineSource::new(vec![0.0, 0.2, 0.4, 0.6, 0.8])
}

#[rstest]
#[case(2, 2)]
#[case(2, 1)]
fn builds_and_searches(#[case] m: usize, #[case] ef: usize) {
    let params = HnswParams::new(2, m, ef);
    let source = source();
    let index = CpuHnsw::new(params.clone(), source.values.len(), 7);

    for node in 0..source.values.len() {
        index
            .insert(node, &source)
            .expect("insertion must succeed");
    }

    let results = index
        .search(0, 3, &source)
        .expect("search must succeed");
    assert_eq!(results.first().map(|n| n.id), Some(0));
    if ef == 1 {
        assert_eq!(results.len(), 1);
    } else {
        assert!(results.len() >= 2);
        assert_eq!(results[1].id, 1);
    }
}

#[rstest]
fn attach_node_rejects_large_level(mut params: HnswParams, source: LineSource) {
    let mut graph = Graph::new(params.clone(), 4);
    let ctx = NodeContext { node: 0, level: params.max_level() + 1 };
    let err = graph.attach_node(ctx.node, ctx.level).unwrap_err();
    assert!(matches!(err, HnswError::InvalidParameters { .. }));

    graph
        .insert_first(NodeContext { node: 0, level: 0 })
        .expect("first insertion must succeed");
    graph
        .insert_node(NodeContext { node: 1, level: 0 }, &params, &source)
        .expect("second insertion must succeed");
}

#[rstest]
fn duplicate_insertion_fails(mut params: HnswParams, source: LineSource) {
    let index = CpuHnsw::new(params.clone(), source.values.len(), 11);
    index
        .insert(0, &source)
        .expect("first insertion must succeed");
    let err = index.insert(0, &source).unwrap_err();
    assert!(matches!(err, HnswError::DuplicateNode { .. }));
}

#[rstest]
fn trimming_respects_max_connections(mut params: HnswParams) {
    params = HnswParams::new(1, 1, 2);
    let source = LineSource::new(vec![0.0, 0.2, 0.25]);
    let mut graph = Graph::new(params.clone(), 3);

    graph
        .insert_first(NodeContext { node: 0, level: 0 })
        .expect("first insertion must succeed");
    graph
        .insert_node(NodeContext { node: 1, level: 0 }, &params, &source)
        .expect("second insertion must succeed");
    graph
        .insert_node(NodeContext { node: 2, level: 0 }, &params, &source)
        .expect("third insertion must succeed");

    let node = graph.node(1).expect("node must exist");
    assert!(node.neighbours(0).len() <= params.max_connections());
}

#[rstest]
fn search_returns_empty_for_empty_index(params: HnswParams, source: LineSource) {
    let index = CpuHnsw::new(params, source.values.len(), 13);
    let results = index.search(0, 3, &source).expect("search must succeed");
    assert!(results.is_empty());
}

#[rstest]
fn validate_distance_rejects_non_finite(_source: LineSource) {
    struct BadSource;
    impl DataSource for BadSource {
        fn distance(&self, _: usize, _: usize) -> Result<f32, DataSourceError> {
            Ok(f32::NAN)
        }
    }
    let result = search::validate_distance(&BadSource, 0, 0);
    assert!(matches!(
        result,
        Err(HnswError::InvalidParameters { reason }) if reason.contains("non-finite")
    ));
}
