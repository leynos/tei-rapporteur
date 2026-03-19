//! Validated wrapper types for TEI identifiers, pointers, certainty values,
//! and speaker attributes.
//!
//! These wrappers keep XML-facing scalar values normalized and reject empty or
//! whitespace-delimited input that would be ambiguous in TEI attributes.

use std::fmt;

use serde::de::Error as DeError;
use serde::ser::Serializer;
use serde::{Deserialize, Deserializer, Serialize};
use thiserror::Error;

use super::body::trim_preserving_original;

/// Validated wrapper for TEI `xml:id` attributes.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
#[serde(transparent)]
pub struct XmlId(String);

/// Errors raised when normalizing identifier input.
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

/// Validated wrapper for TEI pointer tokens.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
#[serde(transparent)]
pub struct Pointer(String);

/// Errors raised when normalizing pointer input.
#[derive(Clone, Debug, Deserialize, Error, Eq, PartialEq, Serialize)]
pub enum PointerValidationError {
    /// The pointer trimmed to an empty string.
    #[error("pointers must not be empty")]
    Empty,
    /// The pointer contained disallowed whitespace.
    #[error("pointers must not contain whitespace")]
    ContainsWhitespace,
}

impl Pointer {
    /// Builds a pointer token from user input.
    ///
    /// # Errors
    ///
    /// Returns [`PointerValidationError::Empty`] when the pointer trims to an
    /// empty string. Returns [`PointerValidationError::ContainsWhitespace`]
    /// when interior whitespace is present.
    pub fn new(value: impl Into<String>) -> Result<Self, PointerValidationError> {
        let trimmed = trim_preserving_original(value.into());

        if trimmed.is_empty() {
            return Err(PointerValidationError::Empty);
        }

        if trimmed.chars().any(char::is_whitespace) {
            return Err(PointerValidationError::ContainsWhitespace);
        }

        Ok(Self(trimmed))
    }

    /// Returns the pointer token as a string slice.
    #[must_use]
    #[expect(
        clippy::missing_const_for_fn,
        reason = "String::as_str is not const-stable on current MSRV."
    )]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// Returns the referenced internal id when the pointer is `#`-prefixed.
    #[must_use]
    pub fn internal_id(&self) -> Option<&str> {
        self.0.strip_prefix('#')
    }

    /// Consumes the pointer token and returns the owned string.
    #[must_use]
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl AsRef<str> for Pointer {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for Pointer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl TryFrom<String> for Pointer {
    type Error = PointerValidationError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<&str> for Pointer {
    type Error = PointerValidationError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<Pointer> for String {
    fn from(value: Pointer) -> Self {
        value.0
    }
}

impl<'de> Deserialize<'de> for Pointer {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;

        Self::new(value).map_err(DeError::custom)
    }
}

/// Errors raised when normalizing whitespace-separated pointer lists.
#[derive(Clone, Debug, Deserialize, Error, Eq, PartialEq, Serialize)]
pub enum PointerListValidationError {
    /// The list trimmed to an empty string or contained no pointer values.
    #[error("pointer lists must not be empty")]
    Empty,
    /// An individual pointer token was invalid.
    #[error(transparent)]
    InvalidPointer(#[from] PointerValidationError),
}

/// Whitespace-separated TEI pointer list attribute.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PointerList(Vec<Pointer>);

impl PointerList {
    /// Builds a pointer list from individual pointer values.
    ///
    /// # Errors
    ///
    /// Returns [`PointerListValidationError::Empty`] when the iterator yields
    /// no values. Returns [`PointerListValidationError::InvalidPointer`] when a
    /// pointer token is invalid.
    pub fn new<S>(values: impl IntoIterator<Item = S>) -> Result<Self, PointerListValidationError>
    where
        S: Into<String>,
    {
        let pointers = values
            .into_iter()
            .map(|value| Pointer::new(value.into()))
            .collect::<Result<Vec<_>, _>>()?;

        if pointers.is_empty() {
            return Err(PointerListValidationError::Empty);
        }

        Ok(Self(pointers))
    }

    /// Parses a TEI pointer-list attribute value.
    ///
    /// # Errors
    ///
    /// Returns [`PointerListValidationError::Empty`] when the input trims to an
    /// empty string. Returns [`PointerListValidationError::InvalidPointer`]
    /// when any token is invalid.
    pub fn parse_attribute(value: impl Into<String>) -> Result<Self, PointerListValidationError> {
        let trimmed = trim_preserving_original(value.into());

        if trimmed.is_empty() {
            return Err(PointerListValidationError::Empty);
        }

        Self::new(trimmed.split_whitespace())
    }

    /// Returns the pointers as a slice.
    #[must_use]
    pub const fn as_slice(&self) -> &[Pointer] {
        self.0.as_slice()
    }

    /// Returns an iterator over the pointers.
    pub fn iter(&self) -> impl Iterator<Item = &Pointer> {
        self.0.iter()
    }

    /// Converts the list into owned string values.
    #[must_use]
    pub fn to_strings(&self) -> Vec<String> {
        self.0
            .iter()
            .map(|pointer| pointer.as_str().to_owned())
            .collect()
    }
}

impl TryFrom<String> for PointerList {
    type Error = PointerListValidationError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse_attribute(value)
    }
}

impl TryFrom<&str> for PointerList {
    type Error = PointerListValidationError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse_attribute(value)
    }
}

impl<S> TryFrom<Vec<S>> for PointerList
where
    S: Into<String>,
{
    type Error = PointerListValidationError;

    fn try_from(value: Vec<S>) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl Serialize for PointerList {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_strings().join(" "))
    }
}

