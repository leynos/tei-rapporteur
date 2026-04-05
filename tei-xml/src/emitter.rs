//! Custom XML emitter for TEI documents.
//!
//! The `quick_xml` serde serializer (v0.38+) rejects sequences of untagged
//! primitives in `$value` fields because consecutive text nodes would merge
//! without delimiters. Since `Vec<Inline>` uses `#[serde(untagged)]` and
//! contains both `Text(String)` and structured variants, serde-based emission
//! fails for any body content.
//!
//! This module works around the limitation by delegating header and stand-off
//! serialization to serde (which handles those subtrees without issue) and
//! hand-writing the body portion using direct string construction.

use tei_core::{
    BodyBlock, Div, DivContent, Hi, Inline, Item, Label, List, P, Pause, TeiDocument, TeiError,
    Utterance,
};

use crate::escape_xml_text;

/// Emits a [`TeiDocument`] as TEI XML markup.
///
/// Delegates header and optional stand-off annotation to the serde serializer,
/// then writes the body using direct XML construction to avoid the untagged
/// primitive limitation in `quick_xml`'s serde integration.
///
/// # Errors
///
/// Returns [`TeiError::Xml`] when any component cannot be represented as
/// well-formed XML.
pub fn emit_document(document: &TeiDocument) -> Result<String, TeiError> {
    let mut output = String::with_capacity(512);
    output.push_str("<TEI>");

    emit_header(&mut output, document)?;
    emit_stand_off(&mut output, document)?;
    emit_text(&mut output, document);

    output.push_str("</TEI>");
    Ok(output)
}

/// Serializes the header via serde — it contains no `Vec<Inline>` fields.
fn emit_header(output: &mut String, document: &TeiDocument) -> Result<(), TeiError> {
    let header_xml =
        quick_xml::se::to_string(document.header()).map_err(|e| TeiError::xml(e.to_string()))?;
    output.push_str(&header_xml);
    Ok(())
}

/// Serializes the optional stand-off annotation via serde.
fn emit_stand_off(output: &mut String, document: &TeiDocument) -> Result<(), TeiError> {
    if let Some(stand_off) = document.stand_off() {
        let xml = quick_xml::se::to_string(stand_off).map_err(|e| TeiError::xml(e.to_string()))?;
        output.push_str(&xml);
    }
    Ok(())
}

/// Writes the `<text><body>...</body></text>` portion by hand.
fn emit_text(output: &mut String, document: &TeiDocument) {
    let body = document.text().body();
    if body.is_empty() {
        output.push_str("<text><body/></text>");
    } else {
        output.push_str("<text><body>");
        for block in body.blocks() {
            emit_body_block(output, block);
        }
        output.push_str("</body></text>");
    }
}

/// Dispatches a single body block to its element-specific emitter.
fn emit_body_block(output: &mut String, block: &BodyBlock) {
    match block {
        BodyBlock::Paragraph(p) => emit_paragraph(output, p),
        BodyBlock::Utterance(u) => emit_utterance(output, u),
        BodyBlock::Div(div) => emit_div(output, div),
    }
}

/// Writes `<p xml:id="...">inline content</p>`.
fn emit_paragraph(output: &mut String, paragraph: &P) {
    output.push_str("<p");
    emit_optional_xml_id(output, paragraph.id());
    if paragraph.content().is_empty() {
        output.push_str("/>");
    } else {
        output.push('>');
        emit_inline_sequence(output, paragraph.content());
        output.push_str("</p>");
    }
}

/// Writes `<u who="..." xml:id="..." ...>inline content</u>`.
fn emit_utterance(output: &mut String, utterance: &Utterance) {
    output.push_str("<u");
    emit_optional_xml_id(output, utterance.id());
    emit_optional_attr(output, "n", utterance.number());
    emit_optional_attr(
        output,
        "who",
        utterance.speaker().map(tei_core::Speaker::as_str),
    );
    emit_optional_pointer_list_attr(output, "source", utterance.source());
    emit_optional_pointer_list_attr(output, "resp", utterance.resp());
    emit_optional_attr(
        output,
        "cert",
        utterance.cert().map(tei_core::Certainty::as_str),
    );
    emit_optional_pointer_list_attr(output, "corresp", utterance.corresp());
    emit_optional_pointer_list_attr(output, "ana", utterance.ana());
    if utterance.content().is_empty() {
        output.push_str("/>");
    } else {
        output.push('>');
        emit_inline_sequence(output, utterance.content());
        output.push_str("</u>");
    }
}

