//! State-specific event handlers for the TEI pull parser.
//!
//! This module contains the handler methods for processing XML events in
//! different parser states. These methods are implemented as part of
//! [`TeiPullParser`] but are separated into this file for modularity.

use std::io::BufRead;

use quick_xml::events::{BytesEnd, BytesStart, BytesText};

use tei_core::{BodyBlock, DivContent, Inline, TeiError};

use super::event::TeiEvent;
use super::helpers::{
    append_empty_element, append_end_element, append_start_element, build_div, build_hi, build_item,
    build_label, build_list, build_paragraph, build_pause, build_utterance, extract_attribute,
    extract_xml_id,
};
use super::parser::TeiPullParser;
use super::state::{ParserState, RawUtteranceAttrs};

/// Start element handlers.
impl<R: BufRead> TeiPullParser<R> {
    /// Handles start elements when awaiting the root TEI element.
    pub(super) fn handle_root_start(&mut self, name_bytes: &[u8]) -> Option<TeiEvent> {
        if name_bytes == b"TEI" {
            self.state = ParserState::AwaitingHeader;
        }
        None
    }

    /// Handles start elements when awaiting the teiHeader element.
    pub(super) fn handle_awaiting_header_start(
        &mut self,
        name_bytes: &[u8],
        element: &BytesStart<'_>,
    ) -> Result<Option<TeiEvent>, TeiError> {
        if name_bytes == b"teiHeader" {
            self.state = ParserState::in_header(1);
            if let ParserState::InHeader { buffer, .. } = &mut self.state {
                append_start_element(buffer, element)?;
            }
        }
        Ok(None)
    }

    /// Handles start elements when inside the header.
    pub(super) fn handle_in_header_start(
        &mut self,
        element: &BytesStart<'_>,
    ) -> Result<Option<TeiEvent>, TeiError> {
        if let ParserState::InHeader { depth, buffer } = &mut self.state {
            *depth += 1;
            append_start_element(buffer, element)?;
        }
        Ok(None)
    }

    /// Handles start elements when awaiting the text element.
    pub(super) fn handle_awaiting_text_start(&mut self, name_bytes: &[u8]) -> Option<TeiEvent> {
        if name_bytes == b"text" {
            self.state = ParserState::AwaitingBody;
        }
        None
    }

    /// Handles start elements when awaiting the body element.
    pub(super) fn handle_awaiting_body_start(&mut self, name_bytes: &[u8]) -> Option<TeiEvent> {
        if name_bytes == b"body" {
            self.state = ParserState::InBody;
        }
        None
    }

    /// Handles start elements for body content (paragraphs, utterances, and divs).
    pub(super) fn handle_body_content_start(
        &mut self,
        name_bytes: &[u8],
        element: &BytesStart<'_>,
    ) -> Result<Option<TeiEvent>, TeiError> {
        if name_bytes == b"p" {
            let id = extract_xml_id(element)?;
            self.state = ParserState::in_paragraph(id);
        } else if name_bytes == b"u" {
            self.state = ParserState::in_utterance(RawUtteranceAttrs {
                id: extract_xml_id(element)?,
                n: extract_attribute(element, b"n")?,
                who: extract_attribute(element, b"who")?,
                source: extract_attribute(element, b"source")?,
                resp: extract_attribute(element, b"resp")?,
                cert: extract_attribute(element, b"cert")?,
                corresp: extract_attribute(element, b"corresp")?,
                ana: extract_attribute(element, b"ana")?,
            });
        } else if name_bytes == b"div" {
            let div_type = extract_attribute(element, b"type")?
                .ok_or_else(|| TeiError::xml("div element missing required @type attribute"))?;
            let id = extract_xml_id(element)?;
            self.state = ParserState::in_div(div_type, id);
        }
        Ok(None)
    }

    /// Handles start elements for inline content (emphasis).
    pub(super) fn handle_block_content_start(
        &mut self,
        name_bytes: &[u8],
        element: &BytesStart<'_>,
    ) -> Result<Option<TeiEvent>, TeiError> {
        if name_bytes == b"hi" {
            let rend = extract_attribute(element, b"rend")?;
            self.state.transition_to_emphasis(rend);
        }
        Ok(None)
    }

    /// Handles start elements within a `<div>` (paragraphs, utterances, lists).
    pub(super) fn handle_div_content_start(
        &mut self,
        name_bytes: &[u8],
        element: &BytesStart<'_>,
    ) -> Result<Option<TeiEvent>, TeiError> {
        if name_bytes == b"p" {
            let id = extract_xml_id(element)?;
            let current_state = std::mem::take(&mut self.state);
            self.state = ParserState::in_paragraph(id);
            self.pending_div_state = Some(Box::new(current_state));
        } else if name_bytes == b"u" {
            let current_state = std::mem::take(&mut self.state);
            self.state = ParserState::in_utterance(RawUtteranceAttrs {
                id: extract_xml_id(element)?,
                n: extract_attribute(element, b"n")?,
                who: extract_attribute(element, b"who")?,
                source: extract_attribute(element, b"source")?,
                resp: extract_attribute(element, b"resp")?,
                cert: extract_attribute(element, b"cert")?,
                corresp: extract_attribute(element, b"corresp")?,
                ana: extract_attribute(element, b"ana")?,
            });
            self.pending_div_state = Some(Box::new(current_state));
        } else if name_bytes == b"list" {
            let list_id = extract_xml_id(element)?;
            let current_state = std::mem::take(&mut self.state);
            self.state = ParserState::in_list(current_state, list_id);
        }
        Ok(None)
    }

