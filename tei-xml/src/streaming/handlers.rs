//! Start-element handlers for the TEI pull parser.

use std::io::BufRead;

use quick_xml::events::BytesStart;

use tei_core::{Inline, TeiError};

use super::event::TeiEvent;
use super::helpers::{
    append_start_element, extract_attribute, extract_div_attrs, extract_item_attrs,
    extract_utterance_attrs, extract_xml_id,
};
use super::parser::TeiPullParser;
use super::state::ParserState;

/// Classifies raw start-element tag names used by streaming handlers.
enum StartTagKind {
    Head,
    Paragraph,
    Utterance,
    List,
    Div,
    Other,
}

impl StartTagKind {
    const fn from_tag(name_bytes: &[u8]) -> Self {
        match name_bytes {
            b"head" => Self::Head,
            b"p" => Self::Paragraph,
            b"u" => Self::Utterance,
            b"list" => Self::List,
            b"div" => Self::Div,
            _ => Self::Other,
        }
    }
}

/// Classifies a tag name encountered directly inside `<body>`.
enum BodyChildKind {
    Paragraph,
    Utterance,
    Div,
    Other,
}

impl BodyChildKind {
    const fn from_tag(name_bytes: &[u8]) -> Self {
        match StartTagKind::from_tag(name_bytes) {
            StartTagKind::Paragraph => Self::Paragraph,
            StartTagKind::Utterance => Self::Utterance,
            StartTagKind::Div => Self::Div,
            StartTagKind::Head | StartTagKind::List | StartTagKind::Other => Self::Other,
        }
    }
}

/// Classifies a tag name encountered inside a `<div>`.
enum DivChildKind {
    Head,
    Paragraph,
    Utterance,
    List,
    NestedDiv,
    Other,
}

