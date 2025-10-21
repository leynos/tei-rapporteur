//! Audience and linguistic profile metadata for TEI headers.
//!
//! Tracks speakers and languages while normalizing optional fields.

use super::{HeaderValidationError, normalise_optional_text};

/// Audience and linguistic profile metadata.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ProfileDesc {
    synopsis: Option<String>,
    speakers: Vec<String>,
    languages: Vec<String>,
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
        Self::record_value(&mut self.speakers, speaker, "speaker")
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
        Self::record_value(&mut self.languages, language, "language")
    }

    /// Returns the synopsis when present.
    #[must_use]
    pub fn synopsis(&self) -> Option<&str> {
        self.synopsis.as_deref()
    }

    /// Returns the registered speakers.
    #[must_use]
    pub fn speakers(&self) -> &[String] {
        self.speakers.as_slice()
    }

    /// Returns the recorded languages.
    #[must_use]
    pub fn languages(&self) -> &[String] {
        self.languages.as_slice()
    }

    /// Reports whether any metadata has been recorded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.synopsis.is_none() && self.speakers.is_empty() && self.languages.is_empty()
    }

    fn record_value(
        target: &mut Vec<String>,
        value: impl Into<String>,
        field: &'static str,
    ) -> Result<(), HeaderValidationError> {
        let Some(value) = normalise_optional_text(value) else {
            return Err(HeaderValidationError::EmptyField { field });
        };

        target.push(value);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_desc_tracks_speakers_and_languages() {
        let mut profile = ProfileDesc::new();
        if let Err(error) = profile.add_speaker("Keisha") {
            panic!("speaker recorded: {error}");
        }
        if let Err(error) = profile.add_language("en-GB") {
            panic!("language recorded: {error}");
        }

        assert_eq!(profile.speakers(), ["Keisha"]);
        assert_eq!(profile.languages(), ["en-GB"]);
    }
}
