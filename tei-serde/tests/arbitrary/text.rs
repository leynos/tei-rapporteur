//! Proptest strategies for body content: `P`, `Utterance`, `BodyBlock`, `TeiBody`, `TeiText`.
//!
//! Generates valid body structures with paragraphs and utterances containing
//! visible inline content.
//!
//! This module provides two sets of strategies:
//!
//! - **Full strategies** (e.g., `paragraph_strategy`, `utterance_strategy`) generate
//!   complete structures with all inline node types (`Text`, `Hi`, `Pause`). Use these
//!   for JSON and `MessagePack` round-trip tests where full content fidelity is preserved.
//!
//! - **Text-only strategies** (e.g., `text_only_paragraph_strategy`) generate structures
//!   containing only plain text nodes. Use these for XML round-trip tests because
//!   `quick-xml`'s serde integration cannot serialize `Hi` and `Pause` structs in
//!   `$value` fields. They also avoid adjacent text node merging issues that occur
//!   during XML parsing.
//!
//! Choose the strategy variant that matches the serializer being exercised in your test.

use proptest::prelude::*;
use tei_core::{
    BodyBlock, Div, DivContent, Inline, Item, Label, List, P, PointerList, TeiBody, TeiText,
    Utterance,
};

use super::inline::{
    has_visible_content, has_visible_content_slice, inline_strategy, text_only_inline_strategy,
};
use super::primitives::{speaker_strategy, xml_id_strategy};

/// Helper to generate a paragraph from any content strategy.
fn paragraph_with_content_strategy<S>(content_strategy: S) -> impl Strategy<Value = P>
where
    S: Strategy<Value = Vec<Inline>>,
{
    (proptest::option::of(xml_id_strategy()), content_strategy).prop_map(|(id, content)| {
        let mut p = P::from_inline(content)
            .unwrap_or_else(|error| panic!("generated content should be valid: {error}"));
        if let Some(id_value) = id {
            p.set_id(id_value)
                .unwrap_or_else(|error| panic!("generated id should be valid: {error}"));
        }
        p
    })
}

/// Helper to generate an utterance from any content strategy.
fn utterance_with_content_strategy<S>(content_strategy: S) -> impl Strategy<Value = Utterance>
where
    S: Strategy<Value = Vec<Inline>>,
{
    (
        proptest::option::of(xml_id_strategy()),
        proptest::option::of(speaker_strategy()),
        content_strategy,
    )
        .prop_map(|(id, speaker, content)| {
            let mut u = Utterance::from_inline(speaker.as_deref(), content)
                .unwrap_or_else(|error| panic!("generated content should be valid: {error}"));
            if let Some(id_value) = id {
                u.set_id(id_value)
                    .unwrap_or_else(|error| panic!("generated id should be valid: {error}"));
            }
            u
        })
}

/// Generates a paragraph with optional `xml:id`.
pub fn paragraph_strategy() -> impl Strategy<Value = P> {
    paragraph_with_content_strategy(
        prop::collection::vec(inline_strategy(), 1..=5)
            .prop_filter("must have visible content", |v| has_visible_content(v)),
    )
}

/// Generates an utterance with optional speaker and `xml:id`.
pub fn utterance_strategy() -> impl Strategy<Value = Utterance> {
    utterance_with_content_strategy(
        prop::collection::vec(inline_strategy(), 1..=5)
            .prop_filter("must have visible content", |v| has_visible_content(v)),
    )
}

/// Generates a `BodyBlock` (either Paragraph or Utterance).
pub fn body_block_strategy() -> impl Strategy<Value = BodyBlock> {
    prop_oneof![
        paragraph_strategy().prop_map(BodyBlock::Paragraph),
        utterance_strategy().prop_map(BodyBlock::Utterance),
        div_strategy().prop_map(BodyBlock::Div),
    ]
}

/// Generates a `TeiBody` with 0-10 blocks.
pub fn tei_body_strategy() -> impl Strategy<Value = TeiBody> {
    prop::collection::vec(body_block_strategy(), 0..=10).prop_map(TeiBody::new)
}

