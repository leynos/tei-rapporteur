use crate::text::{
    Inline,
    types::{Speaker, SpeakerValidationError, XmlId},
};

use super::{
    BodyContentError, ensure_container_content, normalise_optional_speaker, push_validated_inline,
    push_validated_text_segment, set_optional_identifier,
};
use serde::{Deserialize, Serialize};

/// Spoken utterance that may reference a speaker.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename = "u")]
pub struct Utterance {
    #[serde(rename = "xml:id", skip_serializing_if = "Option::is_none", default)]
    id: Option<XmlId>,
    #[serde(rename = "who", skip_serializing_if = "Option::is_none", default)]
    speaker: Option<Speaker>,
    #[serde(rename = "$value", default)]
    content: Vec<Inline>,
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
        let mut content = Vec::new();
        for segment in segments {
            push_validated_text_segment(&mut content, segment, "utterance")?;
        }
        ensure_container_content(&content, "utterance")?;

        Ok(Self {
            id: None,
            speaker: normalised_speaker,
            content,
        })
    }

    /// Builds an utterance from pre-constructed inline content.
    ///
    /// # Errors
    ///
    /// Returns [`BodyContentError::EmptyContent`] when the content lacks
    /// visible inline information.
    pub fn from_inline<S>(
        speaker: Option<S>,
        content: impl IntoIterator<Item = Inline>,
    ) -> Result<Self, BodyContentError>
    where
        S: Into<String>,
    {
        let normalised_speaker = normalise_optional_speaker(speaker)?;
        let collected: Vec<Inline> = content.into_iter().collect();
        ensure_container_content(&collected, "utterance")?;

        Ok(Self {
            id: None,
            speaker: normalised_speaker,
            content: collected,
        })
    }

    /// Sets an `xml:id` attribute on the utterance.
    ///
    /// # Errors
    ///
    /// Returns [`BodyContentError::EmptyIdentifier`] when the identifier lacks
    /// visible characters. Returns
    /// [`BodyContentError::InvalidIdentifier`] when the identifier contains
    /// internal whitespace.
    pub fn set_id(&mut self, id: impl Into<String>) -> Result<(), BodyContentError> {
        set_optional_identifier(&mut self.id, id, "utterance")
    }

    /// Clears any associated `xml:id`.
    pub fn clear_id(&mut self) {
        self.id = None;
    }

    /// Returns the utterance identifier when present.
    #[must_use]
    #[expect(
        clippy::missing_const_for_fn,
        reason = "Option::as_ref is not const-stable on current MSRV."
    )]
    pub fn id(&self) -> Option<&XmlId> {
        self.id.as_ref()
    }

    /// Assigns the speaker responsible for the utterance.
    ///
    /// # Errors
    ///
    /// Returns [`BodyContentError::EmptySpeaker`] when the provided speaker
    /// lacks visible characters.
    pub fn set_speaker(&mut self, speaker: impl Into<String>) -> Result<(), BodyContentError> {
        match Speaker::try_from(speaker.into()) {
            Ok(value) => {
                self.speaker = Some(value);
                Ok(())
            }
            Err(SpeakerValidationError::Empty) => Err(BodyContentError::EmptySpeaker),
        }
    }

    /// Clears the recorded speaker.
    pub fn clear_speaker(&mut self) {
        self.speaker = None;
    }

    /// Returns the recorded speaker when present.
    #[must_use]
    #[expect(
        clippy::missing_const_for_fn,
        reason = "Option::as_ref is not const-stable on current MSRV."
    )]
    pub fn speaker(&self) -> Option<&Speaker> {
        self.speaker.as_ref()
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
        push_validated_text_segment(&mut self.content, segment, "utterance")
    }

    /// Appends a new inline node.
    ///
    /// # Errors
    ///
    /// Returns [`BodyContentError::EmptySegment`] when the inline text lacks
    /// visible characters. Returns [`BodyContentError::EmptyContent`] when the
    /// inline element has no meaningful children.
    pub fn push_inline(&mut self, inline: Inline) -> Result<(), BodyContentError> {
        push_validated_inline(&mut self.content, inline, "utterance")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn records_inline_content() {
        let utterance = Utterance::new(Some("host"), ["Hello"]).expect("valid utterance");

        assert_eq!(utterance.content(), [Inline::text("Hello")]);
    }
}
