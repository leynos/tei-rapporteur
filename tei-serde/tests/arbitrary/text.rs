//! Proptest strategies for body content: `P`, `Utterance`, `BodyBlock`, `TeiBody`, `TeiText`.
//!
//! Generates valid body structures with paragraphs and utterances containing
//! visible inline content.

use proptest::prelude::*;
use tei_core::{BodyBlock, P, TeiBody, TeiText, Utterance};

use super::inline::{
    has_visible_content, has_visible_content_slice, inline_strategy, text_only_inline_strategy,
};
use super::primitives::{speaker_strategy, xml_id_strategy};

/// Generates a paragraph with optional `xml:id`.
pub fn paragraph_strategy() -> impl Strategy<Value = P> {
    (
        proptest::option::of(xml_id_strategy()),
        prop::collection::vec(inline_strategy(), 1..=5)
            .prop_filter("must have visible content", |v| has_visible_content(v)),
    )
        .prop_map(|(id, content)| {
            let mut p = P::from_inline(content)
                .unwrap_or_else(|error| panic!("generated content should be valid: {error}"));
            if let Some(id_value) = id {
                p.set_id(id_value)
                    .unwrap_or_else(|error| panic!("generated id should be valid: {error}"));
            }
            p
        })
}

/// Generates an utterance with optional speaker and `xml:id`.
pub fn utterance_strategy() -> impl Strategy<Value = Utterance> {
    (
        proptest::option::of(xml_id_strategy()),
        proptest::option::of(speaker_strategy()),
        prop::collection::vec(inline_strategy(), 1..=5)
            .prop_filter("must have visible content", |v| has_visible_content(v)),
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
    (
        proptest::option::of(xml_id_strategy()),
        // Use a single text node to avoid adjacent text merging in XML
        text_only_inline_strategy().prop_map(|inline| vec![inline]),
    )
        .prop_map(|(id, content)| {
            let mut p = P::from_inline(content)
                .unwrap_or_else(|error| panic!("generated content should be valid: {error}"));
            if let Some(id_value) = id {
                p.set_id(id_value)
                    .unwrap_or_else(|error| panic!("generated id should be valid: {error}"));
            }
            p
        })
}

/// Generates a text-only utterance (no Hi or Pause elements).
///
/// This is needed for XML round-trip testing because `quick-xml`'s serde
/// integration does not support serializing complex inline structures.
/// Uses a single text node to avoid adjacent text node merging during XML round-trip.
pub fn text_only_utterance_strategy() -> impl Strategy<Value = Utterance> {
    (
        proptest::option::of(xml_id_strategy()),
        proptest::option::of(speaker_strategy()),
        // Use a single text node to avoid adjacent text merging in XML
        text_only_inline_strategy().prop_map(|inline| vec![inline]),
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
    use proptest::strategy::ValueTree;
    use proptest::test_runner::TestRunner;

    #[test]
    fn paragraph_strategy_produces_valid_paragraphs() {
        let mut runner = TestRunner::default();
        for _ in 0..20 {
            let paragraph = paragraph_strategy()
                .new_tree(&mut runner)
                .unwrap_or_else(|error| panic!("strategy should generate values: {error}"))
                .current();

            assert!(
                has_visible_content_slice(paragraph.content()),
                "paragraph must have visible content"
            );
        }
    }

    #[test]
    fn utterance_strategy_produces_valid_utterances() {
        let mut runner = TestRunner::default();
        for _ in 0..20 {
            let utterance = utterance_strategy()
                .new_tree(&mut runner)
                .unwrap_or_else(|error| panic!("strategy should generate values: {error}"))
                .current();

            assert!(
                has_visible_content_slice(utterance.content()),
                "utterance must have visible content"
            );
        }
    }

    #[test]
    fn tei_body_strategy_produces_valid_bodies() {
        let mut runner = TestRunner::default();
        for _ in 0..20 {
            let body = tei_body_strategy()
                .new_tree(&mut runner)
                .unwrap_or_else(|error| panic!("strategy should generate values: {error}"))
                .current();

            // Bodies can be empty, but any blocks must be valid
            for block in body.blocks() {
                assert_block_has_visible_content(block);
            }
        }
    }

    fn assert_block_has_visible_content(block: &BodyBlock) {
        match block {
            BodyBlock::Paragraph(p) => {
                assert!(
                    has_visible_content_slice(p.content()),
                    "paragraph must have visible content"
                );
            }
            BodyBlock::Utterance(u) => {
                assert!(
                    has_visible_content_slice(u.content()),
                    "utterance must have visible content"
                );
            }
        }
    }
}