/// Generates a `TeiText` wrapping a `TeiBody`.
pub fn tei_text_strategy() -> impl Strategy<Value = TeiText> {
    tei_body_strategy().prop_map(TeiText::new)
}

/// Generates a text-only paragraph (no Hi or Pause elements).
///
/// This is needed for XML round-trip testing because `quick-xml`'s serde
/// integration does not support serializing complex inline structures.
/// Uses a single text node to avoid adjacent text node merging during XML round-trip.
pub fn text_only_paragraph_strategy() -> impl Strategy<Value = P> {
    // Use a single text node to avoid adjacent text merging in XML
    paragraph_with_content_strategy(text_only_inline_strategy().prop_map(|inline| vec![inline]))
}

/// Generates a text-only utterance (no Hi or Pause elements).
///
/// This is needed for XML round-trip testing because `quick-xml`'s serde
/// integration does not support serializing complex inline structures.
/// Uses a single text node to avoid adjacent text node merging during XML round-trip.
pub fn text_only_utterance_strategy() -> impl Strategy<Value = Utterance> {
    // Use a single text node to avoid adjacent text merging in XML
    utterance_with_content_strategy(text_only_inline_strategy().prop_map(|inline| vec![inline]))
}

/// Generates a text-only `BodyBlock` (no Hi or Pause elements).
pub fn text_only_body_block_strategy() -> impl Strategy<Value = BodyBlock> {
    prop_oneof![
        text_only_paragraph_strategy().prop_map(BodyBlock::Paragraph),
        text_only_utterance_strategy().prop_map(BodyBlock::Utterance),
        text_only_div_strategy().prop_map(BodyBlock::Div),
    ]
}

/// Generates a text-only `TeiBody` with 0-10 blocks.
pub fn text_only_tei_body_strategy() -> impl Strategy<Value = TeiBody> {
    prop::collection::vec(text_only_body_block_strategy(), 0..=10).prop_map(TeiBody::new)
}

/// Generates a text-only `TeiText` wrapping a `TeiBody`.
pub fn text_only_tei_text_strategy() -> impl Strategy<Value = TeiText> {
    text_only_tei_body_strategy().prop_map(TeiText::new)
}

fn label_with_content_strategy<S>(content_strategy: S) -> impl Strategy<Value = Label>
where
    S: Strategy<Value = Vec<Inline>>,
{
    content_strategy.prop_map(|content| {
        Label::new(content)
            .unwrap_or_else(|error| panic!("generated label content should be valid: {error}"))
    })
}

fn item_with_content_strategy<S, L>(
    content_strategy: S,
    label_content_strategy: L,
) -> impl Strategy<Value = Item>
where
    S: Strategy<Value = Vec<Inline>>,
    L: Strategy<Value = Vec<Inline>>,
{
    (
        proptest::option::of(xml_id_strategy()),
        proptest::option::of(prop::sample::select(vec![
            String::from("1"),
            String::from("2"),
            String::from("intro"),
            String::from("link"),
        ])),
        proptest::option::of(prop::collection::vec(xml_id_strategy(), 1..=3)),
        proptest::option::of(label_with_content_strategy(label_content_strategy)),
        content_strategy,
    )
        .prop_map(|(id, n, corresp_ids, label, content)| {
            let mut item = Item::new(content)
                .unwrap_or_else(|error| panic!("generated item content should be valid: {error}"));
            if let Some(id_value) = id {
                item.set_id(id_value)
                    .unwrap_or_else(|error| panic!("generated id should be valid: {error}"));
            }
            if let Some(n_value) = n {
                item.set_n(n_value)
                    .unwrap_or_else(|error| panic!("generated @n should be valid: {error}"));
            }
            if let Some(corresp_values) = corresp_ids {
                let corresp = PointerList::new(
                    corresp_values
                        .into_iter()
                        .map(|pointer_id| format!("#{pointer_id}")),
                )
                .unwrap_or_else(|error| panic!("generated @corresp should be valid: {error}"));
                item.set_corresp(corresp);
            }
            if let Some(label_value) = label {
                item.set_label(label_value);
            }
            item
        })
}

