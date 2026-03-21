//! Validated TEI pointer, pointer-list, and certainty wrappers.
//!
//! These wrappers normalise XML-facing values and preserve the profile's
//! strict handling of whitespace-delimited attributes.

use std::fmt;

use serde::de::Error as DeError;
use serde::ser::Serializer;
use serde::{Deserialize, Deserializer, Serialize};
use thiserror::Error;

use crate::text::body::trim_preserving_original;

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

#[cfg(test)]
mod tests {
    //! Unit tests for pointer and certainty wrappers.

    use super::*;
    use tei_serde::json;

    #[test]
    fn pointer_list_round_trips_as_attribute_text() {
        let pointers = PointerList::new(["#u1", "https://example.test/source"])
            .unwrap_or_else(|error| panic!("pointer list should validate: {error}"));

        let serialized = json::to_string(&pointers)
            .unwrap_or_else(|error| panic!("failed to serialize PointerList to JSON: {error}"));
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
}