    /// Handles start elements within a `<list>` (items).
    pub(super) fn handle_list_content_start(
        &mut self,
        name_bytes: &[u8],
        element: &BytesStart<'_>,
    ) -> Result<Option<TeiEvent>, TeiError> {
        if name_bytes == b"item" {
            let item_id = extract_xml_id(element)?;
            let item_n = extract_attribute(element, b"n")?;
            let item_corresp = extract_attribute(element, b"corresp")?;
            let current_state = std::mem::take(&mut self.state);
            self.state = ParserState::in_item(current_state, item_id, item_n, item_corresp);
        }
        Ok(None)
    }

    /// Handles start elements within an `<item>` (label, inline content).
    pub(super) fn handle_item_content_start(
        &mut self,
        name_bytes: &[u8],
        element: &BytesStart<'_>,
    ) -> Result<Option<TeiEvent>, TeiError> {
        if name_bytes == b"label" {
            let current_state = std::mem::take(&mut self.state);
            self.state = ParserState::in_label(current_state);
        } else if name_bytes == b"hi" {
            let rend = extract_attribute(element, b"rend")?;
            self.state.transition_to_emphasis(rend);
        }
        Ok(None)
    }
}

/// End element handlers.
impl<R: BufRead> TeiPullParser<R> {
    /// Handles closing of header elements and emits Header event when complete.
    pub(super) fn handle_header_end(
        &mut self,
        element: &BytesEnd<'_>,
    ) -> Result<Option<TeiEvent>, TeiError> {
        if let ParserState::InHeader { depth, buffer } = &mut self.state {
            append_end_element(buffer, element);
            *depth -= 1;
            if *depth == 0 {
                let header = self.parse_accumulated_header()?;
                self.header = Some(header.clone());
                self.state = ParserState::AwaitingText;
                return Ok(Some(TeiEvent::Header(header)));
            }
        }
        Ok(None)
    }

    /// Finishes parsing a paragraph and emits a `BodyBlock` event or pushes to div.
    pub(super) fn finish_paragraph(&mut self) -> Result<Option<TeiEvent>, TeiError> {
        if let ParserState::InParagraph { id, content } = &mut self.state {
            let paragraph = build_paragraph(id.take(), std::mem::take(content))?;

            // Check if we're inside a div
            if let Some(mut parent_div) = self.pending_div_state.take() {
                if let ParserState::InDiv { content, .. } = parent_div.as_mut() {
                    content.push(DivContent::Paragraph(paragraph));
                    self.state = *parent_div;
                    return Ok(None);
                }
            }

            // Not in a div, emit as body block
            self.state = ParserState::InBody;
            return Ok(Some(TeiEvent::BodyBlock(BodyBlock::Paragraph(paragraph))));
        }
        Ok(None)
    }

    /// Finishes parsing an utterance and emits a `BodyBlock` event or pushes to div.
    pub(super) fn finish_utterance(&mut self) -> Result<Option<TeiEvent>, TeiError> {
        if let ParserState::InUtterance { attrs, content } = &mut self.state {
            let utterance = build_utterance(std::mem::take(attrs), std::mem::take(content))?;

            // Check if we're inside a div
            if let Some(mut parent_div) = self.pending_div_state.take() {
                if let ParserState::InDiv { content, .. } = parent_div.as_mut() {
                    content.push(DivContent::Utterance(utterance));
                    self.state = *parent_div;
                    return Ok(None);
                }
            }

            // Not in a div, emit as body block
            self.state = ParserState::InBody;
            return Ok(Some(TeiEvent::BodyBlock(BodyBlock::Utterance(utterance))));
        }
        Ok(None)
    }

    /// Finishes parsing emphasis and pushes it to the parent state.
    pub(super) fn finish_emphasis(&mut self) -> Result<Option<TeiEvent>, TeiError> {
        if let ParserState::InEmphasis {
            parent,
            rend,
            content,
        } = &mut self.state
        {
            let hi = build_hi(rend.take(), std::mem::take(content))?;
            let Some(parent_state) = parent.take() else {
                return Err(TeiError::xml("internal error: InEmphasis parent was None"));
            };
            self.state = *parent_state;
            self.state.push_inline(Inline::Hi(hi));
        }
        Ok(None)
    }

    /// Finishes parsing a label and stores it in the parent item.
    pub(super) fn finish_label(&mut self) -> Result<Option<TeiEvent>, TeiError> {
        if let ParserState::InLabel {
            parent_item,
            content,
        } = &mut self.state
        {
            let label = build_label(std::mem::take(content))?;
            let Some(mut parent_state) = parent_item.take() else {
                return Err(TeiError::xml("internal error: InLabel parent was None"));
            };
            if let ParserState::InItem {
                label: item_label, ..
            } = parent_state.as_mut()
            {
                *item_label = Some(label);
            }
            self.state = *parent_state;
        }
        Ok(None)
    }

