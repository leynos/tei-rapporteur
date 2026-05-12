//! Shared XML attribute extraction helpers.

use quick_xml::{XmlVersion, events::BytesStart};
use tei_core::TeiError;

/// Extracts a normalized attribute value from an element by raw attribute name.
///
/// These parser paths do not currently retain an XML declaration from the
/// reader. XML 1.0 is therefore the correct default because the XML
/// specification treats a missing declaration as version 1.0.
pub(crate) fn extract_normalized_attribute(
    element: &BytesStart<'_>,
    name: &[u8],
) -> Result<Option<String>, TeiError> {
    for attr_result in element.attributes() {
        let attr = attr_result.map_err(|error| TeiError::xml(error.to_string()))?;
        if attr.key.as_ref() == name {
            let value = attr
                .normalized_value(XmlVersion::Implicit1_0)
                .map_err(|error| TeiError::xml(error.to_string()))?;
            return Ok(Some(value.into_owned()));
        }
    }
    Ok(None)
}
