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

/// Ensures behaviour-driven fixtures initialise successfully and returns them.
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
/// let state = expect_validated_state(Ok(42), "demo");
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
            panic!("{context} scenarios must initialise their state successfully: {error}")
        }
    }
}
