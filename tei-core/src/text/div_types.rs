//! Validated wrapper types for division typing attributes.

use std::fmt;

use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer, Serialize};
use thiserror::Error;

use crate::text::body::trim_preserving_original;

/// Validated wrapper for division `@type` attributes.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
#[serde(transparent)]
pub struct DivType(String);

/// Validated wrapper for division `@subtype` attributes.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
#[serde(transparent)]
pub struct DivSubtype(String);

/// Errors raised when normalizing division type input.
#[derive(Clone, Debug, Deserialize, Error, Eq, PartialEq, Serialize)]
pub enum DivTypeValidationError {
    /// The division type trimmed to an empty string.
    #[error("division type must not be empty")]
    Empty,
}

/// Errors raised when normalizing division subtype input.
#[derive(Clone, Debug, Deserialize, Error, Eq, PartialEq, Serialize)]
pub enum DivSubtypeValidationError {
    /// The division subtype trimmed to an empty string.
    #[error("division subtype must not be empty")]
    Empty,
}

impl DivType {
    /// Builds a division type from user input.
    ///
    /// Leading and trailing whitespace is stripped. The value must contain at
    /// least one visible character after trimming.
    ///
    /// # Errors
    ///
    /// Returns [`DivTypeValidationError::Empty`] when the trimmed value is
    /// empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use tei_core::DivType;
    ///
    /// let dt = DivType::new("show-notes")
    ///     .unwrap_or_else(|error| panic!("valid type: {error}"));
    /// assert_eq!(dt.as_str(), "show-notes");
    /// ```
    pub fn new(value: impl Into<String>) -> Result<Self, DivTypeValidationError> {
        let trimmed = trim_preserving_original(value.into());

        if trimmed.is_empty() {
            return Err(DivTypeValidationError::Empty);
        }

        Ok(Self(trimmed))
    }

    /// Returns the division type as a string slice.
    #[must_use]
    pub const fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl DivSubtype {
    /// Builds a division subtype from user input.
    ///
    /// Leading and trailing whitespace is stripped. The value must contain at
    /// least one visible character after trimming.
    ///
    /// # Errors
    ///
    /// Returns [`DivSubtypeValidationError::Empty`] when the trimmed value is
    /// empty.
    pub fn new(value: impl Into<String>) -> Result<Self, DivSubtypeValidationError> {
        let trimmed = trim_preserving_original(value.into());

        if trimmed.is_empty() {
            return Err(DivSubtypeValidationError::Empty);
        }

        Ok(Self(trimmed))
    }

    /// Returns the division subtype as a string slice.
    #[must_use]
    pub const fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

macro_rules! impl_div_string_type {
    ($name:ident, $error:ident) => {
        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                self.as_str()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }

        impl TryFrom<String> for $name {
            type Error = $error;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }

        impl TryFrom<&str> for $name {
            type Error = $error;

            fn try_from(value: &str) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let value = String::deserialize(deserializer)?;

                Self::new(value).map_err(DeError::custom)
            }
        }
    };
}

impl_div_string_type!(DivType, DivTypeValidationError);
impl_div_string_type!(DivSubtype, DivSubtypeValidationError);
