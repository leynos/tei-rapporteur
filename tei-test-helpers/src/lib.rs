//! Common testing utilities shared across workspace crates.
//!
//! The helpers here allow integration and unit tests to share assertion logic
//! without duplicating small but noisy adapters.

use std::fmt::Display;
use tei_core::TeiError;

/// Extracts the serialized markup from a result or panics with context.
///
/// This helper keeps tests expressive by collapsing [`Result`] handling down to
/// a single call. When the serialization succeeds the markup is returned. When
/// it fails the panic message includes the error so failures are easier to
/// diagnose.
///
/// # Examples
///
/// ```
/// use tei_test_helpers::expect_markup;
///
/// let markup = expect_markup(Ok(String::from("<title>Example</title>")));
/// assert_eq!(markup, "<title>Example</title>");
/// ```
///
/// # Panics
///
/// Panics when the provided result contains a [`TeiError::DocumentTitle`]. Tests
/// call this helper when successful serialization is mandatory.
#[must_use]
pub fn expect_markup(result: Result<String, TeiError>) -> String {
    match result {
        Ok(value) => value,
        Err(TeiError::DocumentTitle(error)) => panic!("expected valid title: {error}"),
        Err(other) => panic!("expected document title success, received {other}"),
    }
}

/// Ensures behaviour-driven fixtures initialize successfully and returns them.
///
/// Tests rely on fixture constructors that build up shared state. When those
/// constructors fail the scenario cannot proceed, so this helper panics with a
/// consistent message that includes the failing context. It accepts any
/// [`Result`] whose error implements [`Display`], making it suitable for both
/// `anyhow::Result` and concrete error enums.
///
/// # Examples
///
/// ```
/// use tei_test_helpers::expect_validated_state;
///
/// let state = expect_validated_state(Ok::<_, std::fmt::Error>(42), "demo");
/// assert_eq!(state, 42);
/// ```
///
/// # Panics
///
/// Panics with a descriptive message when the provided result contains an
/// error. The panic message prefixes the supplied `context` so failing
/// scenarios remain easy to trace back to their feature files.
pub fn expect_validated_state<T, E>(result: Result<T, E>, context: &str) -> T
where
    E: Display,
{
    match result {
        Ok(value) => value,
        Err(error) => {
            panic!("{context} scenarios must initialize their state successfully: {error}")
        }
    }
}

/// Collapses a [`Result`] into its value at a documented panic boundary.
///
/// Some testing contexts cannot propagate an error. `proptest` strategy
/// pipelines built with `prop_compose!` are the motivating case: the closure
/// bodies must yield a value, so a fallible constructor has nowhere to send its
/// error. Rather than scatter `expect` calls through those pipelines, route
/// them through this one named boundary so that the panic is deliberate,
/// consistently worded, and easy to find.
///
/// Prefer returning `Result` wherever propagation is possible; reach for this
/// trait only where the surrounding API forbids it.
///
/// # Examples
///
/// ```
/// use tei_test_helpers::ExpectValid;
///
/// let value = Ok::<_, std::fmt::Error>(7).expect_valid("generated count");
/// assert_eq!(value, 7);
/// ```
pub trait ExpectValid<T> {
    /// Returns the success value, panicking with `context` on failure.
    ///
    /// # Panics
    ///
    /// Panics when `self` holds an error. The message names `context` so the
    /// failing strategy or constructor is identifiable from the panic alone.
    fn expect_valid(self, context: &str) -> T;
}

impl<T, E> ExpectValid<T> for Result<T, E>
where
    E: Display,
{
    fn expect_valid(self, context: &str) -> T {
        match self {
            Ok(value) => value,
            Err(error) => panic!("{context} should be valid: {error}"),
        }
    }
}
