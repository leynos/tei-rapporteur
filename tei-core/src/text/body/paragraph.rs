use crate::text::types::XmlId;

use super::{BodyContentError, push_validated_segment, set_optional_identifier};

/// Paragraph element containing linear text segments.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct P {
    id: Option<XmlId>,
    segments: Vec<String>,
}

impl P {
    /// Builds a paragraph from the provided text segments.
    ///
    /// # Errors
    ///
    /// Returns [`BodyContentError::EmptyContent`] when no segments contain
    /// visible characters.
    pub fn new<S>(segments: impl IntoIterator<Item = S>) -> Result<Self, BodyContentError>
    where
        S: Into<String>,
    {
        let collected: Vec<String> = segments.into_iter().map(Into::into).collect();
        super::ensure_content(&collected, "paragraph")?;

        Ok(Self {
            id: None,
            segments: collected,
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
    pub const fn segments(&self) -> &[String] {
        self.segments.as_slice()
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
        push_validated_segment(&mut self.segments, segment, "paragraph")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_paragraph_segments() {
        let result = P::new(Vec::<String>::new());
        assert!(matches!(
            result,
            Err(BodyContentError::EmptyContent { container }) if container == "paragraph"
        ));
    }

    #[test]
    fn rejects_identifier_with_whitespace() {
        let mut paragraph = P::new(["content"]).expect("valid paragraph");
        let error = paragraph
            .set_id("identifier with space")
            .expect_err("identifier whitespace should be rejected");

        assert_eq!(
            error,
            BodyContentError::InvalidIdentifier {
                container: "paragraph"
            }
        );
    }
}
