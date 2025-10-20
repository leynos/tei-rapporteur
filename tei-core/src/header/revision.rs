use super::{HeaderValidationError, normalise_optional_text};

/// Revision history records.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RevisionDesc {
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
    pub fn changes(&self) -> &[RevisionChange] {
        self.changes.as_slice()
    }

    /// Reports whether the revision log has entries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.changes.is_empty()
    }
}

/// Individual revision note captured in `<revisionDesc>`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RevisionChange {
    description: String,
    resp: Option<String>,
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
        let Some(description) = normalise_optional_text(description) else {
            return Err(HeaderValidationError::EmptyField {
                field: "revision note",
            });
        };

        Ok(Self {
            description,
            resp: normalise_optional_text(resp),
        })
    }

    /// Returns the note text.
    #[must_use]
    pub fn description(&self) -> &str {
        self.description.as_str()
    }

    /// Returns the optional responsibility marker.
    #[must_use]
    pub fn resp(&self) -> Option<&str> {
        self.resp.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn revision_change_requires_description() {
        let Err(error) = RevisionChange::new("   ", "") else {
            panic!("revision rejected");
        };

        assert_eq!(
            error,
            HeaderValidationError::EmptyField {
                field: "revision note",
            }
        );
    }
}
