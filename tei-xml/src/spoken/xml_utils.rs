//! XML utility helpers for spoken-text extraction.

use quick_xml::events::{BytesRef, BytesStart};
use tei_core::TeiError;

/// Builds a stable XPath-like locator for an element.
#[must_use]
pub(crate) fn make_locator(parent_locator: Option<&str>, name: &str, index: usize) -> String {
    match (parent_locator, name) {
        (None, _) => format!("/{name}"),
        (Some(root_locator @ "/TEI"), "text") | (Some(root_locator @ "/TEI/text"), "body") => {
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
    for attr_result in element.attributes() {
        let attr = attr_result.map_err(|error| TeiError::xml(error.to_string()))?;
        if attr.key.as_ref() == name {
            let value = attr
                .unescape_value()
                .map_err(|error| TeiError::xml(error.to_string()))?;
            return Ok(Some(value.into_owned()));
        }
    }
    Ok(None)
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
