//! Helper functions for the streaming TEI parser.
//!
//! These functions handle XML element construction, attribute extraction,
//! and building of TEI domain objects from parsed content.

use quick_xml::events::{BytesEnd, BytesRef, BytesStart};

use tei_core::{
    BodyContentError, Certainty, Div, DivContent, Head, Hi, Inline, Item, Label, List, P, Pause,
    PointerList, TeiError, Utterance,
};

use crate::attributes::{NormalizedAttributes, extract_normalized_attribute};

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
    extract_normalized_attribute(element, name)
}

/// Extracts all `<u>` attributes from an element with a single attribute scan.
pub fn extract_utterance_attrs(element: &BytesStart<'_>) -> Result<RawUtteranceAttrs, TeiError> {
    let attributes = NormalizedAttributes::from_element(element)?;
    Ok(RawUtteranceAttrs {
        id: attributes.get(b"xml:id"),
        n: attributes.get(b"n"),
        who: attributes.get(b"who"),
        source: attributes.get(b"source"),
        resp: attributes.get(b"resp"),
        cert: attributes.get(b"cert"),
        corresp: attributes.get(b"corresp"),
        ana: attributes.get(b"ana"),
    })
}

/// Extracts `<div>` type, subtype, and ID attributes with a single scan.
pub fn extract_div_attrs(
    element: &BytesStart<'_>,
    head: Option<Head>,
) -> Result<RawDivAttrs, TeiError> {
    let attributes = NormalizedAttributes::from_element(element)?;
    Ok(RawDivAttrs {
        div_type: attributes
            .get(b"type")
            .ok_or_else(|| TeiError::xml("div element missing required @type attribute"))?,
        subtype: attributes.get(b"subtype"),
        id: attributes.get(b"xml:id"),
        head,
    })
}

/// Extracts `<item>` attributes with a single attribute scan.
pub fn extract_item_attrs(
    element: &BytesStart<'_>,
    label: Option<Label>,
) -> Result<RawItemAttrs, TeiError> {
    let attributes = NormalizedAttributes::from_element(element)?;
    Ok(RawItemAttrs {
        id: attributes.get(b"xml:id"),
        n: attributes.get(b"n"),
        corresp: attributes.get(b"corresp"),
        label,
    })
}