impl<'de> Deserialize<'de> for PointerList {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;

        Self::parse_attribute(value).map_err(DeError::custom)
    }
}

/// Validated wrapper for TEI certainty values.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
#[serde(transparent)]
pub struct Certainty(String);

/// Errors raised when normalizing certainty input.
#[derive(Clone, Debug, Deserialize, Error, Eq, PartialEq, Serialize)]
pub enum CertaintyValidationError {
    /// The certainty trimmed to an empty string.
    #[error("certainty values must not be empty")]
    Empty,
    /// The certainty contained disallowed whitespace.
    #[error("certainty values must not contain whitespace")]
    ContainsWhitespace,
}

impl Certainty {
    /// Builds a certainty token from user input.
    ///
    /// # Errors
    ///
    /// Returns [`CertaintyValidationError::Empty`] when the certainty trims to
    /// an empty string. Returns [`CertaintyValidationError::ContainsWhitespace`]
    /// when interior whitespace is present.
    pub fn new(value: impl Into<String>) -> Result<Self, CertaintyValidationError> {
        let trimmed = trim_preserving_original(value.into());

        if trimmed.is_empty() {
            return Err(CertaintyValidationError::Empty);
        }

        if trimmed.chars().any(char::is_whitespace) {
            return Err(CertaintyValidationError::ContainsWhitespace);
        }

        Ok(Self(trimmed))
    }

    /// Returns the certainty token as a string slice.
    #[must_use]
    #[expect(
        clippy::missing_const_for_fn,
        reason = "String::as_str is not const-stable on current MSRV."
    )]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl AsRef<str> for Certainty {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for Certainty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl TryFrom<String> for Certainty {
    type Error = CertaintyValidationError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<&str> for Certainty {
    type Error = CertaintyValidationError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<Certainty> for String {
    fn from(value: Certainty) -> Self {
        value.0
    }
}

impl<'de> Deserialize<'de> for Certainty {
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
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
#[serde(transparent)]
pub struct Speaker(String);

/// Errors raised when normalizing speaker references.
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
    //! Unit tests for validated TEI scalar and list wrappers.

    use super::*;
    use tei_serde::json;

    #[test]
    fn xml_id_accepts_trimmed_identifiers() {
        let identifier = XmlId::new("  intro ")
            .unwrap_or_else(|error| panic!("identifier should be normalized: {error}"));
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
    fn pointer_list_round_trips_as_attribute_text() {
        let pointers = PointerList::new(["#u1", "https://example.test/source"])
            .unwrap_or_else(|error| panic!("pointer list should validate: {error}"));

        let serialized =
            json::to_string(&pointers).unwrap_or_else(|error| panic!("pointer list JSON: {error}"));
        assert_eq!(serialized, "\"#u1 https://example.test/source\"");

        let deserialized = json::from_str::<PointerList>(&serialized)
            .unwrap_or_else(|error| panic!("pointer list should deserialize: {error}"));
        assert_eq!(deserialized, pointers);
    }

    #[test]
    fn pointer_internal_id_detects_hash_targets() {
        let internal = Pointer::new("#u1").unwrap_or_else(|error| panic!("valid pointer: {error}"));
        let external = Pointer::new("https://example.test/u1")
            .unwrap_or_else(|error| panic!("valid pointer: {error}"));

        assert_eq!(internal.internal_id(), Some("u1"));
        assert_eq!(external.internal_id(), None);
    }

    #[test]
    fn certainty_rejects_whitespace() {
        let result = Certainty::new("very high");
        assert!(matches!(
            result,
            Err(CertaintyValidationError::ContainsWhitespace)
        ));
    }

    #[test]
    fn speaker_rejects_empty_values() {
        let result = Speaker::new("   ");
        assert!(matches!(result, Err(SpeakerValidationError::Empty)));
    }

    #[test]
    fn speaker_accepts_trimmed_values() {
        let speaker = Speaker::new("  host  ")
            .unwrap_or_else(|error| panic!("speaker should be normalized: {error}"));
        assert_eq!(speaker.as_str(), "host");
    }
}
