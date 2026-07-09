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
    BodyBlock, Div, DivContent, Head, Inline, Item, Label, List, P, PointerList, TeiBody, TeiText,
    Utterance,
};

use super::ExpectValid;
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
        let mut p = P::from_inline(content).expect_valid("generated content should be valid");
        if let Some(id_value) = id {
            p.set_id(id_value)
                .expect_valid("generated id should be valid");
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
                .expect_valid("generated content should be valid");
            if let Some(id_value) = id {
                u.set_id(id_value)
                    .expect_valid("generated id should be valid");
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
        Label::new(content).expect_valid("generated label content should be valid")
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
            let mut item =
                Item::new(content).expect_valid("generated item content should be valid");
            if let Some(id_value) = id {
                item.set_id(id_value)
                    .expect_valid("generated id should be valid");
            }
            if let Some(n_value) = n {
                item.set_n(n_value)
                    .expect_valid("generated @n should be valid");
            }
            if let Some(corresp_values) = corresp_ids {
                let corresp = PointerList::new(
                    corresp_values
                        .into_iter()
                        .map(|pointer_id| format!("#{pointer_id}")),
                )
                .expect_valid("generated @corresp should be valid");
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
            let mut list = List::new(items).expect_valid("generated list items should be valid");
            if let Some(id_value) = id {
                list.set_id(id_value)
                    .expect_valid("generated id should be valid");
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

fn div_subtype_strategy() -> impl Strategy<Value = String> {
    prop::sample::select(vec![
        String::from("chapter-marker"),
        String::from("guest-bio"),
        String::from("sponsor-read"),
        String::from("segment"),
    ])
}

fn head_with_content_strategy<S>(content_strategy: S) -> impl Strategy<Value = Head>
where
    S: Strategy<Value = Vec<Inline>>,
{
    content_strategy.prop_map(|content| {
        Head::new(content).expect_valid("generated head content should be valid")
    })
}

fn div_with_content_strategy<S, H>(
    content_strategy: S,
    head_strategy: H,
) -> impl Strategy<Value = Div>
where
    S: Strategy<Value = Vec<DivContent>>,
    H: Strategy<Value = Head>,
{
    (
        proptest::option::of(xml_id_strategy()),
        div_type_strategy(),
        proptest::option::of(div_subtype_strategy()),
        proptest::option::of(head_strategy),
        content_strategy,
    )
        .prop_map(|(id, div_type, subtype, head, content)| {
            let mut div = Div::new(div_type).expect_valid("generated div type should be valid");
            if let Some(id_value) = id {
                div.set_id(id_value)
                    .expect_valid("generated id should be valid");
            }
            if let Some(subtype_value) = subtype {
                div.set_subtype(subtype_value)
                    .expect_valid("generated subtype should be valid");
            }
            if let Some(head_value) = head {
                div.set_head(head_value);
            }
            for child in content {
                match child {
                    DivContent::Paragraph(paragraph_block) => div.push_paragraph(paragraph_block),
                    DivContent::Utterance(utterance_block) => div.push_utterance(utterance_block),
                    DivContent::List(list_block) => div.push_list(list_block),
                    DivContent::Div(nested_div) => div.push_div(nested_div),
                }
            }
            div
        })
}

fn div_strategy() -> impl Strategy<Value = Div> {
    let head_strategy = head_with_content_strategy(
        prop::collection::vec(inline_strategy(), 1..=3)
            .prop_filter("head must have visible content", |v| has_visible_content(v)),
    );
    let leaf_content = prop_oneof![
        paragraph_strategy().prop_map(DivContent::Paragraph),
        utterance_strategy().prop_map(DivContent::Utterance),
        list_with_item_strategy(item_with_content_strategy(
            prop::collection::vec(inline_strategy(), 1..=5)
                .prop_filter("item must have visible content", |v| has_visible_content(v)),
            prop::collection::vec(inline_strategy(), 1..=3)
                .prop_filter("label must have visible content", |v| has_visible_content(
                    v
                )),
        ))
        .prop_map(DivContent::List),
    ];
    let recursive_content = leaf_content.prop_recursive(3, 32, 4, |inner| {
        div_with_content_strategy(
            prop::collection::vec(inner, 0..=3),
            head_with_content_strategy(
                prop::collection::vec(inline_strategy(), 1..=3)
                    .prop_filter("head must have visible content", |v| has_visible_content(v)),
            ),
        )
        .prop_map(DivContent::Div)
    });

    div_with_content_strategy(
        prop::collection::vec(recursive_content, 0..=4),
        head_strategy,
    )
}

fn text_only_div_strategy() -> impl Strategy<Value = Div> {
    let head_strategy =
        head_with_content_strategy(text_only_inline_strategy().prop_map(|inline| vec![inline]));
    let leaf_content = prop_oneof![
        text_only_paragraph_strategy().prop_map(DivContent::Paragraph),
        text_only_utterance_strategy().prop_map(DivContent::Utterance),
        list_with_item_strategy(item_with_content_strategy(
            text_only_inline_strategy().prop_map(|inline| vec![inline]),
            text_only_inline_strategy().prop_map(|inline| vec![inline]),
        ))
        .prop_map(DivContent::List),
    ];
    let recursive_content = leaf_content.prop_recursive(2, 24, 4, |inner| {
        div_with_content_strategy(
            prop::collection::vec(inner, 0..=3),
            head_with_content_strategy(text_only_inline_strategy().prop_map(|inline| vec![inline])),
        )
        .prop_map(DivContent::Div)
    });

    div_with_content_strategy(
        prop::collection::vec(recursive_content, 0..=4),
        head_strategy,
    )
}

#[cfg(test)]
#[path = "text_tests.rs"]
mod tests;
