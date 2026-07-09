//! Proptest strategies for generating valid TEI documents.
//!
//! This module provides strategies that generate `TeiDocument` instances
//! respecting all validation constraints (non-empty titles, valid identifiers,
//! non-blank text segments). The generated documents can be used for
//! property-based round-trip testing between XML, JSON, and `MessagePack`.

pub mod document;
pub mod header;
pub mod inline;
pub mod primitives;
pub mod text;

/// Extension trait that unwraps fallible constructions inside proptest
/// strategy pipelines.
///
/// Strategy composition closures cannot propagate `Result`, so a malformed
/// regex or an invalid generated value aborts the test with a descriptive
/// panic instead.
pub trait ExpectValid {
    /// The success value produced on unwrap.
    type Value;

    /// Unwraps the value, panicking with `context` on failure.
    fn expect_valid(self, context: &str) -> Self::Value;
}

impl<T, E: std::fmt::Display> ExpectValid for Result<T, E> {
    type Value = T;

    fn expect_valid(self, context: &str) -> T {
        match self {
            Ok(value) => value,
            Err(error) => panic!("{context}: {error}"),
        }
    }
}

#[cfg(test)]
pub mod test_utils {
    //! Shared test utilities for strategy validation.

    use proptest::prelude::*;
    use proptest::strategy::ValueTree;
    use proptest::test_runner::TestRunner;

    use super::ExpectValid;

    /// Generates values from a strategy and validates each with the given predicate.
    ///
    /// Generates 20 values from the strategy and asserts that each satisfies
    /// the validator predicate.
    pub fn assert_strategy_produces_valid_values<S, F>(strategy: S, validator: F)
    where
        S: Strategy,
        S::Value: std::fmt::Debug,
        F: Fn(&S::Value) -> bool,
    {
        let mut runner = TestRunner::default();
        for _ in 0..20 {
            let value = strategy
                .new_tree(&mut runner)
                .expect_valid("strategy should generate values")
                .current();
            assert!(validator(&value), "invalid value: {value:?}");
        }
    }
}
