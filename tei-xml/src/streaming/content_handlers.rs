//! Text, entity, CDATA, and empty-element handlers for the TEI pull parser.

use std::io::BufRead;

use quick_xml::events::{BytesRef, BytesStart, BytesText};

use tei_core::{BodyBlock, DivContent, Inline, TeiError};

use super::event::TeiEvent;
use super::helpers::{
    RawDivAttrs, append_empty_element, build_div, build_pause, extract_attribute, extract_xml_id,
    resolve_entity_ref,
};
use super::parser::TeiPullParser;
use super::state::ParserState;

/// Content handlers (text, empty elements, CDATA, EOF).
impl<R: BufRead> TeiPullParser<R> {
    /// Handles a text event.
    pub(super) fn handle_text(
        &mut self,
        text: &BytesText<'_>,
    ) -> Result<Option<TeiEvent>, TeiError> {
        match &mut self.state {
            ParserState::InHeader { buffer, .. } => {
                buffer.extend_from_slice(text.as_ref());
                Ok(None)
            }
            ParserState::InParagraph { content, .. }
            | ParserState::InUtterance { content, .. }
            | ParserState::InEmphasis { content, .. }
            | ParserState::InItem { content, .. }
            | ParserState::InHead { content, .. }
            | ParserState::InLabel { content, .. } => {
                let unescaped = text.decode().map_err(|e| TeiError::xml(e.to_string()))?;
                if !unescaped.is_empty() {
                    content.push(Inline::Text(unescaped.into_owned()));
                }
                Ok(None)
            }
            _ => Ok(None),
        }
    }

    /// Handles a general entity reference (`&name;` or `&#...;`).
    pub(super) fn handle_general_ref(
        &mut self,
        reference: &BytesRef<'_>,
    ) -> Result<Option<TeiEvent>, TeiError> {
        match &mut self.state {
            ParserState::InHeader { buffer, .. } => {
                buffer.push(b'&');
                buffer.extend_from_slice(reference.as_ref());
                buffer.push(b';');
                Ok(None)
            }
            ParserState::InParagraph { content, .. }
            | ParserState::InUtterance { content, .. }
            | ParserState::InEmphasis { content, .. }
            | ParserState::InItem { content, .. }
            | ParserState::InHead { content, .. }
            | ParserState::InLabel { content, .. } => {
                let resolved = resolve_entity_ref(reference)?;
                content.push(Inline::Text(resolved));
                Ok(None)
            }
            _ => Ok(None),
        }
    }

    /// Handles an empty element event (self-closing tag).
    pub(super) fn handle_empty_element(
        &mut self,
        element: &BytesStart<'_>,
    ) -> Result<Option<TeiEvent>, TeiError> {
        let name = element.local_name();
        let name_bytes = name.as_ref();

        match &mut self.state {
            ParserState::InHeader { buffer, .. } => {
                append_empty_element(buffer, element)?;
                Ok(None)
            }
            ParserState::InParagraph { content, .. }
            | ParserState::InUtterance { content, .. }
            | ParserState::InEmphasis { content, .. }
            | ParserState::InItem { content, .. }
            | ParserState::InHead { content, .. }
            | ParserState::InLabel { content, .. }
                if name_bytes == b"pause" =>
            {
                let dur = extract_attribute(element, b"dur")?;
                let pause_type = extract_attribute(element, b"type")?;
                let pause = build_pause(dur, pause_type);
                content.push(Inline::Pause(pause));
                Ok(None)
            }
            ParserState::InParagraph { .. } => {
                Err(unexpected_empty_element_error(name_bytes, "InParagraph"))
            }
            ParserState::InUtterance { .. } => {
                Err(unexpected_empty_element_error(name_bytes, "InUtterance"))
            }
            ParserState::InEmphasis { .. } => {
                Err(unexpected_empty_element_error(name_bytes, "InEmphasis"))
            }
            ParserState::InItem { .. } => Err(unexpected_empty_element_error(name_bytes, "InItem")),
            ParserState::InHead { .. } => Err(unexpected_empty_element_error(name_bytes, "InHead")),
            ParserState::InLabel { .. } => {
                Err(unexpected_empty_element_error(name_bytes, "InLabel"))
            }
            ParserState::AwaitingBody if name_bytes == b"body" => {
                self.state = ParserState::DocumentComplete;
                Ok(Some(TeiEvent::DocumentEnd))
            }
            ParserState::AwaitingBody => {
                Err(unexpected_empty_element_error(name_bytes, "AwaitingBody"))
            }
            ParserState::InBody if name_bytes == b"div" => {
                let div = Self::build_empty_div(element)?;
                Ok(Some(TeiEvent::BodyBlock(BodyBlock::Div(div))))
            }
            ParserState::InDiv { content, .. } if name_bytes == b"div" => {
                let div = Self::build_empty_div(element)?;
                content.push(DivContent::Div(div));
                Ok(None)
            }
            ParserState::InBody => Err(unexpected_empty_element_error(name_bytes, "InBody")),
            ParserState::InDiv { .. } => Err(unexpected_empty_element_error(name_bytes, "InDiv")),
            _ => Ok(None),
        }
    }

    /// Handles CDATA sections.
    pub(super) fn handle_cdata(
        &mut self,
        cdata: &quick_xml::events::BytesCData<'_>,
    ) -> Result<Option<TeiEvent>, TeiError> {
        match &mut self.state {
            ParserState::InHeader { buffer, .. } => {
                buffer.extend_from_slice(b"<![CDATA[");
                buffer.extend_from_slice(cdata.as_ref());
                buffer.extend_from_slice(b"]]>");
                Ok(None)
            }
            ParserState::InParagraph { content, .. }
            | ParserState::InUtterance { content, .. }
            | ParserState::InEmphasis { content, .. }
            | ParserState::InItem { content, .. }
            | ParserState::InHead { content, .. }
            | ParserState::InLabel { content, .. } => {
                let text = std::str::from_utf8(cdata.as_ref())
                    .map_err(|e| TeiError::xml(format!("invalid UTF-8 in CDATA: {e}")))?;
                if !text.is_empty() {
                    content.push(Inline::Text(text.to_owned()));
                }
                Ok(None)
            }
            _ => Ok(None),
        }
    }

    /// Handles end-of-file.
    pub(super) fn handle_eof(&mut self) -> Result<Option<TeiEvent>, TeiError> {
        match &self.state {
            ParserState::DocumentComplete => Ok(None),
            ParserState::InBody | ParserState::AfterBody => {
                self.state = ParserState::DocumentComplete;
                Ok(Some(TeiEvent::DocumentEnd))
            }
            _ => {
                self.state = ParserState::Error;
                Err(TeiError::xml("unexpected end of document"))
            }
        }
    }

    fn build_empty_div(element: &BytesStart<'_>) -> Result<tei_core::Div, TeiError> {
        let div_type = extract_attribute(element, b"type")?
            .ok_or_else(|| TeiError::xml("div element missing required @type attribute"))?;
        let subtype = extract_attribute(element, b"subtype")?;
        let id = extract_xml_id(element)?;

        build_div(
            RawDivAttrs {
                div_type,
                subtype,
                id,
                head: None,
            },
            Vec::new(),
        )
    }
}

fn unexpected_empty_element_error(name_bytes: &[u8], state_name: &str) -> TeiError {
    let name = String::from_utf8_lossy(name_bytes);
    TeiError::xml(format!(
        "unexpected empty element <{name}/> while parsing state {state_name}"
    ))
}
