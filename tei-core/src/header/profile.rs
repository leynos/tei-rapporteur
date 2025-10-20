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
        let Some(speaker) = normalise_optional_text(speaker) else {
            return Err(HeaderValidationError::EmptyField { field: "speaker" });
        };

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
        let Some(language) = normalise_optional_text(language) else {
            return Err(HeaderValidationError::EmptyField { field: "language" });
        };

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
