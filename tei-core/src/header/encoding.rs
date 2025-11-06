//! Encoding documentation (`<encodingDesc>`) and annotation system metadata.
//!
//! Validates identifiers and normalizes optional descriptions to keep the TEI header consistent.

use std::fmt;

use super::{HeaderValidationError, normalise_optional_text};
use serde::{Deserialize, Serialize};

/// Aggregates encoding metadata such as annotation systems.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename = "encodingDesc")]
pub struct EncodingDesc {
    #[serde(
        rename = "annotationSystem",
        skip_serializing_if = "Vec::is_empty",
        default
    )]
    annotation_systems: Vec<AnnotationSystem>,
}

impl EncodingDesc {
    /// Creates an empty encoding description.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers an annotation system.
    pub fn add_annotation_system(&mut self, system: AnnotationSystem) {
        self.annotation_systems.push(system);
    }

    /// Returns the registered systems.
    #[must_use]
    pub const fn annotation_systems(&self) -> &[AnnotationSystem] {
        self.annotation_systems.as_slice()
    }

    /// Reports whether any annotation systems were registered.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.annotation_systems.is_empty()
    }

    /// Finds an annotation system by identifier.
    #[must_use]
    pub fn find(&self, id: &AnnotationSystemId) -> Option<&AnnotationSystem> {
        self.annotation_systems
            .iter()
            .find(|system| system.identifier() == id)
    }

    /// Finds an annotation system by identifier text.
    #[must_use]
    pub fn find_str(&self, id: &str) -> Option<&AnnotationSystem> {
        self.annotation_systems
            .iter()
            .find(|system| system.identifier() == id)
    }
}

/// Annotation toolkit metadata.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AnnotationSystem {
    #[serde(rename = "@xml:id")]
    identifier: AnnotationSystemId,
    #[serde(skip_serializing_if = "Option::is_none", rename = "desc", default)]
    description: Option<String>,
}

impl AnnotationSystem {
    /// Validates the identifier and creates the annotation descriptor.
    ///
    /// # Errors
    ///
    /// Returns [`HeaderValidationError::EmptyField`] when the identifier is
    /// empty after trimming.
    pub fn new(
        identifier: impl Into<String>,
        description: impl Into<String>,
    ) -> Result<Self, HeaderValidationError> {
        let canonical_identifier = AnnotationSystemId::new(identifier)?;

        Ok(Self {
            identifier: canonical_identifier,
            description: normalise_optional_text(description),
        })
    }

    /// Returns the canonical identifier.
    #[must_use]
    pub const fn identifier(&self) -> &AnnotationSystemId {
        &self.identifier
    }

    /// Returns the optional free-text description.
    #[must_use]
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
}

/// Canonical identifier for an annotation system.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(try_from = "String", into = "String")]
pub struct AnnotationSystemId(String);

impl AnnotationSystemId {
    /// Validates the identifier text and constructs the domain wrapper.
    ///
    /// # Errors
    ///
    /// Returns [`HeaderValidationError::EmptyField`] when the identifier is
    /// empty after normalization.
    pub fn new(value: impl Into<String>) -> Result<Self, HeaderValidationError> {
        let Some(identifier) = normalise_optional_text(value) else {
            return Err(HeaderValidationError::EmptyField {
                field: "annotation system",
            });
        };

        Ok(Self(identifier))
    }

    /// Returns the identifier as a string slice.
    #[must_use]
    pub const fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl AsRef<str> for AnnotationSystemId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for AnnotationSystemId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl PartialEq<str> for AnnotationSystemId {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<AnnotationSystemId> for str {
    fn eq(&self, other: &AnnotationSystemId) -> bool {
        self == other.as_str()
    }
}

impl TryFrom<String> for AnnotationSystemId {
    type Error = HeaderValidationError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<&str> for AnnotationSystemId {
    type Error = HeaderValidationError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<AnnotationSystemId> for String {
    fn from(value: AnnotationSystemId) -> Self {
        value.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json as json;
    use std::convert::TryFrom;

    #[test]
    fn annotation_system_requires_identifier() {
        let Err(error) = AnnotationSystem::new("   ", "cliché detection") else {
            panic!("empty identifier accepted");
        };

        assert_eq!(
            error,
            HeaderValidationError::EmptyField {
                field: "annotation system",
            }
        );
    }

    #[test]
    fn finds_registered_annotation_system() {
        let mut encoding = EncodingDesc::new();
        let system = AnnotationSystem::new("timestamps", "Word timing")
            .unwrap_or_else(|error| panic!("valid annotation system should construct: {error}"));
        let identifier = system.identifier().clone();
        encoding.add_annotation_system(system);

        assert!(encoding.find(&identifier).is_some());
        assert!(
            encoding
                .find(
                    &AnnotationSystemId::try_from("other")
                        .unwrap_or_else(|error| panic!("valid id: {error}")),
                )
                .is_none()
        );
        assert!(encoding.find_str(identifier.as_str()).is_some());
        assert!(encoding.find_str("missing").is_none());
    }

    #[test]
    fn blanks_are_removed_from_descriptions() {
        let system = AnnotationSystem::new("tok", "   ")
            .unwrap_or_else(|error| panic!("identifier should be valid: {error}"));

        assert!(system.description().is_none());
    }

    #[test]
    fn annotation_system_id_deserialisation_rejects_empty() {
        let result = json::from_str::<AnnotationSystemId>("\"   \"");

        assert!(result.is_err(), "empty identifier should not deserialise");
    }
}
