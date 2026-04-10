//! End-element and completion handlers for the TEI pull parser.

use std::io::BufRead;

use quick_xml::events::BytesEnd;

use tei_core::{BodyBlock, DivContent, Inline, TeiError};

use super::event::TeiEvent;
use super::helpers::{
    RawDivAttrs, RawItemAttrs, build_div, build_head, build_hi, build_item, build_label,
    build_list, build_paragraph, build_utterance,
};
use super::parser::TeiPullParser;
use super::state::ParserState;

/// End element handlers.
impl<R: BufRead> TeiPullParser<R> {
    /// Handles closing of header elements and emits Header event when complete.
    pub(super) fn handle_header_end(
        &mut self,
        element: &BytesEnd<'_>,
    ) -> Result<Option<TeiEvent>, TeiError> {
        if let ParserState::InHeader { depth, buffer } = &mut self.state {
            super::helpers::append_end_element(buffer, element);
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

    /// Routes a completed body-content value to either the enclosing `<div>`
    /// (if one is pending) or emits it as a top-level `BodyBlock` event.
    fn push_to_div_or_emit<V>(
        &mut self,
        value: V,
        wrap_div: fn(V) -> DivContent,
        wrap_body: fn(V) -> BodyBlock,
    ) -> Option<TeiEvent> {
        if let Some(mut parent_div) = self.pending_div_state.take()
            && let ParserState::InDiv {
                content: div_children,
                ..
            } = parent_div.as_mut()
        {
            div_children.push(wrap_div(value));
            self.state = *parent_div;
            return None;
        }
        self.state = ParserState::InBody;
        Some(TeiEvent::BodyBlock(wrap_body(value)))
    }

    /// Finishes parsing a paragraph and emits a `BodyBlock` event or pushes to div.
    pub(super) fn finish_paragraph(&mut self) -> Result<Option<TeiEvent>, TeiError> {
        if let ParserState::InParagraph { id, content } = &mut self.state {
            let paragraph = build_paragraph(id.take(), std::mem::take(content))?;
            return Ok(self.push_to_div_or_emit(
                paragraph,
                DivContent::Paragraph,
                BodyBlock::Paragraph,
            ));
        }
        Ok(None)
    }

    /// Finishes parsing an utterance and emits a `BodyBlock` event or pushes to div.
    pub(super) fn finish_utterance(&mut self) -> Result<Option<TeiEvent>, TeiError> {
        if let ParserState::InUtterance { attrs, content } = &mut self.state {
            let utterance = build_utterance(std::mem::take(attrs), std::mem::take(content))?;
            return Ok(self.push_to_div_or_emit(
                utterance,
                DivContent::Utterance,
                BodyBlock::Utterance,
            ));
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

    /// Finishes parsing a head and stores it in the parent div.
    pub(super) fn finish_head(&mut self) -> Result<Option<TeiEvent>, TeiError> {
        if let ParserState::InHead {
            parent_div,
            content,
        } = &mut self.state
        {
            let head = build_head(std::mem::take(content))?;
            let Some(mut parent_state) = parent_div.take() else {
                return Err(TeiError::xml("internal error: InHead parent was None"));
            };
            if let ParserState::InDiv { head: div_head, .. } = parent_state.as_mut() {
                *div_head = Some(head);
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
                RawItemAttrs {
                    id: item_id.take(),
                    n: item_n.take(),
                    corresp: item_corresp.take(),
                    label: label.take(),
                },
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
            subtype,
            id,
            head,
            parent_div,
            content,
        } = &mut self.state
        {
            let div = build_div(
                RawDivAttrs {
                    div_type: std::mem::take(div_type),
                    subtype: subtype.take(),
                    id: id.take(),
                    head: head.take(),
                },
                std::mem::take(content),
            )?;
            if let Some(parent_state) = parent_div.take() {
                self.state = push_nested_div(parent_state, div)?;
                return Ok(None);
            }
            self.state = ParserState::InBody;
            return Ok(Some(TeiEvent::BodyBlock(BodyBlock::Div(div))));
        }
        Ok(None)
    }

    /// Handles body end elements.
    pub(super) fn handle_body_end(&mut self, name_bytes: &[u8]) -> Option<TeiEvent> {
        match name_bytes {
            b"body" => {
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

fn push_nested_div(
    parent_state: Box<ParserState>,
    div: tei_core::Div,
) -> Result<ParserState, TeiError> {
    let mut state = parent_state;
    let ParserState::InDiv {
        content: parent_content,
        ..
    } = state.as_mut()
    else {
        return Err(TeiError::xml("internal error: nested div parent was not InDiv"));
    };
    parent_content.push(DivContent::Div(div));
    Ok(*state)
}
