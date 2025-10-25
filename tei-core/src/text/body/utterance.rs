use crate::text::types::{Speaker, SpeakerValidationError, XmlId};

use super::{
    BodyContentError, normalise_optional_speaker, push_validated_segment, set_optional_identifier,
};

/// Spoken utterance that may reference a speaker.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Utterance {
    id: Option<XmlId>,
    speaker: Option<Speaker>,
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
        super::ensure_content(&collected, "utterance")?;

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
    fn rejects_identifier_with_whitespace() {
        let mut utterance = Utterance::new(Some("host"), ["hello"]).expect("valid utterance");
        let error = utterance
            .set_id("identifier with space")
            .expect_err("identifier whitespace should be rejected");

        assert_eq!(
            error,
            BodyContentError::InvalidIdentifier {
                container: "utterance"
            }
        );
    }
}
