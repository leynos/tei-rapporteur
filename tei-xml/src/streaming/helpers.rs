//! Helper functions for the streaming TEI parser.
//!
//! These functions handle XML element construction, attribute extraction,
//! and building of TEI domain objects from parsed content.

use quick_xml::events::{BytesEnd, BytesStart};

use tei_core::{
    BodyContentError, Certainty, Div, DivContent, Hi, Inline, Item, Label, List, P, Pause,
    PointerList, TeiError, Utterance,
};

use super::state::RawUtteranceAttrs;

/// Applies an optional ID to a block element that supports `set_id`.
fn apply_id<T, F>(element: &mut T, id: Option<String>, setter: F) -> Result<(), TeiError>
where
    F: FnOnce(&mut T, String) -> Result<(), BodyContentError>,
{
    if let Some(id_str) = id {
        setter(element, id_str).map_err(|e| TeiError::xml(e.to_string()))?;
    }
    Ok(())
}

/// Generic builder for block elements with optional empty/inline content handling.
#[expect(
    clippy::too_many_arguments,
    reason = "generic builder requires multiple closures"
)]
fn build_block_with_content<T, E>(
    content: Vec<Inline>,
    empty_constructor: impl FnOnce() -> Result<T, E>,
    content_constructor: impl FnOnce(Vec<Inline>) -> Result<T, E>,
    id: Option<String>,
    id_setter: impl FnOnce(&mut T, String) -> Result<(), BodyContentError>,
) -> Result<T, TeiError>
where
    E: std::fmt::Display,
{
    let mut element = if content.is_empty() {
        empty_constructor().map_err(|e| TeiError::xml(e.to_string()))?
    } else {
        content_constructor(content).map_err(|e| TeiError::xml(e.to_string()))?
    };
    apply_id(&mut element, id, id_setter)?;
    Ok(element)
}

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
    build_block_with_content(
        content,
        || P::from_text_segments([""]),
        P::from_inline,
        id,
        P::set_id,
    )
}

fn apply_pointer_list(
    utterance: &mut Utterance,
    attribute_value: Option<String>,
    setter: impl FnOnce(&mut Utterance, PointerList),
) -> Result<(), TeiError> {
    if let Some(raw_value) = attribute_value {
        setter(
            utterance,
            PointerList::parse_attribute(raw_value).map_err(TeiError::from)?,
        );
    }
    Ok(())
}

/// Builds an utterance from raw TEI attributes and inline content.
pub fn build_utterance(
    attrs: RawUtteranceAttrs,
    content: Vec<Inline>,
) -> Result<Utterance, TeiError> {
    let RawUtteranceAttrs {
        id,
        n,
        who,
        source,
        resp,
        cert,
        corresp,
        ana,
    } = attrs;

    let mut utterance = build_block_with_content(
        content,
        || Utterance::from_text_segments(who.as_deref(), [""]),
        |c| Utterance::from_inline(who.as_deref(), c),
        id,
        Utterance::set_id,
    )?;

    if let Some(number) = n {
        utterance.set_number(number);
    }
    apply_pointer_list(&mut utterance, source, Utterance::set_source)?;
    apply_pointer_list(&mut utterance, resp, Utterance::set_resp)?;
    if let Some(certainty) = cert {
        utterance.set_cert(Certainty::new(certainty).map_err(TeiError::from)?);
    }
    apply_pointer_list(&mut utterance, corresp, Utterance::set_corresp)?;
    apply_pointer_list(&mut utterance, ana, Utterance::set_ana)?;

    Ok(utterance)
}

/// Builds an emphasis (hi) element from an optional rendition and content.
///
/// Uses validating constructors to reject empty content.
///
/// # Errors
///
/// Returns `TeiError::Xml` if the content is empty or contains only whitespace.
pub fn build_hi(rend: Option<String>, content: Vec<Inline>) -> Result<Hi, TeiError> {
    let result = match rend {
        Some(r) => Hi::try_with_rend(r, content),
        None => Hi::try_new(content),
    };
    result.map_err(|e| TeiError::xml(e.to_string()))
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

/// Builds a Div from type, optional ID, and content.
pub fn build_div(
    div_type: String,
    id: Option<String>,
    content: Vec<DivContent>,
) -> Result<Div, TeiError> {
    let mut div = Div::new(div_type).map_err(|e| TeiError::xml(e.to_string()))?;
    apply_id(&mut div, id, Div::set_id)?;
    for item in content {
        match item {
            DivContent::Paragraph(p) => div.push_paragraph(p),
            DivContent::Utterance(u) => div.push_utterance(u),
            DivContent::List(l) => div.push_list(l),
        }
    }
    Ok(div)
}

/// Builds a List from optional ID and items.
pub fn build_list(id: Option<String>, items: Vec<Item>) -> Result<List, TeiError> {
    let mut list = List::new(items);
    apply_id(&mut list, id, List::set_id)?;
    Ok(list)
}

/// Raw attributes collected for an `<item>` element during parsing.
pub struct RawItemAttrs {
    /// Optional `xml:id` attribute.
    pub id: Option<String>,
    /// Optional `n` attribute.
    pub n: Option<String>,
    /// Optional `corresp` attribute (unparsed pointer list).
    pub corresp: Option<String>,
    /// Optional `<label>` child element.
    pub label: Option<Label>,
}

/// Builds an `Item` from raw attributes and inline content.
pub fn build_item(attrs: RawItemAttrs, content: Vec<Inline>) -> Result<Item, TeiError> {
    let mut item = if content.is_empty() {
        Item::from_text_segments([""])
    } else {
        Item::new(content)
    }
    .map_err(|e| TeiError::xml(e.to_string()))?;

    apply_id(&mut item, attrs.id, Item::set_id)?;

    if let Some(number) = attrs.n {
        item.set_n(number)
            .map_err(|e| TeiError::xml(e.to_string()))?;
    }

    if let Some(corresp_str) = attrs.corresp {
        item.set_corresp(PointerList::parse_attribute(corresp_str).map_err(TeiError::from)?);
    }

    if let Some(lbl) = attrs.label {
        item.set_label(lbl);
    }

    Ok(item)
}

/// Builds a Label from inline content.
pub fn build_label(content: Vec<Inline>) -> Result<Label, TeiError> {
    if content.is_empty() {
        Label::from_text("").map_err(|e| TeiError::xml(e.to_string()))
    } else {
        Label::new(content).map_err(|e| TeiError::xml(e.to_string()))
    }
}
