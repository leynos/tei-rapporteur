//! XML helpers for TEI-Rapporteur.
//!
//! The module currently focuses on a title serialization shim that exercises the
//! crate graph created during workspace scaffolding.

use quick_xml::de;
use tei_core::{TeiDocument, TeiError};

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
    if !input
        .chars()
        .any(|character| matches!(character, '&' | '<' | '>' | '"' | '\''))
    {
        return input.to_owned();
    }

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

/// Serializes the document title into a minimal TEI snippet.
///
/// # Examples
///
/// ```
/// use tei_core::TeiDocument;
/// use tei_xml::serialize_title;
///
/// let document = TeiDocument::from_title_str("Wolf 359")?;
/// assert_eq!(serialize_title(&document), "<title>Wolf 359</title>");
/// # Ok::<(), tei_core::TeiError>(())
/// ```
#[must_use]
pub fn serialize_title(document: &TeiDocument) -> String {
    format!(
        "<title>{}</title>",
        escape_xml_text(document.title().as_str())
    )
}

/// Validates a raw title and returns the serialized markup.
///
/// # Errors
///
/// Returns [`tei_core::TeiError::DocumentTitle`] when the title trims to an
/// empty string.
///
/// # Examples
///
/// ```
/// use tei_xml::serialize_document_title;
///
/// let markup = serialize_document_title("Alice Isn't Dead")?;
/// assert_eq!(markup, "<title>Alice Isn't Dead</title>");
/// # Ok::<(), tei_core::TeiError>(())
/// ```
///
/// ```
/// use tei_xml::serialize_document_title;
///
/// let markup = serialize_document_title("R&D <Test>")?;
/// assert_eq!(markup, "<title>R&amp;D &lt;Test&gt;</title>");
/// # Ok::<(), tei_core::TeiError>(())
/// ```
pub fn serialize_document_title(raw_title: &str) -> Result<String, TeiError> {
    TeiDocument::from_title_str(raw_title).map(|document| serialize_title(&document))
}

/// Parses a TEI XML string into a [`TeiDocument`].
///
/// # Errors
///
/// Returns [`TeiError::Xml`] when the XML is not well-formed or does not match
/// the profiled TEI structure expected by the data model.
///
/// # Examples
///
/// ```
/// use tei_core::TeiError;
/// use tei_xml::parse_xml;
///
/// let xml = concat!(
///     "<TEI>",
///     "<teiHeader>",
///     "<fileDesc>",
///     "<title>Wolf 359</title>",
///     "</fileDesc>",
///     "</teiHeader>",
///     "<text>",
///     "<body/>",
///     "</text>",
///     "</TEI>",
/// );
/// let document = parse_xml(xml)?;
/// assert_eq!(document.title().as_str(), "Wolf 359");
/// # Ok::<(), TeiError>(())
/// ```
pub fn parse_xml(xml: &str) -> Result<TeiDocument, TeiError> {
    de::from_str(xml).map_err(|error| TeiError::xml(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;
    use tei_core::DocumentTitleError;
    use tei_test_helpers::expect_markup;

    const MINIMAL_TEI: &str = concat!(
        "<TEI>",
        "<teiHeader>",
        "<fileDesc>",
        "<title>Wolf 359</title>",
        "</fileDesc>",
        "</teiHeader>",
        "<text>",
        "<body/>",
        "</text>",
        "</TEI>",
    );

    const MISSING_HEADER_TEI: &str = concat!("<TEI>", "<text>", "<body/>", "</text>", "</TEI>",);
    const BLANK_TITLE_TEI: &str = concat!(
        "<TEI>",
        "<teiHeader>",
        "<fileDesc>",
        "<title>   </title>",
        "</fileDesc>",
        "</teiHeader>",
        "<text>",
        "<body/>",
        "</text>",
        "</TEI>",
    );

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

    fn expect_title_error(result: Result<String, TeiError>) -> DocumentTitleError {
        match result {
            Ok(value) => panic!("expected invalid title, got {value}",),
            Err(TeiError::DocumentTitle(error)) => error,
            Err(other) => panic!("expected document title error, received {other}"),
        }
    }

    #[rstest]
    #[case("Limetown", "<title>Limetown</title>")]
    #[case("  Wooden Overcoats  ", "<title>Wooden Overcoats</title>")]
    #[case("R&D <Test>", "<title>R&amp;D &lt;Test&gt;</title>")]
    fn serializes_titles(#[case] input: &str, #[case] expected: &str) {
        let markup = expect_markup(serialize_document_title(input));
        assert_eq!(markup, expected);
    }

    #[rstest]
    #[case("")]
    #[case("   ")]
    fn rejects_empty_titles(#[case] input: &str) {
        let error = expect_title_error(serialize_document_title(input));
        assert_eq!(error, DocumentTitleError::Empty);
    }

    #[test]
    fn parses_minimal_document() {
        let document = parse_xml(MINIMAL_TEI).expect("valid TEI should parse");
        let expected =
            TeiDocument::from_title_str("Wolf 359").expect("valid title should build document");

        assert_eq!(document, expected);
    }

    #[test]
    fn surfaces_quick_xml_errors() {
        let Err(error) = parse_xml(MISSING_HEADER_TEI) else {
            panic!("expected parsing to fail");
        };

        match error {
            TeiError::Xml { message } => assert!(
                message.contains("teiHeader"),
                "missing header error should mention field, found {message}"
            ),
            other => panic!("expected XML error, found {other}"),
        }
    }

    #[test]
    fn rejects_blank_titles_during_parse() {
        let Err(error) = parse_xml(BLANK_TITLE_TEI) else {
            panic!("blank titles must not parse successfully");
        };

        match error {
            TeiError::Xml { message } => assert!(
                message.contains("document title may not be empty"),
                "error should mention empty title, found {message}"
            ),
            other => panic!("expected XML error signalling empty title, found {other}"),
        }
    }
}
