//! Common testing utilities shared across workspace crates.
//!
//! The helpers here allow integration and unit tests to share assertion logic
//! without duplicating small but noisy adapters.

use tei_core::DocumentTitleError;

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
/// Panics when the provided result contains a [`DocumentTitleError`]. Tests
/// call this helper when successful serialization is mandatory.
#[must_use]
pub fn expect_markup(result: Result<String, DocumentTitleError>) -> String {
    match result {
        Ok(value) => value,
        Err(error) => panic!("expected valid title: {error}"),
    }
}
