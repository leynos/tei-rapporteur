//! Paragraph body model handling inline content and identifier management.
//!
//! Defines the TEI `<p>` block with helper constructors that validate inline
//! segments and optional `xml:id` attributes.

use crate::text::{Inline, types::XmlId};

use super::{
    BodyContentError, ensure_container_content, push_validated_inline, push_validated_text_segment,
    set_optional_identifier,
};
use serde::{Deserialize, Serialize};

/// Paragraph element containing linear text segments.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename = "p")]
pub struct P {
    #[serde(rename = "xml:id", skip_serializing_if = "Option::is_none", default)]
    id: Option<XmlId>,
    #[serde(rename = "$value", default)]
    content: Vec<Inline>,
}

impl P {
    /// Builds a paragraph from the provided text segments.
    ///
    /// # Deprecated
    ///
    /// Use [`P::from_text_segments`] or [`P::from_inline`] to construct
    /// paragraphs. This helper forwards to [`P::from_text_segments`].
    ///
    /// # Errors
    ///
    /// Returns [`BodyContentError::EmptyContent`] when no segments contain
    /// visible characters.
    #[deprecated(
        since = "0.1.0",
        note = "use `P::from_text_segments` or `P::from_inline`"
    )]
    pub fn new<S>(segments: impl IntoIterator<Item = S>) -> Result<Self, BodyContentError>
    where
        S: Into<String>,
    {
        Self::from_text_segments(segments)
    }

    /// Builds a paragraph from text segments, validating inline content.
    ///
    /// # Errors
    ///
    /// Returns [`BodyContentError::EmptyContent`] when no segments contain
    /// visible characters.
    pub fn from_text_segments<S>(
        segments: impl IntoIterator<Item = S>,
    ) -> Result<Self, BodyContentError>
    where
        S: Into<String>,
    {
        let mut content = Vec::new();
        for segment in segments {
            push_validated_text_segment(&mut content, segment, "paragraph")?;
        }
        ensure_container_content(&content, "paragraph")?;

        Ok(Self { id: None, content })
    }

    /// Builds a paragraph from pre-constructed inline content.
    ///
    /// # Errors
    ///
    /// Returns [`BodyContentError::EmptyContent`] when the content lacks
    /// visible inline information.
    pub fn from_inline(
        content: impl IntoIterator<Item = Inline>,
    ) -> Result<Self, BodyContentError> {
        let collected: Vec<Inline> = content.into_iter().collect();
        ensure_container_content(&collected, "paragraph")?;

        Ok(Self {
            id: None,
            content: collected,
        })
    }

    /// Sets an `xml:id` attribute on the paragraph.
    ///
    /// # Errors
    ///
    /// Returns [`BodyContentError::EmptyIdentifier`] when the identifier lacks
    /// visible characters. Returns
    /// [`BodyContentError::InvalidIdentifier`] when the identifier contains
    /// internal whitespace.
    pub fn set_id(&mut self, id: impl Into<String>) -> Result<(), BodyContentError> {
        set_optional_identifier(&mut self.id, id, "paragraph")
    }

    /// Clears any associated `xml:id`.
    pub fn clear_id(&mut self) {
        self.id = None;
    }

    /// Returns the paragraph identifier when present.
    #[must_use]
    #[expect(
        clippy::missing_const_for_fn,
        reason = "Option::as_ref is not const-stable on current MSRV."
    )]
    pub fn id(&self) -> Option<&XmlId> {
        self.id.as_ref()
    }

    /// Returns the stored segments.
    #[must_use]
    pub const fn content(&self) -> &[Inline] {
        self.content.as_slice()
    }

    /// Appends a new segment.
    ///
    /// # Errors
    ///
    /// Returns [`BodyContentError::EmptySegment`] when the segment lacks visible
    /// characters.
    pub fn push_segment<S>(&mut self, segment: S) -> Result<(), BodyContentError>
    where
        S: Into<String>,
    {
        push_validated_text_segment(&mut self.content, segment, "paragraph")
    }

    /// Appends a new inline node.
    ///
    /// # Errors
    ///
    /// Returns [`BodyContentError::EmptySegment`] when the inline text lacks
    /// visible characters. Returns [`BodyContentError::EmptyContent`] when the
    /// inline element has no meaningful children.
    pub fn push_inline(&mut self, inline: Inline) -> Result<(), BodyContentError> {
        push_validated_inline(&mut self.content, inline, "paragraph")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::{Inline, body::Utterance};
    use rstest::rstest;

    fn set_paragraph_identifier(id: &str) -> Result<(), BodyContentError> {
        let mut paragraph = P::from_text_segments(["content"])?;

        paragraph.set_id(id)
    }

    fn set_utterance_identifier(id: &str) -> Result<(), BodyContentError> {
        let mut utterance = Utterance::from_text_segments(Some("host"), ["hello"])?;

        utterance.set_id(id)
    }

    #[test]
    fn rejects_empty_paragraph_segments() {
        let result = P::from_text_segments(Vec::<String>::new());
        assert!(matches!(
            result,
            Err(BodyContentError::EmptyContent { container }) if container == "paragraph"
        ));
    }

    #[rstest]
    #[case::paragraph(
        "paragraph",
        set_paragraph_identifier as fn(&str) -> Result<(), BodyContentError>,
    )]
    #[case::utterance(
        "utterance",
        set_utterance_identifier as fn(&str) -> Result<(), BodyContentError>,
    )]
    fn rejects_identifier_with_whitespace(
        #[case] container: &'static str,
        #[case] constructor: fn(&str) -> Result<(), BodyContentError>,
    ) {
        let Err(error) = constructor("identifier with space") else {
            panic!("identifier whitespace should be rejected");
        };

        assert_eq!(error, BodyContentError::InvalidIdentifier { container });
    }

    #[test]
    fn exposes_content_as_inline_nodes() {
        let paragraph = P::from_text_segments(["Hello world"])
            .unwrap_or_else(|error| panic!("paragraph should be valid: {error}"));

        assert_eq!(paragraph.content(), [Inline::text("Hello world")]);
    }
}
