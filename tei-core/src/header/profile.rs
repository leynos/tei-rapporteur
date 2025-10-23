//! Audience and linguistic profile metadata for TEI headers.
//!
//! Tracks speakers and languages while normalizing optional fields.

use std::fmt;
use std::str::FromStr;

use super::{HeaderValidationError, normalise_optional_text};

/// Validated speaker name stored within [`ProfileDesc`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpeakerName(String);

impl SpeakerName {
    /// Builds a speaker name after trimming whitespace.
    ///
    /// # Errors
    ///
    /// Returns [`HeaderValidationError::EmptyField`] when the name trims to an
    /// empty string.
    pub fn new(value: impl Into<String>) -> Result<Self, HeaderValidationError> {
        build_validated_text(value, "speaker").map(Self)
    }

    /// Returns the speaker name as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// Consumes the wrapper and returns the owned string.
    #[must_use]
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl AsRef<str> for SpeakerName {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for SpeakerName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl FromStr for SpeakerName {
    type Err = HeaderValidationError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl TryFrom<String> for SpeakerName {
    type Error = HeaderValidationError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<&str> for SpeakerName {
    type Error = HeaderValidationError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

/// Validated language identifier stored within [`ProfileDesc`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LanguageTag(String);

impl LanguageTag {
    /// Builds a language identifier after trimming whitespace.
    ///
    /// # Errors
    ///
    /// Returns [`HeaderValidationError::EmptyField`] when the tag trims to an
    /// empty string.
    pub fn new(value: impl Into<String>) -> Result<Self, HeaderValidationError> {
        build_validated_text(value, "language").map(Self)
    }

    /// Returns the language identifier as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// Consumes the wrapper and returns the owned string.
    #[must_use]
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl AsRef<str> for LanguageTag {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for LanguageTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl FromStr for LanguageTag {
    type Err = HeaderValidationError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl TryFrom<String> for LanguageTag {
    type Error = HeaderValidationError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<&str> for LanguageTag {
    type Error = HeaderValidationError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

/// Audience and linguistic profile metadata.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ProfileDesc {
    synopsis: Option<String>,
    speakers: Vec<SpeakerName>,
    languages: Vec<LanguageTag>,
}

impl ProfileDesc {
    /// Creates an empty profile description.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Assigns an optional synopsis.
    #[must_use]
    pub fn with_synopsis(mut self, synopsis: impl Into<String>) -> Self {
        self.synopsis = normalise_optional_text(synopsis);
        self
    }

    /// Adds a speaker to the cast list.
    ///
    /// # Errors
    ///
    /// Returns [`HeaderValidationError::EmptyField`] when the speaker name is
    /// empty after trimming.
    pub fn add_speaker(&mut self, speaker: impl Into<String>) -> Result<(), HeaderValidationError> {
        let speaker = SpeakerName::new(speaker)?;
        self.speakers.push(speaker);
        Ok(())
    }

    /// Adds a language identifier to the profile.
    ///
    /// # Errors
    ///
    /// Returns [`HeaderValidationError::EmptyField`] when the language tag is
    /// empty after trimming.
    pub fn add_language(
        &mut self,
        language: impl Into<String>,
    ) -> Result<(), HeaderValidationError> {
        let language = LanguageTag::new(language)?;
        self.languages.push(language);
        Ok(())
    }

    /// Returns the synopsis when present.
    #[must_use]
    pub fn synopsis(&self) -> Option<&str> {
        self.synopsis.as_deref()
    }

    /// Returns the registered speakers.
    #[must_use]
    pub fn speakers(&self) -> &[SpeakerName] {
        self.speakers.as_slice()
    }

    /// Returns the number of speakers recorded.
    #[must_use]
    pub fn len_speakers(&self) -> usize {
        self.speakers.len()
    }

    /// Returns the recorded languages.
    #[must_use]
    pub fn languages(&self) -> &[LanguageTag] {
        self.languages.as_slice()
    }

    /// Returns the number of language tags recorded.
    #[must_use]
    pub fn len_languages(&self) -> usize {
        self.languages.len()
    }

    /// Reports whether any metadata has been recorded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.synopsis.is_none() && self.speakers.is_empty() && self.languages.is_empty()
    }
}

fn build_validated_text(
    value: impl Into<String>,
    field: &'static str,
) -> Result<String, HeaderValidationError> {
    normalise_optional_text(value).ok_or(HeaderValidationError::EmptyField { field })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[expect(
        clippy::expect_used,
        reason = "Test exercises validated metadata constructors"
    )]
    fn profile_desc_tracks_speakers_and_languages() {
        let mut profile = ProfileDesc::new();
        profile.add_speaker("Keisha").expect("speaker recorded");
        profile.add_language("en-GB").expect("language recorded");

        assert_eq!(
            profile
                .speakers()
                .iter()
                .map(SpeakerName::as_str)
                .collect::<Vec<_>>(),
            ["Keisha"],
        );
        assert_eq!(profile.len_speakers(), 1);
        assert_eq!(
            profile
                .languages()
                .iter()
                .map(LanguageTag::as_str)
                .collect::<Vec<_>>(),
            ["en-GB"],
        );
        assert_eq!(profile.len_languages(), 1);
    }
}
