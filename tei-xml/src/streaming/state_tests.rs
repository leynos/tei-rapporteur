//! Tests for streaming parser state transitions.
use super::*;

/// Test-only helper trait for `ParserState` predicates.
trait ParserStateTestExt {
    fn is_initial(&self) -> bool;
    fn is_complete(&self) -> bool;
    fn is_error(&self) -> bool;
    fn is_in_body(&self) -> bool;
    fn is_in_block(&self) -> bool;
    fn take_content(&mut self) -> Vec<Inline>;
}

impl ParserStateTestExt for ParserState {
    fn is_initial(&self) -> bool {
        matches!(self, Self::Initial)
    }
    fn is_complete(&self) -> bool {
        matches!(self, Self::DocumentComplete)
    }
    fn is_error(&self) -> bool {
        matches!(self, Self::Error)
    }
    fn is_in_body(&self) -> bool {
        matches!(self, Self::InBody)
    }
    fn is_in_block(&self) -> bool {
        matches!(
            self,
            Self::InParagraph { .. }
                | Self::InUtterance { .. }
                | Self::InEmphasis { .. }
                | Self::InHead { .. }
                | Self::InItem { .. }
                | Self::InLabel { .. }
        )
    }
    fn take_content(&mut self) -> Vec<Inline> {
        self.content_mut().map(std::mem::take).unwrap_or_default()
    }
}

#[test]
fn default_state_is_initial() {
    assert_eq!(ParserState::default(), ParserState::Initial);
    assert!(ParserState::Initial.is_initial());
}

#[test]
fn state_predicates() {
    assert!(ParserState::DocumentComplete.is_complete());
    assert!(ParserState::Error.is_error());
    assert!(ParserState::InBody.is_in_body());
}

#[test]
fn in_block_detection() {
    let paragraph = ParserState::in_paragraph(None);
    assert!(paragraph.is_in_block());

    let utterance = ParserState::in_utterance(RawUtteranceAttrs {
        id: Some("u1".into()),
        who: Some("speaker".into()),
        ..RawUtteranceAttrs::default()
    });
    assert!(utterance.is_in_block());

    let emphasis = ParserState::in_emphasis(ParserState::InBody, Some("italic".into()));
    assert!(emphasis.is_in_block());

    let item = ParserState::in_item(
        ParserState::in_list(ParserState::in_div("section".into(), None, None), None),
        Some("item1".into()),
        None,
        None,
    );
    assert!(item.is_in_block());

    let label = ParserState::in_label(item);
    assert!(label.is_in_block());

    assert!(!ParserState::InBody.is_in_block());
}

#[test]
fn push_and_take_inline_content() {
    let mut state = ParserState::in_paragraph(Some("p1".into()));
    state.push_inline(Inline::Text("Hello".into()));
    state.push_inline(Inline::Text(" World".into()));

    let content = state.take_content();
    assert_eq!(content.len(), 2);
}

#[test]
fn push_inline_in_item_state() {
    let parent = ParserState::in_list(ParserState::in_div("section".into(), None, None), None);
    let mut item_state = ParserState::in_item(parent, Some("i1".into()), Some("1".into()), None);
    item_state.push_inline(Inline::Text("Item content".into()));

    let content = item_state.take_content();
    assert_eq!(content, vec![Inline::Text("Item content".into())]);
}

#[test]
fn push_inline_in_label_state() {
    let parent_item = ParserState::in_item(
        ParserState::in_list(ParserState::in_div("section".into(), None, None), None),
        None,
        None,
        None,
    );
    let mut label_state = ParserState::in_label(parent_item);
    label_state.push_inline(Inline::Text("Label:".into()));

    let content = label_state.take_content();
    assert_eq!(content, vec![Inline::Text("Label:".into())]);
}

#[test]
fn push_inline_in_head_state() {
    let parent_div = ParserState::in_div("section".into(), None, None);
    let mut head_state = ParserState::in_head(parent_div);
    head_state.push_inline(Inline::Text("Heading".into()));

    let content = head_state.take_content();
    assert_eq!(content, vec![Inline::Text("Heading".into())]);
}

#[test]
fn header_buffer_accumulation() {
    let state = ParserState::in_header(1);
    match state {
        ParserState::InHeader { depth, buffer } => {
            assert_eq!(depth, 1);
            assert!(buffer.is_empty());
        }
        _ => panic!("expected InHeader state"),
    }
}
