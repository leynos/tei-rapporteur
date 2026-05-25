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
use tracing::debug;

/// Normalized attributes collected from a single XML start element.
#[derive(Debug, Eq, PartialEq)]
pub(crate) struct NormalizedAttributes {
    values: Vec<(Vec<u8>, String)>,
}

impl NormalizedAttributes {
    /// Collects all normalized attributes from `element` in one pass.
    pub(crate) fn from_element(element: &BytesStart<'_>) -> Result<Self, TeiError> {
        let mut values = Vec::new();
        let element_name = element_name(element);

        for attr_result in element.attributes() {
            let attr = attr_result.map_err(|error| {
                debug!(
                    element = %element_name,
                    error = %error,
                    "xml_attribute_iteration_failed"
                );
                TeiError::xml(error.to_string())
            })?;
            let attribute_name = attribute_name(attr.key.as_ref());
            let value = attr
                .normalized_value(XmlVersion::Implicit1_0)
                .map_err(|error| {
                    debug!(
                        element = %element_name,
                        attribute = %attribute_name,
                        error = %error,
                        "xml_attribute_normalization_failed"
                    );
                    TeiError::xml(error.to_string())
                })?;
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
}

fn element_name(element: &BytesStart<'_>) -> String {
    let local_name = element.local_name();
    String::from_utf8_lossy(local_name.as_ref()).into_owned()
}

fn attribute_name(name: &[u8]) -> String {
    String::from_utf8_lossy(name).into_owned()
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
    use proptest::prelude::*;
    use quick_xml::events::BytesStart;
    use rstest::{fixture, rstest};

    use super::{NormalizedAttributes, extract_normalized_attribute};

    fn element(content: &str) -> BytesStart<'_> {
        BytesStart::from_content(content, 3)
    }

    #[fixture]
    fn element_fixture() -> for<'a> fn(&'a str) -> BytesStart<'a> {
        element
    }

    #[rstest]
    #[case(r#"tag present="value""#, b"missing", None)]
    #[case(
        r#"tag value="&quot;quoted&quot; &apos;single&apos; &lt;tag&gt; &amp; text""#,
        b"value",
        Some("\"quoted\" 'single' <tag> & text")
    )]
    #[case(
        "tag value='alpha\tbeta\r\ngamma\nomega'",
        b"value",
        Some("alpha beta gamma omega")
    )]
    fn extracts_normalized_attribute_values(
        #[case] content: &str,
        #[case] name: &[u8],
        #[case] expected: Option<&str>,
        element_fixture: for<'a> fn(&'a str) -> BytesStart<'a>,
    ) {
        let element = element_fixture(content);
        let result = extract_normalized_attribute(&element, name);

        assert_eq!(
            result.unwrap_or_else(|error| panic!("{error}")),
            expected.map(str::to_owned)
        );
    }

    #[rstest]
    #[case(r"tag key='value' key='duplicate'", b"key", "duplicated attribute")]
    #[case(r#"tag value="&unknown;""#, b"value", "unknown")]
    fn reports_normalized_attribute_errors(
        #[case] content: &str,
        #[case] name: &[u8],
        #[case] expected_error: &str,
        element_fixture: for<'a> fn(&'a str) -> BytesStart<'a>,
    ) {
        let element = element_fixture(content);
        let error = extract_normalized_attribute(&element, name)
            .err()
            .unwrap_or_else(|| panic!("attribute extraction should fail"));

        assert!(error.to_string().contains(expected_error));
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

    #[derive(Clone, Debug)]
    enum AttributeFragment {
        Text(String),
        Whitespace(char),
        Entity(&'static str),
    }

    impl AttributeFragment {
        fn raw(&self) -> String {
            match self {
                Self::Text(value) => value.clone(),
                Self::Whitespace(ch) => ch.to_string(),
                Self::Entity(name) => format!("&{name};"),
            }
        }
    }

    fn normalized_value(fragments: &[AttributeFragment]) -> String {
        let raw_value = fragments
            .iter()
            .map(AttributeFragment::raw)
            .collect::<String>();
        normalized_raw_attribute_value(&raw_value)
    }

    fn normalized_raw_attribute_value(raw_value: &str) -> String {
        let mut normalized = String::new();
        let mut chars = raw_value.chars().peekable();

        while let Some(ch) = chars.next() {
            match ch {
                '\r' => {
                    let _ = chars.next_if_eq(&'\n');
                    normalized.push(' ');
                }
                '\n' | '\t' => normalized.push(' '),
                '&' => {
                    let entity = chars
                        .by_ref()
                        .take_while(|next| *next != ';')
                        .collect::<String>();
                    normalized.push(match entity.as_str() {
                        "quot" => '"',
                        "apos" => '\'',
                        "lt" => '<',
                        "gt" => '>',
                        _ => '&',
                    });
                }
                _ => normalized.push(ch),
            }
        }

        normalized
    }

    fn escaped_attribute_value(value: &str) -> String {
        value
            .replace('&', "&amp;")
            .replace('"', "&quot;")
            .replace('\'', "&apos;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
    }

    fn attribute_fragment_strategy() -> impl Strategy<Value = AttributeFragment> {
        prop_oneof![
            "[A-Za-z0-9,.;:!? -]{0,16}".prop_map(AttributeFragment::Text),
            prop::sample::select(vec!['\t', '\n', '\r']).prop_map(AttributeFragment::Whitespace),
            prop::sample::select(vec!["quot", "apos", "lt", "gt", "amp"])
                .prop_map(AttributeFragment::Entity),
        ]
    }

    proptest! {
        #[test]
        fn normalizes_xml_1_0_attribute_values_for_arbitrary_fragments(
            fragments in prop::collection::vec(attribute_fragment_strategy(), 0..24)
        ) {
            let raw_value = fragments.iter().map(AttributeFragment::raw).collect::<String>();
            let expected = normalized_value(&fragments);
            let element = BytesStart::from_content(
                format!(r#"tag value="{raw_value}""#),
                3,
            );

            let actual = extract_normalized_attribute(&element, b"value")
                .expect("attribute extraction must succeed");

            prop_assert_eq!(actual, Some(expected));
        }

        #[test]
        fn normalized_attribute_values_are_idempotent(
            fragments in prop::collection::vec(attribute_fragment_strategy(), 0..24)
        ) {
            let normalized_value = normalized_value(&fragments);
            let raw_value = escaped_attribute_value(&normalized_value);
            let element = BytesStart::from_content(
                format!(r#"tag value="{raw_value}""#),
                3,
            );

            let actual = extract_normalized_attribute(&element, b"value")
                .expect("attribute extraction must succeed");

            prop_assert_eq!(actual, Some(normalized_value));
        }

        #[test]
        fn normalized_attributes_cache_returns_stable_values(
            attributes in prop::collection::btree_map(
                "[A-Za-z][A-Za-z0-9_-]{0,24}",
                prop::collection::vec(attribute_fragment_strategy(), 0..12),
                0..12,
            )
        ) {
            let content = attributes
                .iter()
                .map(|(name, fragments)| {
                    let raw_value = fragments.iter().map(AttributeFragment::raw).collect::<String>();
                    format!(r#"{name}="{raw_value}""#)
                })
                .collect::<Vec<_>>()
                .join(" ");
            let element = BytesStart::from_content(format!("tag {content}"), 3);
            let normalized_attributes = NormalizedAttributes::from_element(&element)
                .expect("attribute collection must succeed");

            for (name, fragments) in attributes {
                let expected = normalized_value(&fragments);

                prop_assert_eq!(normalized_attributes.get(name.as_bytes()), Some(expected.clone()));
                prop_assert_eq!(normalized_attributes.get(name.as_bytes()), Some(expected.clone()));
                prop_assert_eq!(normalized_attributes.get(name.as_bytes()), Some(expected));
            }

            prop_assert_eq!(normalized_attributes.get(b"missing"), None);
            prop_assert_eq!(normalized_attributes.get(b"missing"), None);
        }
    }
}
