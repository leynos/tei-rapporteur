//! XML helpers for TEI-Rapporteur.
//!
//! The module currently focuses on a title serialisation shim that exercises the
//! crate graph created during workspace scaffolding.

use tei_core::{DocumentTitleError, TeiDocument};

/// Encodes text for inclusion in XML content.
///
/// The helper escapes markup-significant characters to keep the resulting
/// document well-formed. It intentionally mirrors the narrow surface required
/// for text nodes, not attributes.
///
/// # Examples
///
/// ```
/// use tei_xml::escape_xml_text;
///
/// assert_eq!(escape_xml_text("R&D <Test>"), "R&amp;D &lt;Test&gt;");
/// ```
#[must_use]
pub fn escape_xml_text(input: &str) -> String {
    let mut escaped = String::with_capacity(input.len());

    for character in input.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&apos;"),
            other => escaped.push(other),
        }
    }

    escaped
}

/// Serialises the document title into a minimal TEI snippet.
///
/// # Examples
///
/// ```
/// use tei_core::TeiDocument;
/// use tei_xml::serialise_title;
///
/// let document = TeiDocument::from_title_str("Wolf 359")?;
/// assert_eq!(serialise_title(&document), "<title>Wolf 359</title>");
/// # Ok::<(), tei_core::DocumentTitleError>(())
/// ```
#[must_use]
pub fn serialise_title(document: &TeiDocument) -> String {
    format!(
        "<title>{}</title>",
        escape_xml_text(document.title().as_str())
    )
}

/// Validates a raw title and returns the serialised markup.
///
/// # Errors
///
/// Returns [`tei_core::DocumentTitleError::Empty`] when the title trims to an
/// empty string.
///
/// # Examples
///
/// ```
/// use tei_xml::serialise_document_title;
///
/// let markup = serialise_document_title("Alice Isn't Dead")?;
/// assert_eq!(markup, "<title>Alice Isn't Dead</title>");
/// # Ok::<(), tei_core::DocumentTitleError>(())
/// ```
///
/// ```
/// use tei_xml::serialise_document_title;
///
/// let markup = serialise_document_title("R&D <Test>")?;
/// assert_eq!(markup, "<title>R&amp;D &lt;Test&gt;</title>");
/// # Ok::<(), tei_core::DocumentTitleError>(())
/// ```
pub fn serialise_document_title(raw_title: &str) -> Result<String, DocumentTitleError> {
    TeiDocument::from_title_str(raw_title).map(|document| serialise_title(&document))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case("Plain", "Plain")]
    #[case("Fish & Chips", "Fish &amp; Chips")]
    #[case("5 < 7", "5 &lt; 7")]
    #[case("7 > 5", "7 &gt; 5")]
    #[case("\"Quote\"", "&quot;Quote&quot;")]
    #[case("'Single'", "&apos;Single&apos;")]
    fn escapes_xml_text(#[case] input: &str, #[case] expected: &str) {
        assert_eq!(escape_xml_text(input), expected);
    }

    fn expect_markup(result: Result<String, DocumentTitleError>) -> String {
        match result {
            Ok(value) => value,
            Err(error) => panic!("expected valid title: {error}"),
        }
    }

    fn expect_title_error(result: Result<String, DocumentTitleError>) -> DocumentTitleError {
        match result {
            Ok(value) => panic!("expected invalid title, got {value}",),
            Err(error) => error,
        }
    }

    #[rstest]
    #[case("Limetown", "<title>Limetown</title>")]
    #[case("  Wooden Overcoats  ", "<title>Wooden Overcoats</title>")]
    #[case("R&D <Test>", "<title>R&amp;D &lt;Test&gt;</title>")]
    fn serialises_titles(#[case] input: &str, #[case] expected: &str) {
        let markup = expect_markup(serialise_document_title(input));
        assert_eq!(markup, expected);
    }

    #[rstest]
    #[case("")]
    #[case("   ")]
    fn rejects_empty_titles(#[case] input: &str) {
        let error = expect_title_error(serialise_document_title(input));
        assert_eq!(error, DocumentTitleError::Empty);
    }
}
