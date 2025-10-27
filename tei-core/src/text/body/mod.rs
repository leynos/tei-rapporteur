mod error;
mod paragraph;
mod utterance;
mod validation;

pub use error::BodyContentError;
pub use paragraph::P;
pub use utterance::Utterance;

pub(crate) use validation::{
    ensure_container_content, normalise_optional_speaker, push_validated_inline,
    push_validated_text_segment, set_optional_identifier, trim_preserving_original,
};

use serde::{Deserialize, Serialize};

/// Ordered collection of block-level TEI elements.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
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
    /// let paragraph = P::new(["Hello"]).expect("valid paragraph");
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
    pub fn utterances(&self) -> impl Iterator<Item = &Utterance> {
        self.blocks.iter().filter_map(|block| {
            if let BodyBlock::Utterance(utterance) = block {
                Some(utterance)
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
pub enum BodyBlock {
    /// A prose paragraph.
    #[serde(rename = "p")]
    Paragraph(P),
    /// A spoken utterance.
    #[serde(rename = "u")]
    Utterance(Utterance),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn body_iterators_filter_by_variant() {
        let paragraph = P::new(["Setup"]).expect("valid paragraph");
        let utterance = Utterance::new(Some("host"), ["Hello"]).expect("valid utterance");

        let mut body = TeiBody::default();
        body.push_paragraph(paragraph.clone());
        body.push_utterance(utterance.clone());

        assert_eq!(body.paragraphs().collect::<Vec<_>>(), vec![&paragraph]);
        assert_eq!(body.utterances().collect::<Vec<_>>(), vec![&utterance]);
    }
}
