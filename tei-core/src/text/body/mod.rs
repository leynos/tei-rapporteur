//! TEI body model: ordered sequence of block-level elements.
//!
//! Serializes as `<body>` containing `<p>`, `<u>`, and `<div>` elements via
//! serde with blocks stored in the `$value` field.

mod div;
mod error;
mod item;
mod list;
mod paragraph;
mod utterance;
mod validation;

pub use div::{Div, DivContent};
pub use error::BodyContentError;
pub use item::{Item, Label};
pub use list::List;
pub use paragraph::P;
pub use utterance::Utterance;

pub(crate) use validation::{
    ensure_container_content, normalise_optional_speaker, push_validated_inline,
    push_validated_text_segment, set_optional_identifier, trim_preserving_original,
};

use serde::{Deserialize, Serialize};

/// Ordered collection of block-level TEI elements.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
#[serde(rename = "body")]
pub struct TeiBody {
    #[serde(rename = "$value", default)]
    blocks: Vec<BodyBlock>,
}

impl TeiBody {
    /// Constructs a body from pre-existing blocks.
    ///
    /// # Examples
    ///
    /// ```
    /// use tei_core::{BodyBlock, P, TeiBody};
    ///
    /// let paragraph = P::from_text_segments(["Hello"]).unwrap_or_else(|error| {
    ///     panic!("paragraph should be valid: {error}")
    /// });
    /// let body = TeiBody::new([BodyBlock::Paragraph(paragraph)]);
    ///
    /// assert_eq!(body.blocks().len(), 1);
    /// ```
    #[must_use]
    pub fn new(blocks: impl IntoIterator<Item = BodyBlock>) -> Self {
        Self {
            blocks: blocks.into_iter().collect(),
        }
    }

    /// Appends a paragraph block to the body.
    pub fn push_paragraph(&mut self, paragraph: P) {
        self.blocks.push(BodyBlock::Paragraph(paragraph));
    }

    /// Appends an utterance block to the body.
    pub fn push_utterance(&mut self, utterance: Utterance) {
        self.blocks.push(BodyBlock::Utterance(utterance));
    }

    /// Appends a division block to the body.
    pub fn push_div(&mut self, div: Div) {
        self.blocks.push(BodyBlock::Div(div));
    }

    /// Extends the body with additional blocks.
    pub fn extend(&mut self, blocks: impl IntoIterator<Item = BodyBlock>) {
        self.blocks.extend(blocks);
    }

    /// Returns the recorded blocks.
    #[must_use]
    pub const fn blocks(&self) -> &[BodyBlock] {
        self.blocks.as_slice()
    }

    /// Returns an iterator over recorded paragraphs.
    #[must_use = "Iterators are lazy; iterate or collect to inspect paragraphs."]
    pub fn paragraphs(&self) -> impl Iterator<Item = &P> {
        self.blocks.iter().filter_map(|block| {
            if let BodyBlock::Paragraph(paragraph) = block {
                Some(paragraph)
            } else {
                None
            }
        })
    }

    /// Returns an iterator over recorded utterances.
    #[must_use = "Iterators are lazy; iterate or collect to inspect utterances."]
    pub fn utterances(&self) -> impl Iterator<Item = &Utterance> {
        self.blocks.iter().filter_map(|block| {
            if let BodyBlock::Utterance(utterance) = block {
                Some(utterance)
            } else {
                None
            }
        })
    }

    /// Returns an iterator over recorded divisions.
    #[must_use = "Iterators are lazy; iterate or collect to inspect divisions."]
    pub fn divs(&self) -> impl Iterator<Item = &Div> {
        self.blocks.iter().filter_map(|block| {
            if let BodyBlock::Div(div) = block {
                Some(div)
            } else {
                None
            }
        })
    }

    /// Reports whether the body contains any blocks.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.blocks.is_empty()
    }
}

/// Block-level body content.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
pub enum BodyBlock {
    /// A prose paragraph.
    #[serde(rename = "p")]
    Paragraph(P),
    /// A spoken utterance.
    #[serde(rename = "u")]
    Utterance(Utterance),
    /// A thematic division grouping related content.
    #[serde(rename = "div")]
    Div(Div),
}

#[cfg(test)]
mod tests {
    //! Unit tests for TEI body block construction and iterators.

    use super::*;

