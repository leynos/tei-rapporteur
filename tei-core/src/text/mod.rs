//! Models the textual body stored alongside the TEI header metadata.
//!
//! The text model now records structured body content. A `TeiText` owns a
//! `TeiBody`, which in turn stores ordered blocks of paragraphs and utterances.
//! Each element validates that visible text is present so downstream tooling can
//! rely on non-empty content.

mod body;
mod types;

pub use body::{BodyBlock, BodyContentError, P, TeiBody, Utterance};
pub use types::{IdentifierValidationError, Speaker, SpeakerValidationError, XmlId};

/// Body of a TEI document, including paragraphs and utterances.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TeiText {
    body: TeiBody,
}

impl TeiText {
    /// Builds a text wrapper around the provided body.
    ///
    /// # Examples
    ///
    /// ```
    /// use tei_core::{P, TeiBody, TeiText, Utterance};
    ///
    /// let mut body = TeiBody::default();
    /// body.push_paragraph(P::new(["Intro"]).expect("valid paragraph"));
    /// body.push_utterance(
    ///     Utterance::new(Some("host"), ["Welcome!"]).expect("valid utterance"),
    /// );
    ///
    /// let text = TeiText::new(body);
    /// assert!(!text.is_empty());
    /// ```
    #[must_use]
    pub const fn new(body: TeiBody) -> Self {
        Self { body }
    }

    /// Returns an empty text node.
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    /// Reports whether any body content has been recorded.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.body.is_empty()
    }

    /// Returns the stored body.
    #[must_use]
    pub const fn body(&self) -> &TeiBody {
        &self.body
    }

    /// Returns a mutable reference to the stored body.
    pub const fn body_mut(&mut self) -> &mut TeiBody {
        &mut self.body
    }
}

#[cfg(test)]
mod tests {
    use super::{BodyBlock, P, TeiBody, TeiText, Utterance};

    #[test]
    fn text_reports_emptiness() {
        let mut text = TeiText::empty();
        assert!(text.is_empty());

        let paragraph = P::new(["Intro paragraph"]).expect("valid paragraph");
        text.body_mut().push_paragraph(paragraph);
        assert!(!text.is_empty());
    }

    #[test]
    fn body_preserves_insertion_order() {
        let mut body = TeiBody::default();
        let paragraph = P::new(["Setup"]).expect("valid paragraph");
        let utterance = Utterance::new(Some("host"), ["Hello"]).expect("valid utterance");

        body.push_paragraph(paragraph.clone());
        body.push_utterance(utterance.clone());

        assert_eq!(
            body.blocks(),
            [
                BodyBlock::Paragraph(paragraph),
                BodyBlock::Utterance(utterance)
            ]
        );
    }
}
