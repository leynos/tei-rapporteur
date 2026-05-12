//! Shared XML attribute extraction helpers for TEI parser components.
//!
//! Both the spoken-text extractor and the streaming parser need to read
//! `quick-xml` attributes with the same entity handling and whitespace
//! normalization. This module centralizes that behaviour so the parser paths do
//! not drift when `quick-xml` changes its attribute APIs. It also provides an
//! element-local cache for callers that need several attributes from the same
//! start tag, avoiding repeated scans in hot streaming parser paths.
//!
//! These helpers use [`XmlVersion::Implicit1_0`] because the current parser
//! states do not retain an XML declaration alongside `BytesStart` events. XML
//! 1.0 is the XML specification's default when the declaration is absent, and
//! future XML-version-aware parsing should change this module before changing
//! individual parser call sites.

use quick_xml::{XmlVersion, events::BytesStart};
use tei_core::TeiError;

/// Normalized attributes collected from a single XML start element.
#[derive(Debug, Eq, PartialEq)]
pub(crate) struct NormalizedAttributes {
    values: Vec<(Vec<u8>, String)>,
}

impl NormalizedAttributes {
    /// Collects all normalized attributes from `element` in one pass.
    pub(crate) fn from_element(element: &BytesStart<'_>) -> Result<Self, TeiError> {
        let mut values = Vec::new();

        for attr_result in element.attributes() {
            let attr = attr_result.map_err(|error| TeiError::xml(error.to_string()))?;
            let value = attr
                .normalized_value(XmlVersion::Implicit1_0)
                .map_err(|error| TeiError::xml(error.to_string()))?;
            values.push((attr.key.as_ref().to_vec(), value.into_owned()));
        }

        Ok(Self { values })
    }

    /// Returns a cloned attribute value by raw attribute name.
    pub(crate) fn get(&self, name: &[u8]) -> Option<String> {
        self.values
            .iter()
            .find_map(|(key, value)| (key.as_slice() == name).then(|| value.clone()))
    }

    /// Returns a required cloned attribute value by raw attribute name.
    pub(crate) fn required(&self, name: &[u8], message: &str) -> Result<String, TeiError> {
        self.get(name).ok_or_else(|| TeiError::xml(message))
    }
}

/// Extracts a normalized attribute value from an element by raw attribute name.
///
/// These parser paths do not currently retain an XML declaration from the
/// reader. XML 1.0 is therefore the correct default because the XML
/// specification treats a missing declaration as version 1.0.
pub(crate) fn extract_normalized_attribute(
    element: &BytesStart<'_>,
    name: &[u8],
) -> Result<Option<String>, TeiError> {
    NormalizedAttributes::from_element(element).map(|attributes| attributes.get(name))
}

#[cfg(test)]
mod tests {
    use quick_xml::events::BytesStart;

    use super::{NormalizedAttributes, extract_normalized_attribute};

    fn element(content: &str) -> BytesStart<'_> {
        BytesStart::from_content(content, 3)
    }

    #[test]
    fn returns_none_for_missing_attribute() {
        let element = element(r#"tag present="value""#);

        let result = extract_normalized_attribute(&element, b"missing");

        assert_eq!(result.unwrap_or_else(|error| panic!("{error}")), None);
    }

    #[test]
    fn normalizes_predefined_xml_entities_in_attribute_values() {
        let element =
            element(r#"tag value="&quot;quoted&quot; &apos;single&apos; &lt;tag&gt; &amp; text""#);

        let result = extract_normalized_attribute(&element, b"value");

        assert_eq!(
            result.unwrap_or_else(|error| panic!("{error}")),
            Some("\"quoted\" 'single' <tag> & text".to_owned())
        );
    }

    #[test]
    fn normalizes_xml_1_0_attribute_whitespace() {
        let element = element("tag value='alpha\tbeta\r\ngamma\nomega'");

        let result = extract_normalized_attribute(&element, b"value");

        assert_eq!(
            result.unwrap_or_else(|error| panic!("{error}")),
            Some("alpha beta gamma omega".to_owned())
        );
    }

    #[test]
    fn reports_attribute_iteration_errors() {
        let element = element(r"tag key='value' key='duplicate'");

        let error = extract_normalized_attribute(&element, b"key")
            .err()
            .unwrap_or_else(|| panic!("duplicate attribute should fail"));

        assert!(error.to_string().contains("duplicated attribute"));
    }

    #[test]
    fn reports_normalization_errors() {
        let element = element(r#"tag value="&unknown;""#);

        let error = extract_normalized_attribute(&element, b"value")
            .err()
            .unwrap_or_else(|| panic!("unknown entity should fail"));

        assert!(error.to_string().contains("unknown"));
    }

    #[test]
    fn caches_attributes_for_repeated_lookup() {
        let element = element(r#"tag first="one" second="two""#);

        let attributes =
            NormalizedAttributes::from_element(&element).unwrap_or_else(|error| panic!("{error}"));

        assert_eq!(attributes.get(b"first"), Some("one".to_owned()));
        assert_eq!(attributes.get(b"second"), Some("two".to_owned()));
        assert_eq!(attributes.get(b"missing"), None);
    }
}
