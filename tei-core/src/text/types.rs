//! Validated wrapper types for TEI identifier and speaker attributes.
//!
//! Provides `XmlId` and `Speaker` newtypes that enforce non-empty,
//! normalised values and reject invalid whitespace patterns.

use std::fmt;

use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer, Serialize};
use thiserror::Error;

use super::body::trim_preserving_original;

/// Validated wrapper for TEI `xml:id` attributes.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct XmlId(String);

/// Errors raised when normalising identifier input.
#[derive(Clone, Debug, Deserialize, Error, Eq, PartialEq, Serialize)]
pub enum IdentifierValidationError {
    /// The identifier trimmed to an empty string.
    #[error("identifiers must not be empty")]
    Empty,
    /// The identifier contained disallowed whitespace.
    #[error("identifiers must not contain whitespace")]
    ContainsWhitespace,
}

impl XmlId {
    /// Builds an identifier from user input.
    ///
    /// # Errors
    ///
    /// Returns [`IdentifierValidationError::Empty`] when the trimmed identifier
    /// is empty. Returns [`IdentifierValidationError::ContainsWhitespace`] when
    /// interior whitespace is present.
    pub fn new(value: impl Into<String>) -> Result<Self, IdentifierValidationError> {
        let trimmed = trim_preserving_original(value.into());

        if trimmed.is_empty() {
            return Err(IdentifierValidationError::Empty);
        }

        if trimmed.chars().any(char::is_whitespace) {
            return Err(IdentifierValidationError::ContainsWhitespace);
        }

        Ok(Self(trimmed))
    }

    /// Returns the identifier as a string slice.
    #[must_use]
    #[expect(
        clippy::missing_const_for_fn,
        reason = "String::as_str is not const-stable on current MSRV."
    )]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// Consumes the identifier and returns the owned string.
    #[must_use]
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl AsRef<str> for XmlId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for XmlId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl TryFrom<String> for XmlId {
    type Error = IdentifierValidationError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<&str> for XmlId {
    type Error = IdentifierValidationError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl<'de> Deserialize<'de> for XmlId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;

        Self::new(value).map_err(DeError::custom)
    }
}

/// Validated wrapper for utterance speaker references.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Speaker(String);

/// Errors raised when normalising speaker references.
#[derive(Clone, Debug, Deserialize, Error, Eq, PartialEq, Serialize)]
pub enum SpeakerValidationError {
    /// The speaker trimmed to an empty string.
    #[error("speaker references must not be empty")]
    Empty,
}

impl Speaker {
    /// Builds a speaker reference from user input.
    ///
    /// # Errors
    ///
    /// Returns [`SpeakerValidationError::Empty`] when the trimmed speaker
    /// reference is empty.
    pub fn new(value: impl Into<String>) -> Result<Self, SpeakerValidationError> {
        let trimmed = trim_preserving_original(value.into());

        if trimmed.is_empty() {
            return Err(SpeakerValidationError::Empty);
        }

        Ok(Self(trimmed))
    }

    /// Returns the speaker reference as a string slice.
    #[must_use]
    #[expect(
        clippy::missing_const_for_fn,
        reason = "String::as_str is not const-stable on current MSRV."
    )]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// Consumes the speaker reference and returns the owned string.
    #[must_use]
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl AsRef<str> for Speaker {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for Speaker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl TryFrom<String> for Speaker {
    type Error = SpeakerValidationError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<&str> for Speaker {
    type Error = SpeakerValidationError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl<'de> Deserialize<'de> for Speaker {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;

        Self::new(value).map_err(DeError::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json as json;

    #[test]
    fn xml_id_accepts_trimmed_identifiers() {
        let identifier = XmlId::new("  intro ")
            .unwrap_or_else(|error| panic!("identifier should be normalised: {error}"));
        assert_eq!(identifier.as_str(), "intro");
    }

    #[test]
    fn xml_id_rejects_identifiers_with_whitespace() {
        let result = XmlId::new("identifier with space");
        assert!(matches!(
            result,
            Err(IdentifierValidationError::ContainsWhitespace)
        ));
    }

    #[test]
    fn xml_id_display_matches_as_str() {
        let id = XmlId::new("intro")
            .unwrap_or_else(|error| panic!("identifier should validate: {error}"));
        assert_eq!(id.to_string(), id.as_str());
    }

    #[test]
    fn xml_id_deserialisation_rejects_invalid_input() {
        assert!(json::from_str::<XmlId>("\"   \"").is_err());
        assert!(json::from_str::<XmlId>("\"identifier with space\"").is_err());
    }

    #[test]
    fn speaker_rejects_empty_values() {
        let result = Speaker::new("   ");
        assert!(matches!(result, Err(SpeakerValidationError::Empty)));
    }

    #[test]
    fn speaker_accepts_trimmed_values() {
        let speaker = Speaker::new("  host  ")
            .unwrap_or_else(|error| panic!("speaker should be normalised: {error}"));
        assert_eq!(speaker.as_str(), "host");
    }

    #[test]
    fn speaker_try_from_str_validates() {
        let result = Speaker::try_from("   ");
        assert!(matches!(result, Err(SpeakerValidationError::Empty)));
    }

    #[test]
    fn speaker_deserialisation_rejects_empty_input() {
        assert!(json::from_str::<Speaker>("\"   \"").is_err());
    }
}
