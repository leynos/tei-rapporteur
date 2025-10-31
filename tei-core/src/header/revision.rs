//! Revision history metadata recorded in the TEI header.
//!
//! Captures revision notes alongside optional responsible parties while
//! enforcing trimmed, non-empty text for each recorded field.

use std::fmt;
use std::str::FromStr;

use super::{HeaderValidationError, normalise_optional_text};
use serde::de::{self, Deserializer};
use serde::{Deserialize, Serialize};

/// Named agent responsible for a revision note.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(try_from = "String", into = "String")]
pub struct ResponsibleParty(String);

impl ResponsibleParty {
    /// Builds a responsibility marker from the provided text.
    ///
    /// # Errors
    ///
    /// Returns [`HeaderValidationError::EmptyField`] when the marker trims to an
    /// empty string.
    pub fn new(value: impl Into<String>) -> Result<Self, HeaderValidationError> {
        required_text(value, "revision responsibility").map(Self)
    }

    /// Returns the marker as a string slice.
    #[must_use]
    #[expect(
        clippy::missing_const_for_fn,
        reason = "String::as_str is not const-stable on current MSRV."
    )]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    #[expect(
        clippy::missing_const_for_fn,
        reason = "Normalised strings may rely on non-const standard library APIs."
    )]
    fn from_normalised(value: String) -> Self {
        Self(value)
    }
}

impl AsRef<str> for ResponsibleParty {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for ResponsibleParty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl FromStr for ResponsibleParty {
    type Err = HeaderValidationError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl TryFrom<String> for ResponsibleParty {
    type Error = HeaderValidationError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<&str> for ResponsibleParty {
    type Error = HeaderValidationError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<ResponsibleParty> for String {
    fn from(value: ResponsibleParty) -> Self {
        value.0
    }
}

/// Revision history records.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename = "revisionDesc")]
pub struct RevisionDesc {
    #[serde(rename = "change", skip_serializing_if = "Vec::is_empty", default)]
    changes: Vec<RevisionChange>,
}

impl RevisionDesc {
    /// Creates an empty revision log.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends a revision note.
    pub fn add_change(&mut self, change: RevisionChange) {
        self.changes.push(change);
    }

    /// Returns the recorded revision history.
    #[must_use]
    #[expect(
        clippy::missing_const_for_fn,
        reason = "Vec::as_slice is not const-stable on the current toolchain."
    )]
    pub fn changes(&self) -> &[RevisionChange] {
        self.changes.as_slice()
    }

    /// Returns an iterator over the recorded changes.
    #[must_use = "Iterators must be consumed to inspect revision history"]
    pub fn iter(&self) -> std::slice::Iter<'_, RevisionChange> {
        self.changes.iter()
    }

    /// Reports whether the revision log has entries.
    #[must_use]
    #[expect(
        clippy::missing_const_for_fn,
        reason = "Vec::is_empty is not const-stable on the current toolchain."
    )]
    pub fn is_empty(&self) -> bool {
        self.changes.is_empty()
    }
}

impl<'a> IntoIterator for &'a RevisionDesc {
    type Item = &'a RevisionChange;
    type IntoIter = std::slice::Iter<'a, RevisionChange>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// Individual revision note captured in `<revisionDesc>`.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RevisionChange {
    #[serde(rename = "$value", deserialize_with = "de_nonempty_text")]
    description: String,
    #[serde(skip_serializing_if = "Option::is_none", rename = "resp", default)]
    resp: Option<ResponsibleParty>,
}

impl RevisionChange {
    /// Creates a revision note with an optional responsibility marker.
    ///
    /// # Errors
    ///
    /// Returns [`HeaderValidationError::EmptyField`] when the description is
    /// empty after trimming.
    pub fn new(
        description: impl Into<String>,
        resp: impl Into<String>,
    ) -> Result<Self, HeaderValidationError> {
        let normalised_description = required_text(description, "revision note")?;

        Ok(Self {
            description: normalised_description,
            resp: normalise_optional_text(resp).map(ResponsibleParty::from_normalised),
        })
    }

    /// Returns the note text.
    #[must_use]
    #[expect(
        clippy::missing_const_for_fn,
        reason = "String::as_str is not const-stable on current MSRV."
    )]
    pub fn description(&self) -> &str {
        self.description.as_str()
    }

    /// Returns the optional responsibility marker.
    #[must_use]
    #[expect(
        clippy::missing_const_for_fn,
        reason = "Option::as_ref is not const-stable on current MSRV."
    )]
    pub fn resp(&self) -> Option<&ResponsibleParty> {
        self.resp.as_ref()
    }
}

fn required_text(
    value: impl Into<String>,
    field: &'static str,
) -> Result<String, HeaderValidationError> {
    normalise_optional_text(value).ok_or(HeaderValidationError::EmptyField { field })
}

fn de_nonempty_text<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let raw = String::deserialize(deserializer)?;

    normalise_optional_text(raw).ok_or_else(|| de::Error::custom("empty revision note"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json as json;

    #[test]
    fn revision_change_requires_description() {
        let Err(error) = RevisionChange::new("   ", "") else {
            panic!("empty description accepted");
        };

        assert_eq!(
            error,
            HeaderValidationError::EmptyField {
                field: "revision note",
            }
        );
    }

    #[test]
    fn responsible_party_deserialisation_rejects_empty() {
        let result = json::from_str::<ResponsibleParty>("\"   \"");

        assert!(
            result.is_err(),
            "empty responsibility should not deserialise"
        );
    }

    #[test]
    fn revision_change_deserialisation_rejects_empty_description() {
        let payload = "{\"$value\": \"   \"}";
        let result = json::from_str::<RevisionChange>(payload);

        assert!(
            result.is_err(),
            "empty revision note should not deserialise"
        );
    }
}