    /// Finishes parsing an item and pushes it to the parent list.
    pub(super) fn finish_item(&mut self) -> Result<Option<TeiEvent>, TeiError> {
        if let ParserState::InItem {
            parent_list,
            item_id,
            item_n,
            item_corresp,
            label,
            content,
        } = &mut self.state
        {
            let item = build_item(
                item_id.take(),
                item_n.take(),
                item_corresp.take(),
                label.take(),
                std::mem::take(content),
            )?;
            let Some(mut parent_state) = parent_list.take() else {
                return Err(TeiError::xml("internal error: InItem parent was None"));
            };
            if let ParserState::InList { items, .. } = parent_state.as_mut() {
                items.push(item);
            }
            self.state = *parent_state;
        }
        Ok(None)
    }

    /// Finishes parsing a list and pushes it to the parent div.
    pub(super) fn finish_list(&mut self) -> Result<Option<TeiEvent>, TeiError> {
        if let ParserState::InList {
            parent_div,
            list_id,
            items,
        } = &mut self.state
        {
            let list = build_list(list_id.take(), std::mem::take(items))?;
            let Some(mut parent_state) = parent_div.take() else {
                return Err(TeiError::xml("internal error: InList parent was None"));
            };
            if let ParserState::InDiv { content, .. } = parent_state.as_mut() {
                content.push(DivContent::List(list));
            }
            self.state = *parent_state;
        }
        Ok(None)
    }

    /// Finishes parsing a div and emits a `BodyBlock` event.
    pub(super) fn finish_div(&mut self) -> Result<Option<TeiEvent>, TeiError> {
        if let ParserState::InDiv {
            div_type,
            id,
            content,
        } = &mut self.state
        {
            let div = build_div(
                std::mem::take(div_type),
                id.take(),
                std::mem::take(content),
            )?;
            self.state = ParserState::InBody;
            return Ok(Some(TeiEvent::BodyBlock(BodyBlock::Div(div))));
        }
        Ok(None)
    }

    /// Handles body end elements.
    pub(super) fn handle_body_end(&mut self, name_bytes: &[u8]) -> Option<TeiEvent> {
        match name_bytes {
            b"body" => {
                // Body closed, transition to AfterBody state
                self.state = ParserState::AfterBody;
                None
            }
            _ => None,
        }
    }

    /// Handles end elements after the body has closed.
    pub(super) fn handle_after_body_end(&mut self, name_bytes: &[u8]) -> Option<TeiEvent> {
        match name_bytes {
            b"text" | b"TEI" => {
                self.state = ParserState::DocumentComplete;
                Some(TeiEvent::DocumentEnd)
            }
            _ => None,
        }
    }
}

/// Content handlers (text, empty elements, CDATA, EOF).
impl<R: BufRead> TeiPullParser<R> {
    /// Handles a text event.
    pub(super) fn handle_text(
        &mut self,
        text: &BytesText<'_>,
    ) -> Result<Option<TeiEvent>, TeiError> {
        match &mut self.state {
            ParserState::InHeader { buffer, .. } => {
                // Keep text escaped for later reparsing by quick_xml::de
                buffer.extend_from_slice(text.as_ref());
                Ok(None)
            }
            ParserState::InParagraph { content, .. }
            | ParserState::InUtterance { content, .. }
            | ParserState::InEmphasis { content, .. } => {
                let unescaped = text.decode().map_err(|e| TeiError::xml(e.to_string()))?;
                if !unescaped.is_empty() {
                    content.push(Inline::Text(unescaped.into_owned()));
                }
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
                if name_bytes == b"pause" =>
            {
                let dur = extract_attribute(element, b"dur")?;
                let pause_type = extract_attribute(element, b"type")?;
                let pause = build_pause(dur, pause_type);
                content.push(Inline::Pause(pause));
                Ok(None)
            }
            ParserState::AwaitingBody if name_bytes == b"body" => {
                // Empty body, move to document end
                self.state = ParserState::DocumentComplete;
                Ok(Some(TeiEvent::DocumentEnd))
            }
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
                // Reconstruct CDATA for reparsing
                buffer.extend_from_slice(b"<![CDATA[");
                buffer.extend_from_slice(cdata.as_ref());
                buffer.extend_from_slice(b"]]>");
                Ok(None)
            }
            ParserState::InParagraph { content, .. }
            | ParserState::InUtterance { content, .. }
            | ParserState::InEmphasis { content, .. } => {
                // CDATA content is already unescaped, convert to string with strict UTF-8
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
                // Allow EOF after body content if document wasn't properly closed
                self.state = ParserState::DocumentComplete;
                Ok(Some(TeiEvent::DocumentEnd))
            }
            _ => {
                self.state = ParserState::Error;
                Err(TeiError::xml("unexpected end of document"))
            }
        }
    }
}
