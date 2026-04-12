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
    ) -> Result<Option<TeiEvent>, TeiError> {
        if let Some(mut parent_div) = self.pending_div_state.take() {
            match parent_div.as_mut() {
                ParserState::InDiv {
                    content: div_children,
                    ..
                } => {
                    div_children.push(wrap_div(value));
                    self.state = *parent_div;
                    Ok(None)
                }
                _ => Err(TeiError::xml(
                    "internal error: pending div parent was not InDiv",
                )),
            }
        } else {
            self.state = ParserState::InBody;
            Ok(Some(TeiEvent::BodyBlock(wrap_body(value))))
        }
    }

    fn finish_div_or_body_block<V>(
        &mut self,
        extract: impl FnOnce(&mut ParserState) -> Option<Result<V, TeiError>>,
        wrap_div: fn(V) -> DivContent,
        wrap_body: fn(V) -> BodyBlock,
    ) -> Result<Option<TeiEvent>, TeiError> {
        if let Some(result) = extract(&mut self.state) {
            return self.push_to_div_or_emit(result?, wrap_div, wrap_body);
        }
        Ok(None)
    }

    fn finish_with_parent_restore<V>(
        &mut self,
        extract: impl FnOnce(&mut ParserState) -> Result<Option<(V, Box<ParserState>)>, TeiError>,
        apply: impl FnOnce(V, &mut ParserState),
    ) -> Result<Option<TeiEvent>, TeiError> {
        let Some((value, mut parent_state)) = extract(&mut self.state)? else {
            return Ok(None);
        };

        apply(value, parent_state.as_mut());
        self.state = *parent_state;
        Ok(None)
    }

    /// Finishes parsing a paragraph and emits a `BodyBlock` event or pushes to div.
    pub(super) fn finish_paragraph(&mut self) -> Result<Option<TeiEvent>, TeiError> {
        self.finish_div_or_body_block(
            |state| {
                let ParserState::InParagraph { id, content } = state else {
                    return None;
                };
                Some(build_paragraph(id.take(), std::mem::take(content)))
            },
            DivContent::Paragraph,
            BodyBlock::Paragraph,
        )
    }

    /// Finishes parsing an utterance and emits a `BodyBlock` event or pushes to div.
    pub(super) fn finish_utterance(&mut self) -> Result<Option<TeiEvent>, TeiError> {
        self.finish_div_or_body_block(
            |state| {
                let ParserState::InUtterance { attrs, content } = state else {
                    return None;
                };
                Some(build_utterance(
                    std::mem::take(attrs),
                    std::mem::take(content),
                ))
            },
            DivContent::Utterance,
            BodyBlock::Utterance,
        )
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
        self.finish_with_parent_restore(
            |state| {
                let ParserState::InLabel {
                    parent_item,
                    content,
                } = state
                else {
                    return Ok(None);
                };
                let label = build_label(std::mem::take(content))?;
                let parent = parent_item
                    .take()
                    .ok_or_else(|| TeiError::xml("internal error: InLabel parent was None"))?;

                Ok(Some((label, parent)))
            },
            |label, parent_state| {
                if let ParserState::InItem {
                    label: item_label, ..
                } = parent_state
                {
                    *item_label = Some(label);
                }
            },
        )
    }

    /// Finishes parsing a head and stores it in the parent div.
    pub(super) fn finish_head(&mut self) -> Result<Option<TeiEvent>, TeiError> {
        self.finish_with_parent_restore(
            |state| {
                let ParserState::InHead {
                    parent_div,
                    content,
                } = state
                else {
                    return Ok(None);
                };
                let head = build_head(std::mem::take(content))?;
                let parent = parent_div
                    .take()
                    .ok_or_else(|| TeiError::xml("internal error: InHead parent was None"))?;

                Ok(Some((head, parent)))
            },
            |head, parent_state| {
                if let ParserState::InDiv { head: div_head, .. } = parent_state {
                    *div_head = Some(head);
                }
            },
        )
    }

    /// Finishes parsing an item and pushes it to the parent list.
    pub(super) fn finish_item(&mut self) -> Result<Option<TeiEvent>, TeiError> {
        self.finish_with_parent_restore(
            |state| {
                let ParserState::InItem {
                    parent_list,
                    item_id,
                    item_n,
                    item_corresp,
                    label,
                    content,
                } = state
                else {
                    return Ok(None);
                };
                let item = build_item(
                    RawItemAttrs {
                        id: item_id.take(),
                        n: item_n.take(),
                        corresp: item_corresp.take(),
                        label: label.take(),
                    },
                    std::mem::take(content),
                )?;
                let parent = parent_list
                    .take()
                    .ok_or_else(|| TeiError::xml("internal error: InItem parent was None"))?;

                Ok(Some((item, parent)))
            },
            |item, parent_state| {
                if let ParserState::InList { items, .. } = parent_state {
                    items.push(item);
                }
            },
        )
    }

    /// Finishes parsing a list and pushes it to the parent div.
    pub(super) fn finish_list(&mut self) -> Result<Option<TeiEvent>, TeiError> {
        self.finish_with_parent_restore(
            |state| {
                let ParserState::InList {
                    parent_div,
                    list_id,
                    items,
                } = state
                else {
                    return Ok(None);
                };
                let list = build_list(list_id.take(), std::mem::take(items))?;
                let parent = parent_div
                    .take()
                    .ok_or_else(|| TeiError::xml("internal error: InList parent was None"))?;

                Ok(Some((list, parent)))
            },
            |list, parent_state| {
                if let ParserState::InDiv { content, .. } = parent_state {
                    content.push(DivContent::List(list));
                }
            },
        )
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
        return Err(TeiError::xml(
            "internal error: nested div parent was not InDiv",
        ));
    };
    parent_content.push(DivContent::Div(div));
    Ok(*state)
}
