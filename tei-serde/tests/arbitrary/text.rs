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
use tei_core::{BodyBlock, Inline, P, TeiBody, TeiText, Utterance};

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arbitrary::test_utils::assert_strategy_produces_valid_values;

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
                BodyBlock::Div(_) => true, // Divs don't need content validation here
            })
        });
    }
}
