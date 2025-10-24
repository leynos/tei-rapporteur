//! Data access traits used by the HNSW graph.

use std::fmt;

use thiserror::Error;

/// Errors surfaced by a [`DataSource`].
#[derive(Debug, Error, PartialEq)]
pub enum DataSourceError {
    /// The caller referenced a vector index outside the available range.
    #[error("index {index} is out of bounds for data source")]
    OutOfBounds {
        /// The invalid index requested by the caller.
        index: usize,
    },
    /// The distance function reported an application-defined failure.
    #[error("distance computation failed: {message}")]
    Operation {
        /// Descriptive reason explaining the failure.
        message: String,
    },
}

impl DataSourceError {
    /// Creates a new [`DataSourceError::Operation`] from an arbitrary message.
    #[must_use]
    pub fn operation(message: impl Into<String>) -> Self {
        Self::Operation {
            message: message.into(),
        }
    }
}

/// Provides vector distances for the HNSW index.
pub trait DataSource {
    /// Computes the metric distance between `query` and `candidate`.
    ///
    /// # Errors
    ///
    /// Implementations may return [`DataSourceError`] when either index lies
    /// outside the available range or when the distance function fails.
    fn distance(&self, query: usize, candidate: usize) -> Result<f32, DataSourceError>;

    /// Computes distances from `query` to all `candidates` in a single pass.
    ///
    /// # Errors
    ///
    /// Returns [`DataSourceError`] when any candidate index is invalid or the
    /// data source encounters an error computing a distance.
    fn batch_distances(
        &self,
        query: usize,
        candidates: &[usize],
    ) -> Result<Vec<f32>, DataSourceError> {
        candidates
            .iter()
            .copied()
            .map(|candidate| self.distance(query, candidate))
            .collect()
    }
}

impl fmt::Debug for dyn DataSource + Send + Sync {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("DataSource")
    }
}

#[cfg(test)]
mod tests {
    use super::{DataSource, DataSourceError};
    use rstest::rstest;

    #[derive(Debug, Default)]
    struct IdentitySource;

    impl DataSource for IdentitySource {
        fn distance(&self, query: usize, candidate: usize) -> Result<f32, DataSourceError> {
            if candidate >= 8 {
                return Err(DataSourceError::OutOfBounds { index: candidate });
            }
            let query_i16 = i16::try_from(query)
                .map_err(|_| DataSourceError::operation("query index exceeds i16 range"))?;
            let candidate_i16 = i16::try_from(candidate)
                .map_err(|_| DataSourceError::operation("candidate index exceeds i16 range"))?;
            Ok(f32::from((query_i16 - candidate_i16).abs()))
        }
    }

    #[rstest]
    #[case(0, &[1, 2, 3], &[1.0, 2.0, 3.0])]
    #[case(3, &[0, 6], &[3.0, 3.0])]
    fn batch_distances_matches_pointwise(
        #[case] query: usize,
        #[case] candidates: &[usize],
        #[case] expected: &[f32],
    ) {
        let source = IdentitySource;
        let distances = match source.batch_distances(query, candidates) {
            Ok(distances) => distances,
            Err(err) => panic!("batch_distances failed: {err}"),
        };
        assert_eq!(distances, expected);
    }

    #[test]
    fn batch_distances_returns_first_error() {
        let source = IdentitySource;
        let result = source.batch_distances(0, &[1, 8, 2]);
        assert_eq!(result, Err(DataSourceError::OutOfBounds { index: 8 }));
    }
}
