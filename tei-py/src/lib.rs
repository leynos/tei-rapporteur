//! Python-facing helpers.
//!
//! The crate will eventually host the `PyO3` bindings. For now it exercises the
//! workspace plumbing by re-exporting a simple serialization helper.

use tei_core::TeiError;
use tei_xml::serialize_document_title;

/// Validates and emits TEI markup suitable for exposure through `PyO3`.
///
/// # Errors
///
/// Returns [`tei_core::TeiError::DocumentTitle`] when the provided title is
/// blank after trimming. The helper exists so `PyO3` glue can focus on Python
/// ergonomics whilst reusing the Rust validation logic.
///
/// # Examples
///
/// ```
/// use tei_py::emit_title_markup;
///
/// let markup = emit_title_markup("Welcome to Night Vale")?;
/// assert_eq!(markup, "<title>Welcome to Night Vale</title>");
/// # Ok::<(), tei_core::TeiError>(())
/// ```
pub fn emit_title_markup(raw_title: &str) -> Result<String, TeiError> {
    serialize_document_title(raw_title)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;
    use tei_core::DocumentTitleError;
    use tei_test_helpers::expect_markup;

    #[rstest]
    #[case("Archive 81", "<title>Archive 81</title>")]
    fn delegates_to_xml_serializer(#[case] input: &str, #[case] expected: &str) {
        let markup = expect_markup(emit_title_markup(input));
        assert_eq!(markup, expected);
    }

    #[test]
    fn propagates_empty_title_error() {
        let result = emit_title_markup("   ");
        let Err(err) = result else {
            panic!("expected TeiError::DocumentTitle for blank titles");
        };
        let TeiError::DocumentTitle(inner) = err else {
            panic!("expected document title error variant");
        };
        assert_eq!(inner, DocumentTitleError::Empty);
    }
}
