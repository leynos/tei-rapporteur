use super::{HeaderValidationError, normalise_optional_text};

/// Documentation of annotation systems used within the document.
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
}

/// Annotation toolkit metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnnotationSystem {
    identifier: String,
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
        let Some(identifier) = normalise_optional_text(identifier) else {
            return Err(HeaderValidationError::EmptyField {
                field: "annotation system",
            });
        };

        Ok(Self {
            identifier,
            description: normalise_optional_text(description),
        })
    }

    /// Returns the canonical identifier.
    #[must_use]
    pub fn identifier(&self) -> &str {
        self.identifier.as_str()
    }

    /// Returns the optional free-text description.
    #[must_use]
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn annotation_system_requires_identifier() {
        let Err(error) = AnnotationSystem::new("   ", "cliché detection") else {
            panic!("identifier rejected");
        };

        assert_eq!(
            error,
            HeaderValidationError::EmptyField {
                field: "annotation system",
            }
        );
    }
}
