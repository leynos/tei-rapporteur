//! Models the textual body stored alongside the TEI header metadata.
//!
//! The text model now records structured body content. A `TeiText` owns a
//! `TeiBody`, which in turn stores ordered blocks of paragraphs and utterances.
//! Each element validates that visible text is present so downstream tooling can
//! rely on non-empty content.

use thiserror::Error;

/// Error raised when TEI body content fails validation.
#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum BodyContentError {
    /// A container (paragraph or utterance) was left empty after trimming.
    #[error("{container} content must include at least one non-empty segment")]
    EmptyContent {
        /// Name of the container that failed validation.
        container: &'static str,
    },

    /// A text segment lacked visible characters.
    #[error("{container} segments may not be empty")]
    EmptySegment {
        /// Name of the container that received the invalid segment.
        container: &'static str,
    },

    /// A speaker reference was provided but contained no visible characters.
    #[error("speaker references must not be empty")]
    EmptySpeaker,

    /// An `xml:id` attribute was provided but contained no visible characters.
    #[error("{container} identifiers must not be empty")]
    EmptyIdentifier {
        /// Name of the container that received the invalid identifier.
        container: &'static str,
    },
}

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

/// Ordered collection of block-level TEI elements.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TeiBody {
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
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BodyBlock {
    /// A prose paragraph.
    Paragraph(P),
    /// A spoken utterance.
    Utterance(Utterance),
}

/// Paragraph element containing linear text segments.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct P {
    id: Option<String>,
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
        ensure_content(&collected, "paragraph")?;

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
    /// visible characters.
    pub fn set_id(&mut self, id: impl Into<String>) -> Result<(), BodyContentError> {
        set_optional_identifier(&mut self.id, id, "paragraph")
    }

    /// Clears any associated `xml:id`.
    pub fn clear_id(&mut self) {
        self.id = None;
    }

    /// Returns the paragraph identifier when present.
    #[must_use]
    pub fn id(&self) -> Option<&str> {
        self.id.as_deref()
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

/// Spoken utterance that may reference a speaker.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Utterance {
    id: Option<String>,
    speaker: Option<String>,
    segments: Vec<String>,
}

impl Utterance {
    /// Builds an utterance from the provided segments and optional speaker.
    ///
    /// # Errors
    ///
    /// Returns [`BodyContentError::EmptyContent`] when no segments contain
    /// visible characters. Returns [`BodyContentError::EmptySpeaker`] when the
    /// provided speaker lacks visible characters.
    pub fn new<S, T>(
        speaker: Option<S>,
        segments: impl IntoIterator<Item = T>,
    ) -> Result<Self, BodyContentError>
    where
        S: Into<String>,
        T: Into<String>,
    {
        let normalised_speaker = normalise_optional_speaker(speaker)?;
        let collected: Vec<String> = segments.into_iter().map(Into::into).collect();
        ensure_content(&collected, "utterance")?;

        Ok(Self {
            id: None,
            speaker: normalised_speaker,
            segments: collected,
        })
    }

    /// Sets an `xml:id` attribute on the utterance.
    ///
    /// # Errors
    ///
    /// Returns [`BodyContentError::EmptyIdentifier`] when the identifier lacks
    /// visible characters.
    pub fn set_id(&mut self, id: impl Into<String>) -> Result<(), BodyContentError> {
        set_optional_identifier(&mut self.id, id, "utterance")
    }

    /// Clears any associated `xml:id`.
    pub fn clear_id(&mut self) {
        self.id = None;
    }

    /// Returns the utterance identifier when present.
    #[must_use]
    pub fn id(&self) -> Option<&str> {
        self.id.as_deref()
    }

    /// Assigns the speaker responsible for the utterance.
    ///
    /// # Errors
    ///
    /// Returns [`BodyContentError::EmptySpeaker`] when the provided speaker
    /// lacks visible characters.
    pub fn set_speaker(&mut self, speaker: impl Into<String>) -> Result<(), BodyContentError> {
        let trimmed = trim_preserving_original(speaker.into());

        if trimmed.is_empty() {
            return Err(BodyContentError::EmptySpeaker);
        }

        self.speaker = Some(trimmed);
        Ok(())
    }

    /// Clears the recorded speaker.
    pub fn clear_speaker(&mut self) {
        self.speaker = None;
    }

    /// Returns the recorded speaker when present.
    #[must_use]
    pub fn speaker(&self) -> Option<&str> {
        self.speaker.as_deref()
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
        push_validated_segment(&mut self.segments, segment, "utterance")
    }
}

fn ensure_content(segments: &[String], container: &'static str) -> Result<(), BodyContentError> {
    if segments.is_empty() || segments.iter().all(|segment| segment.trim().is_empty()) {
        return Err(BodyContentError::EmptyContent { container });
    }

    Ok(())
}

fn normalise_optional_speaker<S>(speaker: Option<S>) -> Result<Option<String>, BodyContentError>
where
    S: Into<String>,
{
    speaker.map(Into::into).map_or(Ok(None), |value| {
        let trimmed = trim_preserving_original(value);

        if trimmed.is_empty() {
            Err(BodyContentError::EmptySpeaker)
        } else {
            Ok(Some(trimmed))
        }
    })
}

fn trim_preserving_original(value: String) -> String {
    let trimmed = value.trim();

    if trimmed.len() == value.len() {
        value
    } else {
        trimmed.to_owned()
    }
}

fn set_optional_identifier(
    field: &mut Option<String>,
    value: impl Into<String>,
    container: &'static str,
) -> Result<(), BodyContentError> {
    let trimmed = trim_preserving_original(value.into());

    if trimmed.is_empty() {
        return Err(BodyContentError::EmptyIdentifier { container });
    }

    *field = Some(trimmed);
    Ok(())
}

fn push_validated_segment(
    segments: &mut Vec<String>,
    segment: impl Into<String>,
    container: &'static str,
) -> Result<(), BodyContentError> {
    let candidate = segment.into();

    if candidate.trim().is_empty() {
        return Err(BodyContentError::EmptySegment { container });
    }

    segments.push(candidate);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn rejects_empty_paragraph_segments() {
        let result = P::new(Vec::<String>::new());
        assert!(matches!(
            result,
            Err(BodyContentError::EmptyContent { container }) if container == "paragraph"
        ));
    }

    #[test]
    fn rejects_empty_utterance_segments() {
        let result = Utterance::new::<String, String>(None, Vec::<String>::new());
        assert!(matches!(
            result,
            Err(BodyContentError::EmptyContent { container }) if container == "utterance"
        ));
    }

    #[test]
    fn rejects_blank_speaker_reference() {
        let result = Utterance::new(Some("   "), ["Hello"]);
        assert!(matches!(result, Err(BodyContentError::EmptySpeaker)));
    }
}
