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

#[cfg(test)]
pub mod test_utils {
    //! Shared test utilities for strategy validation.

    use proptest::prelude::*;
    use proptest::strategy::ValueTree;
    use proptest::test_runner::TestRunner;

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
                .unwrap_or_else(|error| panic!("strategy should generate values: {error}"))
                .current();
            assert!(validator(&value), "invalid value: {value:?}");
        }
    }
}
