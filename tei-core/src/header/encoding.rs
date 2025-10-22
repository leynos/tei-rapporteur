//! Encoding documentation metadata and annotation tooling descriptors.
//!
//! Validates identifiers and normalizes optional descriptions to keep the TEI
//! header consistent.

use super::{HeaderValidationError, normalise_optional_text};

/// Aggregates encoding metadata such as annotation systems.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct EncodingDesc {
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
    pub fn annotation_systems(&self) -> &[AnnotationSystem] {
        self.annotation_systems.as_slice()
    }

    /// Reports whether any annotation systems were registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.annotation_systems.is_empty()
    }

    /// Finds an annotation system by identifier.
    #[must_use]
    pub fn find(&self, id: &AnnotationSystemId) -> Option<&AnnotationSystem> {
        self.annotation_systems
            .iter()
            .find(|system| system.identifier() == id)
    }
}

/// Annotation toolkit metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnnotationSystem {
    identifier: AnnotationSystemId,
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
        let identifier = AnnotationSystemId::new(identifier)?;

        Ok(Self {
            identifier,
            description: normalise_optional_text(description),
        })
    }

    /// Returns the canonical identifier.
    #[must_use]
    pub fn identifier(&self) -> &AnnotationSystemId {
        &self.identifier
    }

    /// Returns the optional free-text description.
    #[must_use]
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
}

/// Canonical identifier for an annotation system.
#[derive(Clone, Debug, Eq, PartialEq)]
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
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl AsRef<str> for AnnotationSystemId {
    fn as_ref(&self) -> &str {
        self.as_str()
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

#[cfg(test)]
mod tests {
    use super::*;

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
    #[expect(
        clippy::expect_used,
        reason = "Test fixtures assert success for valid metadata"
    )]
    fn finds_registered_annotation_system() {
        let mut encoding = EncodingDesc::new();
        let system = AnnotationSystem::new("timestamps", "Word timing")
            .expect("valid annotation system should construct");
        let identifier = system.identifier().clone();
        encoding.add_annotation_system(system);

        assert!(encoding.find(&identifier).is_some());
        assert!(
            encoding
                .find(&AnnotationSystemId::new("other").expect("valid id"))
                .is_none()
        );
    }
}
