//! Tests for text and body strategies.
use super::*;
use crate::arbitrary::test_utils::assert_strategy_produces_valid_values;

fn label_is_valid(label: &Label) -> bool {
    has_visible_content_slice(label.content())
}

fn head_is_valid(head: &Head) -> bool {
    has_visible_content_slice(head.content())
}

fn item_is_valid(item: &Item) -> bool {
    has_visible_content_slice(item.content()) && item.label().is_none_or(label_is_valid)
}

fn list_is_valid(list: &List) -> bool {
    !list.items().is_empty() && list.items().iter().all(item_is_valid)
}

fn div_is_valid(div: &Div) -> bool {
    !div.div_type().trim().is_empty()
        && div
            .subtype()
            .is_none_or(|subtype| !subtype.trim().is_empty())
        && div.head().is_none_or(head_is_valid)
        && div.content().iter().all(|content| match content {
            DivContent::Paragraph(paragraph) => has_visible_content_slice(paragraph.content()),
            DivContent::Utterance(utterance) => has_visible_content_slice(utterance.content()),
            DivContent::List(list) => list_is_valid(list),
            DivContent::Div(nested_div) => div_is_valid(nested_div),
        })
}

#[test]
fn paragraph_strategy_produces_valid_paragraphs() {
    assert_strategy_produces_valid_values(paragraph_strategy(), |paragraph| {
        has_visible_content_slice(paragraph.content())
    });
}

#[test]
fn utterance_strategy_produces_valid_utterances() {
    assert_strategy_produces_valid_values(utterance_strategy(), |utterance| {
        has_visible_content_slice(utterance.content())
    });
}

#[test]
fn tei_body_strategy_produces_valid_bodies() {
    assert_strategy_produces_valid_values(tei_body_strategy(), |body| {
        // Bodies can be empty, but any blocks must be valid
        body.blocks().iter().all(|block| match block {
            BodyBlock::Paragraph(p) => has_visible_content_slice(p.content()),
            BodyBlock::Utterance(u) => has_visible_content_slice(u.content()),
            BodyBlock::Div(div) => div_is_valid(div),
        })
    });
}