fn list_with_item_strategy<S>(item_strategy: S) -> impl Strategy<Value = List>
where
    S: Strategy<Value = Item>,
{
    (
        proptest::option::of(xml_id_strategy()),
        prop::collection::vec(item_strategy, 1..=4),
    )
        .prop_map(|(id, items)| {
            let mut list = List::new(items);
            if let Some(id_value) = id {
                list.set_id(id_value)
                    .unwrap_or_else(|error| panic!("generated id should be valid: {error}"));
            }
            list
        })
}

fn div_type_strategy() -> impl Strategy<Value = String> {
    prop::sample::select(vec![
        String::from("chapter"),
        String::from("intro"),
        String::from("show-notes"),
        String::from("links"),
        String::from("sponsors"),
    ])
}

fn div_with_content_strategy<S>(content_strategy: S) -> impl Strategy<Value = Div>
where
    S: Strategy<Value = Vec<DivContent>>,
{
    (
        proptest::option::of(xml_id_strategy()),
        div_type_strategy(),
        content_strategy,
    )
        .prop_map(|(id, div_type, content)| {
            let mut div = Div::new(div_type)
                .unwrap_or_else(|error| panic!("generated div type should be valid: {error}"));
            if let Some(id_value) = id {
                div.set_id(id_value)
                    .unwrap_or_else(|error| panic!("generated id should be valid: {error}"));
            }
            for child in content {
                match child {
                    DivContent::Paragraph(paragraph_block) => div.push_paragraph(paragraph_block),
                    DivContent::Utterance(utterance_block) => div.push_utterance(utterance_block),
                    DivContent::List(list_block) => div.push_list(list_block),
                }
            }
            div
        })
}

fn div_strategy() -> impl Strategy<Value = Div> {
    let paragraph_content = paragraph_strategy().prop_map(DivContent::Paragraph);
    let utterance_content = utterance_strategy().prop_map(DivContent::Utterance);
    let list_content = list_with_item_strategy(item_with_content_strategy(
        prop::collection::vec(inline_strategy(), 1..=5)
            .prop_filter("item must have visible content", |v| has_visible_content(v)),
        prop::collection::vec(inline_strategy(), 1..=3)
            .prop_filter("label must have visible content", |v| {
                has_visible_content(v)
            }),
    ))
    .prop_map(DivContent::List);

    div_with_content_strategy(prop::collection::vec(
        prop_oneof![paragraph_content, utterance_content, list_content],
        0..=4,
    ))
}

fn text_only_div_strategy() -> impl Strategy<Value = Div> {
    let paragraph_content = text_only_paragraph_strategy().prop_map(DivContent::Paragraph);
    let utterance_content = text_only_utterance_strategy().prop_map(DivContent::Utterance);
    let list_content = list_with_item_strategy(item_with_content_strategy(
        text_only_inline_strategy().prop_map(|inline| vec![inline]),
        text_only_inline_strategy().prop_map(|inline| vec![inline]),
    ))
    .prop_map(DivContent::List);

    div_with_content_strategy(prop::collection::vec(
        prop_oneof![paragraph_content, utterance_content, list_content],
        0..=4,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arbitrary::test_utils::assert_strategy_produces_valid_values;

    fn label_is_valid(label: &Label) -> bool {
        has_visible_content_slice(label.content())
    }

    fn item_is_valid(item: &Item) -> bool {
        has_visible_content_slice(item.content()) && item.label().is_none_or(label_is_valid)
    }

    fn list_is_valid(list: &List) -> bool {
        !list.items().is_empty() && list.items().iter().all(item_is_valid)
    }

    fn div_is_valid(div: &Div) -> bool {
        !div.div_type().trim().is_empty()
            && div.content().iter().all(|content| match content {
                DivContent::Paragraph(paragraph) => has_visible_content_slice(paragraph.content()),
                DivContent::Utterance(utterance) => has_visible_content_slice(utterance.content()),
                DivContent::List(list) => list_is_valid(list),
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
}
