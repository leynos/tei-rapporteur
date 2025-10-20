use thiserror::Error;

mod encoding;
mod file;
mod profile;
mod revision;

pub use encoding::{AnnotationSystem, EncodingDesc};
pub use file::FileDesc;
pub use profile::ProfileDesc;
pub use revision::{RevisionChange, RevisionDesc};

/// Error raised when TEI header metadata fails validation.
#[derive(Debug, Error, Clone, Eq, PartialEq)]
pub enum HeaderValidationError {
    /// A textual field was empty once normalised.
    #[error("{field} may not be empty")]
    EmptyField { field: &'static str },
}

/// Metadata container for TEI header information.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TeiHeader {
    file: FileDesc,
    profile: Option<ProfileDesc>,
    encoding: Option<EncodingDesc>,
    revision: Option<RevisionDesc>,
}

impl TeiHeader {
    /// Creates a header from its mandatory file description.
    #[must_use]
    pub fn new(file_desc: FileDesc) -> Self {
        Self {
            file: file_desc,
            profile: None,
            encoding: None,
            revision: None,
        }
    }

    /// Returns the file description.
    #[must_use]
    pub fn file_desc(&self) -> &FileDesc {
        &self.file
    }

    /// Returns the profile description when provided.
    #[must_use]
    pub fn profile_desc(&self) -> Option<&ProfileDesc> {
        self.profile.as_ref()
    }

    /// Returns the encoding description when provided.
    #[must_use]
    pub fn encoding_desc(&self) -> Option<&EncodingDesc> {
        self.encoding.as_ref()
    }

    /// Returns the revision description when provided.
    #[must_use]
    pub fn revision_desc(&self) -> Option<&RevisionDesc> {
        self.revision.as_ref()
    }

    /// Attaches a profile description.
    #[must_use]
    pub fn with_profile_desc(mut self, profile_desc: ProfileDesc) -> Self {
        self.profile = Some(profile_desc);
        self
    }

    /// Attaches an encoding description.
    #[must_use]
    pub fn with_encoding_desc(mut self, encoding_desc: EncodingDesc) -> Self {
        self.encoding = Some(encoding_desc);
        self
    }

    /// Attaches a revision description.
    #[must_use]
    pub fn with_revision_desc(mut self, revision_desc: RevisionDesc) -> Self {
        self.revision = Some(revision_desc);
        self
    }
}

fn normalise_optional_text(value: impl Into<String>) -> Option<String> {
    let trimmed = value.into().trim().to_owned();

    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::title::DocumentTitle;

    #[test]
    fn attaches_optional_sections() {
        let header = TeiHeader::new(FileDesc::new(match DocumentTitle::new("Title") {
            Ok(title) => title,
            Err(error) => panic!("valid title: {error}"),
        }))
        .with_profile_desc(ProfileDesc::new())
        .with_encoding_desc(EncodingDesc::new())
        .with_revision_desc(RevisionDesc::new());

        assert!(header.profile_desc().is_some());
        assert!(header.encoding_desc().is_some());
        assert!(header.revision_desc().is_some());
    }
}