    #[test]
    fn body_iterators_filter_by_variant() {
        let paragraph = P::from_text_segments(["Setup"])
            .unwrap_or_else(|error| panic!("valid paragraph: {error}"));
        let utterance = Utterance::from_text_segments(Some("host"), ["Hello"])
            .unwrap_or_else(|error| panic!("valid utterance: {error}"));

        let mut body = TeiBody::default();
        body.push_paragraph(paragraph.clone());
        body.push_utterance(utterance.clone());

        assert_eq!(body.paragraphs().collect::<Vec<_>>(), vec![&paragraph]);
        assert_eq!(body.utterances().collect::<Vec<_>>(), vec![&utterance]);
    }

    /// Stage B prototype: validate serde round-trip for new `Div`/`List`/`Item`
    /// structures before committing to the final type shapes.
    ///
    /// This test uses JSON serialization to verify the serde structure works
    /// correctly. The actual XML serialization will be tested in Stage D after
    /// the streaming parser is implemented.
    #[test]
    fn div_serde_prototype() {
        use crate::text::{Inline, types::{PointerList, XmlId}};

        // Prototype Label struct
        #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
        #[serde(rename = "label")]
        struct Label {
            #[serde(rename = "$value", default)]
            content: Vec<Inline>,
        }

        // Prototype Item struct
        #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
        #[serde(rename = "item", deny_unknown_fields)]
        struct Item {
            #[serde(
                rename = "@xml:id",
                alias = "@id",
                skip_serializing_if = "Option::is_none",
                default
            )]
            id: Option<XmlId>,
            #[serde(rename = "@n", skip_serializing_if = "Option::is_none", default)]
            n: Option<String>,
            #[serde(rename = "@corresp", skip_serializing_if = "Option::is_none", default)]
            corresp: Option<PointerList>,
            #[serde(skip_serializing_if = "Option::is_none", default)]
            label: Option<Label>,
            #[serde(rename = "$value", default)]
            content: Vec<Inline>,
        }

        // Prototype List struct
        #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
        #[serde(rename = "list")]
        struct List {
            #[serde(
                rename = "@xml:id",
                alias = "@id",
                skip_serializing_if = "Option::is_none",
                default
            )]
            id: Option<XmlId>,
            #[serde(rename = "$value", default)]
            items: Vec<Item>,
        }

        // Prototype DivContent enum
        #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
        enum DivContent {
            #[serde(rename = "p")]
            Paragraph(P),
            #[serde(rename = "u")]
            Utterance(Utterance),
            #[serde(rename = "list")]
            List(List),
        }

        // Prototype Div struct
        #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
        #[serde(rename = "div")]
        struct Div {
            #[serde(rename = "@type")]
            div_type: String,
            #[serde(
                rename = "@xml:id",
                alias = "@id",
                skip_serializing_if = "Option::is_none",
                default
            )]
            id: Option<XmlId>,
            #[serde(rename = "$value", default)]
            content: Vec<DivContent>,
        }

        // Build test data: Div containing Paragraph, Utterance, and List with
        // two Items (one with a Label)
        let paragraph = P::from_text_segments(["Welcome to the show notes."])
            .unwrap_or_else(|error| panic!("valid paragraph: {error}"));
        let utterance = Utterance::from_text_segments(Some("host"), ["And that's all!"])
            .unwrap_or_else(|error| panic!("valid utterance: {error}"));

        let label = Label {
            content: vec![Inline::text("Link:")],
        };
        let item1 = Item {
            id: Some(XmlId::new("item1").expect("valid id")),
            n: Some("1".into()),
            corresp: None,
            label: Some(label),
            content: vec![Inline::text("Visit our website")],
        };
        let item2 = Item {
            id: None,
            n: Some("2".into()),
            corresp: Some(
                PointerList::new(["#guest1"]).expect("valid pointer list")
            ),
            label: None,
            content: vec![Inline::text("Guest bio summary")],
        };

        let list = List {
            id: None,
            items: vec![item1, item2],
        };

        let div = Div {
            div_type: "show-notes".into(),
            id: None,
            content: vec![
                DivContent::Paragraph(paragraph),
                DivContent::List(list),
                DivContent::Utterance(utterance),
            ],
        };

        // Serialize to JSON
        let json_output = tei_serde::serde_json::to_string(&div)
            .unwrap_or_else(|error| panic!("serialization should succeed: {error}"));

        // Deserialize back
        let round_tripped: Div = tei_serde::serde_json::from_str(&json_output)
            .unwrap_or_else(|error| {
                panic!(
                    "deserialization should succeed for JSON:\n{json_output}\n\nError: {error}"
                )
            });

        // Assert equality
        assert_eq!(
            round_tripped, div,
            "Round-trip failed: deserialized value does not match original"
        );
    }
}
