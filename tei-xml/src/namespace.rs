//! TEI namespace handling utilities.
//!
//! The `quick-xml` serializer does not emit namespace declarations, so this
//! module provides helpers to add namespace attributes for external validators.

/// TEI namespace URI required for schema validation.
pub const TEI_NAMESPACE: &str = "http://www.tei-c.org/ns/1.0";

/// Adds the TEI namespace declaration to the root `<TEI>` element.
///
/// The `quick-xml` serializer does not emit namespace declarations, so this
/// helper rewrites `<TEI>` to `<TEI xmlns="...">` for external validators.
///
/// # Examples
///
/// ```
/// use tei_xml::add_tei_namespace;
///
/// let input = "<TEI><teiHeader/></TEI>";
/// let output = add_tei_namespace(input);
/// assert!(output.starts_with("<TEI xmlns=\"http://www.tei-c.org/ns/1.0\">"));
/// ```
#[must_use]
pub fn add_tei_namespace(xml: &str) -> String {
    xml.replacen("<TEI>", &format!("<TEI xmlns=\"{TEI_NAMESPACE}\">"), 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adds_namespace_to_tei_element() {
        let input = "<TEI><teiHeader/></TEI>";
        let output = add_tei_namespace(input);
        assert!(output.starts_with("<TEI xmlns=\"http://www.tei-c.org/ns/1.0\">"));
    }

    #[test]
    fn only_replaces_first_tei_element() {
        let input = "<TEI><TEI/></TEI>";
        let output = add_tei_namespace(input);
        assert_eq!(
            output,
            "<TEI xmlns=\"http://www.tei-c.org/ns/1.0\"><TEI/></TEI>"
        );
    }
}
