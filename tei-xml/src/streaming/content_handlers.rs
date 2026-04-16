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
    /// Appends an `Inline::Text` node to whichever content buffer the current
    /// parser state owns, if any.  Returns `Ok(None)` regardless of state so
    /// callers can propagate it directly.  The `produce` closure is only
    /// invoked when the state carries an inline-content buffer.
    fn push_text_inline(
        &mut self,
        produce: impl FnOnce() -> Result<String, TeiError>,
    ) -> Result<Option<TeiEvent>, TeiError> {
        let maybe_content = match &mut self.state {
            ParserState::InParagraph { content, .. }
            | ParserState::InUtterance { content, .. }
            | ParserState::InEmphasis { content, .. }
            | ParserState::InItem { content, .. }
            | ParserState::InHead { content, .. }
            | ParserState::InLabel { content, .. } => Some(content),
            _ => None,
        };
        if let Some(content) = maybe_content {
            let text = produce()?;
            if !text.is_empty() {
                content.push(Inline::Text(text));
            }
        }
        Ok(None)
    }

    /// Handles a text event.
    pub(super) fn handle_text(
        &mut self,
        text: &BytesText<'_>,
    ) -> Result<Option<TeiEvent>, TeiError> {
        if let ParserState::InHeader { buffer, .. } = &mut self.state {
            buffer.extend_from_slice(text.as_ref());
            return Ok(None);
        }
        self.push_text_inline(|| {
            text.decode()
                .map(std::borrow::Cow::into_owned)
                .map_err(|e| TeiError::xml(e.to_string()))
        })
    }

    /// Handles a general entity reference (`&name;` or `&#...;`).
    pub(super) fn handle_general_ref(
        &mut self,
        reference: &BytesRef<'_>,
    ) -> Result<Option<TeiEvent>, TeiError> {
        if let ParserState::InHeader { buffer, .. } = &mut self.state {
            buffer.push(b'&');
            buffer.extend_from_slice(reference.as_ref());
            buffer.push(b';');
            return Ok(None);
        }
        self.push_text_inline(|| resolve_entity_ref(reference))
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
            ParserState::InParagraph { content, .. } => {
                handle_inline_empty_element(content, name_bytes, element, "InParagraph")
            }
            ParserState::InUtterance { content, .. } => {
                handle_inline_empty_element(content, name_bytes, element, "InUtterance")
            }
            ParserState::InEmphasis { content, .. } => {
                handle_inline_empty_element(content, name_bytes, element, "InEmphasis")
            }
            ParserState::InItem { content, .. } => {
                handle_inline_empty_element(content, name_bytes, element, "InItem")
            }
            ParserState::InHead { content, .. } => {
                handle_inline_empty_element(content, name_bytes, element, "InHead")
            }
            ParserState::InLabel { content, .. } => {
                handle_inline_empty_element(content, name_bytes, element, "InLabel")
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
            ParserState::InList { .. } if name_bytes == b"item" => {
                Err(unexpected_empty_element_error(name_bytes, "InList"))
            }
            ParserState::InList { .. } => Err(unexpected_empty_element_error(name_bytes, "InList")),
            ParserState::InDiv { content, .. } if name_bytes == b"div" => {
                let div = Self::build_empty_div(element)?;
                content.push(DivContent::Div(div));
                Ok(None)
            }
            ParserState::InBody => Err(unexpected_empty_element_error(name_bytes, "InBody")),
            ParserState::InDiv { .. } => Err(unexpected_empty_element_error(name_bytes, "InDiv")),
            ParserState::Initial
            | ParserState::AwaitingRoot
            | ParserState::AwaitingHeader
            | ParserState::AwaitingText
            | ParserState::AfterBody
            | ParserState::DocumentComplete
            | ParserState::Error => Ok(None),
        }
    }

    /// Handles CDATA sections.
    pub(super) fn handle_cdata(
        &mut self,
        cdata: &quick_xml::events::BytesCData<'_>,
    ) -> Result<Option<TeiEvent>, TeiError> {
        if let ParserState::InHeader { buffer, .. } = &mut self.state {
            buffer.extend_from_slice(b"<![CDATA[");
            buffer.extend_from_slice(cdata.as_ref());
            buffer.extend_from_slice(b"]]>");
            return Ok(None);
        }
        self.push_text_inline(|| {
            std::str::from_utf8(cdata.as_ref())
                .map(std::borrow::ToOwned::to_owned)
                .map_err(|e| TeiError::xml(format!("invalid UTF-8 in CDATA: {e}")))
        })
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

fn handle_inline_empty_element(
    content: &mut Vec<Inline>,
    name_bytes: &[u8],
    element: &BytesStart<'_>,
    state_name: &str,
) -> Result<Option<TeiEvent>, TeiError> {
    if name_bytes == b"pause" {
        let dur = extract_attribute(element, b"dur")?;
        let pause_type = extract_attribute(element, b"type")?;
        content.push(Inline::Pause(build_pause(dur, pause_type)));
        Ok(None)
    } else {
        Err(unexpected_empty_element_error(name_bytes, state_name))
    }
}

fn unexpected_empty_element_error(name_bytes: &[u8], state_name: &str) -> TeiError {
    let name = String::from_utf8_lossy(name_bytes);
    TeiError::xml(format!(
        "unexpected empty element <{name}/> while parsing state {state_name}"
    ))
}
