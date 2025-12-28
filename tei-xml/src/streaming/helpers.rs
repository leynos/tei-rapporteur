//! Helper functions for the streaming TEI parser.
//!
//! These functions handle XML element construction, attribute extraction,
//! and building of TEI domain objects from parsed content.

use quick_xml::events::{BytesEnd, BytesStart};

use tei_core::{Hi, Inline, P, Pause, TeiError, Utterance};

/// Extracts the `xml:id` attribute from an element.
pub fn extract_xml_id(element: &BytesStart<'_>) -> Result<Option<String>, TeiError> {
    extract_attribute(element, b"xml:id")
}

/// Extracts an attribute value from an element by name.
pub fn extract_attribute(
    element: &BytesStart<'_>,
    name: &[u8],
) -> Result<Option<String>, TeiError> {
    for attr_result in element.attributes() {
        let attr = attr_result.map_err(|e| TeiError::xml(e.to_string()))?;
        if attr.key.as_ref() == name {
            let value = attr
                .unescape_value()
                .map_err(|e| TeiError::xml(e.to_string()))?;
            return Ok(Some(value.into_owned()));
        }
    }
    Ok(None)
}

/// Appends an element opening tag with attributes and a custom closing sequence.
fn append_element_with_attributes(
    buffer: &mut Vec<u8>,
    element: &BytesStart<'_>,
    closing: &[u8],
) -> Result<(), TeiError> {
    buffer.push(b'<');
    buffer.extend_from_slice(element.name().as_ref());
    for attr_result in element.attributes() {
        let attr = attr_result.map_err(|e| TeiError::xml(e.to_string()))?;
        buffer.push(b' ');
        buffer.extend_from_slice(attr.key.as_ref());
        buffer.extend_from_slice(b"=\"");
        buffer.extend_from_slice(&attr.value);
        buffer.push(b'"');
    }
    buffer.extend_from_slice(closing);
    Ok(())
}

/// Appends a start element tag to the buffer.
pub fn append_start_element(
    buffer: &mut Vec<u8>,
    element: &BytesStart<'_>,
) -> Result<(), TeiError> {
    append_element_with_attributes(buffer, element, b">")
}

/// Appends an end element tag to the buffer.
pub fn append_end_element(buffer: &mut Vec<u8>, element: &BytesEnd<'_>) {
    buffer.extend_from_slice(b"</");
    buffer.extend_from_slice(element.name().as_ref());
    buffer.push(b'>');
}

/// Appends an empty element tag to the buffer.
pub fn append_empty_element(
    buffer: &mut Vec<u8>,
    element: &BytesStart<'_>,
) -> Result<(), TeiError> {
    append_element_with_attributes(buffer, element, b"/>")
}

/// Builds a paragraph from an optional ID and inline content.
pub fn build_paragraph(id: Option<String>, content: Vec<Inline>) -> Result<P, TeiError> {
    let mut paragraph = if content.is_empty() {
        P::from_text_segments([""]).map_err(|e| TeiError::xml(e.to_string()))?
    } else {
        P::from_inline(content).map_err(|e| TeiError::xml(e.to_string()))?
    };
    if let Some(id_str) = id {
        paragraph
            .set_id(id_str)
            .map_err(|e| TeiError::xml(e.to_string()))?;
    }
    Ok(paragraph)
}

/// Builds an utterance from an optional ID, speaker, and inline content.
pub fn build_utterance(
    id: Option<String>,
    who: Option<&str>,
    content: Vec<Inline>,
) -> Result<Utterance, TeiError> {
    let mut utterance = if content.is_empty() {
        Utterance::from_text_segments(who, [""]).map_err(|e| TeiError::xml(e.to_string()))?
    } else {
        Utterance::from_inline(who, content).map_err(|e| TeiError::xml(e.to_string()))?
    };
    if let Some(id_str) = id {
        utterance
            .set_id(id_str)
            .map_err(|e| TeiError::xml(e.to_string()))?;
    }
    Ok(utterance)
}

/// Builds an emphasis (hi) element from an optional rendition and content.
pub fn build_hi(rend: Option<String>, content: Vec<Inline>) -> Hi {
    if content.is_empty() {
        // Empty hi element - use a single empty text node
        let hi = Hi::new([Inline::text("")]);
        return match rend {
            Some(r) => Hi::with_rend(r, hi.content().iter().cloned()),
            None => hi,
        };
    }

    match rend {
        Some(r) => Hi::with_rend(r, content),
        None => Hi::new(content),
    }
}

/// Builds a pause element from optional duration and type.
pub fn build_pause(dur: Option<String>, pause_type: Option<String>) -> Pause {
    let mut pause = Pause::new();
    if let Some(d) = dur {
        pause.set_duration(d);
    }
    if let Some(t) = pause_type {
        pause.set_kind(t);
    }
    pause
}