/// Writes `<div type="..." xml:id="...">div content</div>`.
fn emit_div(output: &mut String, div: &Div) {
    output.push_str("<div");
    emit_attr(output, "type", div.div_type());
    emit_optional_xml_id(output, div.id());
    if div.is_empty() {
        output.push_str("/>");
    } else {
        output.push('>');
        for child in div.content() {
            emit_div_content(output, child);
        }
        output.push_str("</div>");
    }
}

/// Dispatches a single div-level block.
fn emit_div_content(output: &mut String, content: &DivContent) {
    match content {
        DivContent::Paragraph(p) => emit_paragraph(output, p),
        DivContent::Utterance(u) => emit_utterance(output, u),
        DivContent::List(list) => emit_list(output, list),
    }
}

/// Writes `<list xml:id="..."><item>...</item></list>`.
fn emit_list(output: &mut String, list: &List) {
    output.push_str("<list");
    emit_optional_xml_id(output, list.id());
    if list.is_empty() {
        output.push_str("/>");
    } else {
        output.push('>');
        for item in list.items() {
            emit_item(output, item);
        }
        output.push_str("</list>");
    }
}

/// Writes `<item n="..." xml:id="..." corresp="..."><label>...</label>inline</item>`.
fn emit_item(output: &mut String, item: &Item) {
    output.push_str("<item");
    emit_optional_xml_id(output, item.id());
    emit_optional_attr(output, "n", item.n());
    emit_optional_pointer_list_attr(output, "corresp", item.corresp());
    output.push('>');
    if let Some(label) = item.label() {
        emit_label(output, label);
    }
    emit_inline_sequence(output, item.content());
    output.push_str("</item>");
}

/// Writes `<label>inline content</label>`.
fn emit_label(output: &mut String, label: &Label) {
    output.push_str("<label>");
    emit_inline_sequence(output, label.content());
    output.push_str("</label>");
}

/// Writes a sequence of inline nodes (text, hi, pause).
fn emit_inline_sequence(output: &mut String, inlines: &[Inline]) {
    for inline in inlines {
        match inline {
            Inline::Text(text) => output.push_str(&escape_xml_text(text)),
            Inline::Hi(hi) => emit_hi(output, hi),
            Inline::Pause(pause) => emit_pause(output, pause),
        }
    }
}

/// Writes `<hi rend="...">inline content</hi>`.
fn emit_hi(output: &mut String, hi: &Hi) {
    output.push_str("<hi");
    emit_optional_attr(output, "rend", hi.rend());
    if hi.content().is_empty() {
        output.push_str("/>");
    } else {
        output.push('>');
        emit_inline_sequence(output, hi.content());
        output.push_str("</hi>");
    }
}

/// Writes `<pause dur="..." type="..."/>`.
fn emit_pause(output: &mut String, pause: &Pause) {
    output.push_str("<pause");
    emit_optional_attr(output, "dur", pause.duration());
    emit_optional_attr(output, "type", pause.kind());
    output.push_str("/>");
}

// --- Attribute helpers ---

/// Writes ` xml:id="value"` when the id is present.
fn emit_optional_xml_id(output: &mut String, id: Option<&tei_core::XmlId>) {
    if let Some(xml_id) = id {
        emit_attr(output, "xml:id", xml_id.as_str());
    }
}

/// Writes ` name="value"` when the value is present.
fn emit_optional_attr(output: &mut String, name: &str, value: Option<&str>) {
    if let Some(val) = value {
        emit_attr(output, name, val);
    }
}

/// Writes ` name="ptr1 ptr2 ..."` for a pointer list attribute.
fn emit_optional_pointer_list_attr(
    output: &mut String,
    name: &str,
    pointer_list: Option<&tei_core::PointerList>,
) {
    if let Some(pl) = pointer_list {
        let joined = pl.to_strings().join(" ");
        emit_attr(output, name, &joined);
    }
}

/// Writes ` name="escaped_value"`.
fn emit_attr(output: &mut String, name: &str, value: &str) {
    output.push(' ');
    output.push_str(name);
    output.push_str("=\"");
    output.push_str(&escape_xml_text(value));
    output.push('"');
}