impl DivChildKind {
    const fn from_tag(name_bytes: &[u8]) -> Self {
        match StartTagKind::from_tag(name_bytes) {
            StartTagKind::Head => Self::Head,
            StartTagKind::Paragraph => Self::Paragraph,
            StartTagKind::Utterance => Self::Utterance,
            StartTagKind::List => Self::List,
            StartTagKind::Div => Self::NestedDiv,
            StartTagKind::Other => Self::Other,
        }
    }
}

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
        element: &BytesStart<'_>,
    ) -> Result<Option<TeiEvent>, TeiError> {
        let name = element.local_name();
        let name_bytes: &[u8] = name.as_ref();

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
        element: &BytesStart<'_>,
    ) -> Result<Option<TeiEvent>, TeiError> {
        let name = element.local_name();
        let name_bytes: &[u8] = name.as_ref();

        match BodyChildKind::from_tag(name_bytes) {
            BodyChildKind::Paragraph => self.enter_body_paragraph(element)?,
            BodyChildKind::Utterance => self.enter_body_utterance(element)?,
            BodyChildKind::Div => self.enter_body_div(element)?,
            BodyChildKind::Other => {}
        }
        Ok(None)
    }

    /// Enters a paragraph that is a direct child of the body.
    fn enter_body_paragraph(&mut self, element: &BytesStart<'_>) -> Result<(), TeiError> {
        let id = extract_xml_id(element)?;
        self.state = ParserState::in_paragraph(id);
        Ok(())
    }

    /// Enters an utterance that is a direct child of the body.
    fn enter_body_utterance(&mut self, element: &BytesStart<'_>) -> Result<(), TeiError> {
        let attrs = extract_utterance_attrs(element)?;
        self.state = ParserState::in_utterance(attrs);
        Ok(())
    }

    /// Enters a division that is a direct child of the body.
    fn enter_body_div(&mut self, element: &BytesStart<'_>) -> Result<(), TeiError> {
        let attrs = extract_div_attrs(element, None)?;
        self.state = ParserState::in_div(attrs.div_type, attrs.subtype, attrs.id);
        Ok(())
    }

    /// Handles start elements for inline content (emphasis).
    pub(super) fn handle_block_content_start(
        &mut self,
        element: &BytesStart<'_>,
    ) -> Result<Option<TeiEvent>, TeiError> {
        let name = element.local_name();
        let name_bytes: &[u8] = name.as_ref();

        if name_bytes == b"hi" {
            let rend = extract_attribute(element, b"rend")?;
            self.state.transition_to_emphasis(rend);
        }
        Ok(None)
    }

    /// Handles start elements within a `<div>`.
    pub(super) fn handle_div_content_start(
        &mut self,
        element: &BytesStart<'_>,
    ) -> Result<Option<TeiEvent>, TeiError> {
        let name = element.local_name();
        let name_bytes: &[u8] = name.as_ref();

        match DivChildKind::from_tag(name_bytes) {
            DivChildKind::Head => self.enter_div_head()?,
            DivChildKind::Paragraph => self.enter_div_paragraph(element)?,
            DivChildKind::Utterance => self.enter_div_utterance(element)?,
            DivChildKind::List => self.enter_div_list(element)?,
            DivChildKind::NestedDiv => self.enter_nested_div(element)?,
            DivChildKind::Other => {}
        }
        Ok(None)
    }

    /// Enters the head of the current division, which must be its first child.
    fn enter_div_head(&mut self) -> Result<(), TeiError> {
        self.validate_head_placement()?;
        let current_state = std::mem::take(&mut self.state);
        self.state = ParserState::in_head(current_state);
        Ok(())
    }

    /// Enters a paragraph inside the current division, parking the division state.
    fn enter_div_paragraph(&mut self, element: &BytesStart<'_>) -> Result<(), TeiError> {
        let id = extract_xml_id(element)?;
        let current_state = std::mem::take(&mut self.state);
        self.state = ParserState::in_paragraph(id);
        self.pending_div_state = Some(Box::new(current_state));
        Ok(())
    }

    /// Enters an utterance inside the current division, parking the division state.
    fn enter_div_utterance(&mut self, element: &BytesStart<'_>) -> Result<(), TeiError> {
        let attrs = extract_utterance_attrs(element)?;
        let current_state = std::mem::take(&mut self.state);
        self.state = ParserState::in_utterance(attrs);
        self.pending_div_state = Some(Box::new(current_state));
        Ok(())
    }

    /// Enters a list inside the current division.
    fn enter_div_list(&mut self, element: &BytesStart<'_>) -> Result<(), TeiError> {
        let list_id = extract_xml_id(element)?;
        let current_state = std::mem::take(&mut self.state);
        self.state = ParserState::in_list(current_state, list_id);
        Ok(())
    }

    /// Enters a division nested inside the current division.
    fn enter_nested_div(&mut self, element: &BytesStart<'_>) -> Result<(), TeiError> {
        let attrs = extract_div_attrs(element, None)?;
        let current_state = std::mem::take(&mut self.state);
        self.state =
            ParserState::nested_div(current_state, attrs.div_type, attrs.subtype, attrs.id);
        Ok(())
    }

    /// Validates that a head appears at most once and before any div content.
    fn validate_head_placement(&self) -> Result<(), TeiError> {
        let ParserState::InDiv { head, content, .. } = &self.state else {
            return Ok(());
        };
        if head.is_some() {
            return Err(TeiError::xml("div may only contain one head"));
        }
        if !content.is_empty() {
            return Err(TeiError::xml("head must appear before div content"));
        }
        Ok(())
    }

    /// Handles start elements within a `<list>` (items).
    pub(super) fn handle_list_content_start(
        &mut self,
        element: &BytesStart<'_>,
    ) -> Result<Option<TeiEvent>, TeiError> {
        let name = element.local_name();
        let name_bytes: &[u8] = name.as_ref();

        if name_bytes == b"item" {
            let attrs = extract_item_attrs(element, None)?;
            let current_state = std::mem::take(&mut self.state);
            self.state = ParserState::in_item(current_state, attrs.id, attrs.n, attrs.corresp);
        }
        Ok(None)
    }

    /// Handles start elements within an `<item>` (label, inline content).
    pub(super) fn handle_item_content_start(
        &mut self,
        element: &BytesStart<'_>,
    ) -> Result<Option<TeiEvent>, TeiError> {
        let name = element.local_name();
        let name_bytes: &[u8] = name.as_ref();

        if name_bytes == b"label" {
            self.validate_label_placement()?;
            let current_state = std::mem::take(&mut self.state);
            self.state = ParserState::in_label(current_state);
        } else if name_bytes == b"hi" {
            let rend = extract_attribute(element, b"rend")?;
            self.state.transition_to_emphasis(rend);
        }
        Ok(None)
    }

    /// Validates that a label appears at most once and before any item content.
    fn validate_label_placement(&self) -> Result<(), TeiError> {
        let ParserState::InItem { label, content, .. } = &self.state else {
            return Ok(());
        };
        if label.is_some() {
            return Err(TeiError::xml("item may only contain one label"));
        }
        if content
            .iter()
            .any(|inline| !matches!(inline, Inline::Text(text) if text.trim().is_empty()))
        {
            return Err(TeiError::xml("label must appear before item content"));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for start-element handlers.

    use std::io::Cursor;

    use proptest::prelude::*;
    use quick_xml::events::BytesStart;

    use super::*;
    use crate::streaming::state::ParserState;

    /// Returns a `<u>` element whose `who` attribute contains a raw unknown
    /// entity reference. `BytesStart::from_content` is used so that the
    /// unescaped `&badentity;` bytes are preserved and reach
    /// `normalized_value()`, triggering a parse error.
    fn utterance_with_bad_entity() -> BytesStart<'static> {
        BytesStart::from_content(r#"u who="&badentity;""#, 1)
    }

    /// Attribute extraction must fail before the parser state is taken.
    #[test]
    fn utterance_extraction_failure_preserves_div_state() {
        let mut parser = TeiPullParser::new(Cursor::new(""));
        parser.state = ParserState::in_div("section".into(), None, None);

        let element = utterance_with_bad_entity();
        let result = parser.handle_div_content_start(&element);

        assert!(result.is_err(), "expected an error from the unknown entity");
        assert!(
            matches!(parser.state, ParserState::InDiv { .. }),
            "InDiv state must be preserved after attribute extraction failure; got: {:?}",
            parser.state,
        );
    }

    proptest! {
        #[test]
        fn utterance_extraction_failure_preserves_arbitrary_div_state(
            div_type in "[A-Za-z][A-Za-z0-9_-]{0,24}",
            subtype in proptest::option::of("[A-Za-z][A-Za-z0-9_-]{0,24}"),
            id in proptest::option::of("[A-Za-z_][A-Za-z0-9_-]{0,24}"),
        ) {
            let mut parser = TeiPullParser::new(Cursor::new(""));
            let expected_state = ParserState::in_div(div_type, subtype, id);
            parser.state = expected_state.clone();

            let element = utterance_with_bad_entity();
            let result = parser.handle_div_content_start(&element);

            prop_assert!(result.is_err(), "unknown entity extraction should fail");
            prop_assert_eq!(parser.state, expected_state);
            prop_assert!(parser.pending_div_state.is_none());
        }
    }
}