/// Extracts `<pause>` attributes with a single attribute scan.
pub fn extract_pause_attrs(
    element: &BytesStart<'_>,
) -> Result<(Option<String>, Option<String>), TeiError> {
    let attributes = NormalizedAttributes::from_element(element)?;
    Ok((attributes.get(b"dur"), attributes.get(b"type")))
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

/// Raw attributes collected for a `<div>` element during parsing.
pub struct RawDivAttrs {
    /// Required `@type` attribute.
    pub div_type: String,
    /// Optional `@subtype` attribute.
    pub subtype: Option<String>,
    /// Optional `xml:id` attribute.
    pub id: Option<String>,
    /// Optional `<head>` child element.
    pub head: Option<Head>,
}

/// Builds a Div from raw attributes and child content.
pub fn build_div(attrs: RawDivAttrs, content: Vec<DivContent>) -> Result<Div, TeiError> {
    let mut div = Div::new(attrs.div_type).map_err(|e| TeiError::xml(e.to_string()))?;
    if let Some(subtype_value) = attrs.subtype {
        div.set_subtype(subtype_value)
            .map_err(|e| TeiError::xml(e.to_string()))?;
    }
    apply_id(&mut div, attrs.id, Div::set_id)?;
    if let Some(heading) = attrs.head {
        div.set_head(heading);
    }
    for item in content {
        match item {
            DivContent::Paragraph(p) => div.push_paragraph(p),
            DivContent::Utterance(u) => div.push_utterance(u),
            DivContent::List(l) => div.push_list(l),
            DivContent::Div(nested_div) => div.push_div(nested_div),
        }
    }
    Ok(div)
}

/// Builds a List from optional ID and items.
pub fn build_list(id: Option<String>, items: Vec<Item>) -> Result<List, TeiError> {
    let mut list = List::new(items)?;
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

/// Builds a Head from inline content.
pub fn build_head(content: Vec<Inline>) -> Result<Head, TeiError> {
    Head::new(content).map_err(|e| TeiError::xml(e.to_string()))
}

/// Resolves a `BytesRef` entity reference to its text representation.
///
/// Handles the five predefined XML entities (`lt`, `gt`, `amp`, `quot`,
/// `apos`) and numeric character references (`&#...;`, `&#x...;`).
/// Returns an error for unrecognised named entities.
pub fn resolve_entity_ref(reference: &BytesRef<'_>) -> Result<String, TeiError> {
    let name = reference
        .decode()
        .map_err(|e| TeiError::xml(e.to_string()))?;

    match name.as_ref() {
        "lt" => Ok("<".to_owned()),
        "gt" => Ok(">".to_owned()),
        "amp" => Ok("&".to_owned()),
        "quot" => Ok("\"".to_owned()),
        "apos" => Ok("'".to_owned()),
        _ => match reference.resolve_char_ref() {
            Ok(Some(ch)) => Ok(ch.to_string()),
            Ok(None) => Err(TeiError::xml(format!(
                "unrecognised entity reference: &{name};"
            ))),
            Err(e) => Err(TeiError::xml(e.to_string())),
        },
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for streaming attribute extraction helpers.

    use quick_xml::events::BytesStart;

    use super::*;

    #[test]
    fn utterance_attrs_all_fields_extracted() {
        let mut el = BytesStart::new("u");
        el.push_attribute(("xml:id", "u1"));
        el.push_attribute(("n", "1"));
        el.push_attribute(("who", "#speaker1"));
        el.push_attribute(("source", "#src1"));
        el.push_attribute(("resp", "#resp1"));
        el.push_attribute(("cert", "high"));
        el.push_attribute(("corresp", "#u2"));
        el.push_attribute(("ana", "#ana1"));

        let attrs = extract_utterance_attrs(&el).unwrap_or_else(|error| panic!("{error}"));

        assert_eq!(attrs.id.as_deref(), Some("u1"));
        assert_eq!(attrs.n.as_deref(), Some("1"));
        assert_eq!(attrs.who.as_deref(), Some("#speaker1"));
        assert_eq!(attrs.source.as_deref(), Some("#src1"));
        assert_eq!(attrs.resp.as_deref(), Some("#resp1"));
        assert_eq!(attrs.cert.as_deref(), Some("high"));
        assert_eq!(attrs.corresp.as_deref(), Some("#u2"));
        assert_eq!(attrs.ana.as_deref(), Some("#ana1"));
    }

    #[test]
    fn utterance_attrs_absent_fields_are_none() {
        let el = BytesStart::new("u");

        let attrs = extract_utterance_attrs(&el).unwrap_or_else(|error| panic!("{error}"));

        assert!(attrs.id.is_none());
        assert!(attrs.n.is_none());
        assert!(attrs.who.is_none());
        assert!(attrs.source.is_none());
        assert!(attrs.resp.is_none());
        assert!(attrs.cert.is_none());
        assert!(attrs.corresp.is_none());
        assert!(attrs.ana.is_none());
    }

    #[test]
    fn utterance_attrs_unknown_entity_returns_error() {
        let el = BytesStart::from_content(r#"u who="&badentity;""#, 1);

        assert!(
            extract_utterance_attrs(&el).is_err(),
            "unknown entity reference must produce an error"
        );
    }

    #[test]
    fn div_attrs_required_and_optional_fields_extracted() {
        let mut el = BytesStart::new("div");
        el.push_attribute(("type", "interview"));
        el.push_attribute(("subtype", "formal"));
        el.push_attribute(("xml:id", "d1"));

        let attrs = extract_div_attrs(&el, None).unwrap_or_else(|error| panic!("{error}"));

        assert_eq!(attrs.div_type, "interview");
        assert_eq!(attrs.subtype.as_deref(), Some("formal"));
        assert_eq!(attrs.id.as_deref(), Some("d1"));
        assert!(attrs.head.is_none());
    }

    #[test]
    fn div_attrs_optional_fields_absent() {
        let mut el = BytesStart::new("div");
        el.push_attribute(("type", "session"));

        let attrs = extract_div_attrs(&el, None).unwrap_or_else(|error| panic!("{error}"));

        assert_eq!(attrs.div_type, "session");
        assert!(attrs.subtype.is_none());
        assert!(attrs.id.is_none());
    }

    #[test]
    fn div_attrs_missing_required_type_returns_error() {
        let el = BytesStart::new("div");

        assert!(
            extract_div_attrs(&el, None).is_err(),
            "absent `@type` attribute must produce an error"
        );
    }

    #[test]
    fn div_attrs_unknown_entity_in_type_returns_error() {
        let el = BytesStart::from_content(r#"div type="&badentity;""#, 3);

        assert!(
            extract_div_attrs(&el, None).is_err(),
            "unknown entity reference in `@type` must produce an error"
        );
    }

    #[test]
    fn item_attrs_all_optional_fields_extracted() {
        let mut el = BytesStart::new("item");
        el.push_attribute(("xml:id", "i1"));
        el.push_attribute(("n", "42"));
        el.push_attribute(("corresp", "#i2"));

        let attrs = extract_item_attrs(&el, None).unwrap_or_else(|error| panic!("{error}"));

        assert_eq!(attrs.id.as_deref(), Some("i1"));
        assert_eq!(attrs.n.as_deref(), Some("42"));
        assert_eq!(attrs.corresp.as_deref(), Some("#i2"));
        assert!(attrs.label.is_none());
    }

    #[test]
    fn item_attrs_all_absent_fields_are_none() {
        let el = BytesStart::new("item");

        let attrs = extract_item_attrs(&el, None).unwrap_or_else(|error| panic!("{error}"));

        assert!(attrs.id.is_none());
        assert!(attrs.n.is_none());
        assert!(attrs.corresp.is_none());
        assert!(attrs.label.is_none());
    }

    #[test]
    fn item_attrs_unknown_entity_returns_error() {
        let el = BytesStart::from_content(r#"item n="&badentity;""#, 4);

        assert!(
            extract_item_attrs(&el, None).is_err(),
            "unknown entity reference must produce an error"
        );
    }

    #[test]
    fn pause_attrs_both_fields_extracted() {
        let mut el = BytesStart::new("pause");
        el.push_attribute(("dur", "PT1S"));
        el.push_attribute(("type", "short"));

        let (dur, pause_type) = extract_pause_attrs(&el).unwrap_or_else(|error| panic!("{error}"));

        assert_eq!(dur.as_deref(), Some("PT1S"));
        assert_eq!(pause_type.as_deref(), Some("short"));
    }

    #[test]
    fn pause_attrs_both_absent_are_none() {
        let el = BytesStart::new("pause");

        let (dur, pause_type) = extract_pause_attrs(&el).unwrap_or_else(|error| panic!("{error}"));

        assert!(dur.is_none());
        assert!(pause_type.is_none());
    }

    #[test]
    fn pause_attrs_unknown_entity_returns_error() {
        let el = BytesStart::from_content(r#"pause dur="&badentity;""#, 5);

        assert!(
            extract_pause_attrs(&el).is_err(),
            "unknown entity reference must produce an error"
        );
    }
}
