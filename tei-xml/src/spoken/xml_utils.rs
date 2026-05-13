//! XML utility helpers for spoken-text extraction.

use quick_xml::events::{BytesRef, BytesStart};
use tei_core::TeiError;

use crate::attributes::extract_normalized_attribute;

use super::element_names::{BODY, TEXT};

/// Builds a stable XPath-like locator for an element.
#[must_use]
pub(crate) fn make_locator(parent_locator: Option<&str>, name: &str, index: usize) -> String {
    match (parent_locator, name) {
        (None, _) => format!("/{name}"),
        (Some(root_locator @ "/TEI"), TEXT) | (Some(root_locator @ "/TEI/text"), BODY) => {
            format!("{root_locator}/{name}")
        }
        (Some(parent_path), _) => format!("{parent_path}/{name}[{index}]"),
    }
}

/// Converts an XML local name into a validated UTF-8 string.
pub(crate) fn local_name(name: &[u8]) -> Result<String, TeiError> {
    String::from_utf8(name.to_vec())
        .map_err(|error| TeiError::xml(format!("invalid UTF-8 in element name: {error}")))
}

/// Extracts the `xml:id` attribute from an element.
pub(crate) fn extract_xml_id(element: &BytesStart<'_>) -> Result<Option<String>, TeiError> {
    extract_attribute(element, b"xml:id")
}

/// Extracts an attribute value from an element by raw attribute name.
pub(crate) fn extract_attribute(
    element: &BytesStart<'_>,
    name: &[u8],
) -> Result<Option<String>, TeiError> {
    extract_normalized_attribute(element, name)
}

/// Resolves an XML entity reference to its literal text.
pub(crate) fn resolve_entity_ref(reference: &BytesRef<'_>) -> Result<String, TeiError> {
    let name = reference
        .decode()
        .map_err(|error| TeiError::xml(error.to_string()))?;

    match name.as_ref() {
        "lt" => Ok("<".to_owned()),
        "gt" => Ok(">".to_owned()),
        "amp" => Ok("&".to_owned()),
        "quot" => Ok("\"".to_owned()),
        "apos" => Ok("'".to_owned()),
        _ => match reference.resolve_char_ref() {
            Ok(Some(ch)) => Ok(ch.to_string()),
            Ok(None) => Err(TeiError::xml(format!(
                "unrecognized entity reference: &{name};"
            ))),
            Err(error) => Err(TeiError::xml(error.to_string())),
        },
    }
}

#[cfg(test)]
mod tests {
    use quick_xml::events::BytesStart;

    use super::{extract_attribute, extract_xml_id};

    #[test]
    fn extract_attribute_returns_value_when_present() {
        let mut el = BytesStart::new("foo");
        el.push_attribute(("lang", "en"));
        assert_eq!(
            extract_attribute(&el, b"lang").expect("extraction must succeed"),
            Some("en".to_owned())
        );
    }

    #[test]
    fn extract_attribute_returns_none_when_absent() {
        let el = BytesStart::new("foo");
        assert_eq!(
            extract_attribute(&el, b"missing").expect("extraction must succeed"),
            None
        );
    }

    #[test]
    fn extract_attribute_propagates_unknown_entity_error() {
        let el = BytesStart::from_content(r#"foo lang="&badentity;""#, 3);
        let err =
            extract_attribute(&el, b"lang").expect_err("unknown entity must produce an error");
        assert!(
            err.to_string().contains("badentity"),
            "error message must mention the unknown entity; got: {err}"
        );
    }

    #[test]
    fn extract_xml_id_returns_value_when_present() {
        let mut el = BytesStart::new("u");
        el.push_attribute(("xml:id", "u42"));
        assert_eq!(
            extract_xml_id(&el).expect("extraction must succeed"),
            Some("u42".to_owned())
        );
    }

    #[test]
    fn extract_xml_id_returns_none_when_absent() {
        let el = BytesStart::new("u");
        assert_eq!(extract_xml_id(&el).expect("extraction must succeed"), None);
    }
}
