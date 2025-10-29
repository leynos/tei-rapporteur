//! TEI header model combining bibliographic, profile, encoding, and revision
//! metadata.
//!
//! Exposes the validation errors and helper types consumed throughout the
//! `tei-core` crate.

use serde::{Deserialize, Serialize};
use thiserror::Error;

mod encoding;
mod file;
mod profile;
mod revision;

pub use encoding::{AnnotationSystem, AnnotationSystemId, EncodingDesc};
pub use file::FileDesc;
pub use profile::{LanguageTag, ProfileDesc, SpeakerName};
pub use revision::{ResponsibleParty, RevisionChange, RevisionDesc};

/// Error raised when TEI header metadata fails validation.
#[derive(Clone, Debug, Error, Eq, PartialEq, Serialize)]
pub enum HeaderValidationError {
    /// A textual field was empty once normalised.
    #[error("{field} may not be empty")]
    EmptyField {
        /// Name of the empty field.
        field: &'static str,
    },
}

/// Metadata container for TEI header information.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename = "teiHeader")]
pub struct TeiHeader {
    #[serde(rename = "fileDesc")]
    file: FileDesc,
    #[serde(
        rename = "profileDesc",
        skip_serializing_if = "Option::is_none",
        default
    )]
    profile: Option<ProfileDesc>,
    #[serde(
        rename = "encodingDesc",
        skip_serializing_if = "Option::is_none",
        default
    )]
    encoding: Option<EncodingDesc>,
    #[serde(
        rename = "revisionDesc",
        skip_serializing_if = "Option::is_none",
        default
    )]
    revision: Option<RevisionDesc>,
}

impl TeiHeader {
    /// Creates a header from its mandatory file description.
    #[must_use]
    pub const fn new(file_desc: FileDesc) -> Self {
        Self {
            file: file_desc,
            profile: None,
            encoding: None,
            revision: None,
        }
    }

    /// Returns the file description.
    #[must_use]
    pub const fn file_desc(&self) -> &FileDesc {
        &self.file
    }

    /// Returns the profile description when provided.
    #[must_use]
    pub const fn profile_desc(&self) -> Option<&ProfileDesc> {
        self.profile.as_ref()
    }

    /// Returns the encoding description when provided.
    #[must_use]
    pub const fn encoding_desc(&self) -> Option<&EncodingDesc> {
        self.encoding.as_ref()
    }

    /// Returns the revision description when provided.
    #[must_use]
    pub const fn revision_desc(&self) -> Option<&RevisionDesc> {
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

#[must_use]
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
        let title = DocumentTitle::new("Title").expect("valid title");
        let header = TeiHeader::new(FileDesc::new(title))
            .with_profile_desc(ProfileDesc::new())
            .with_encoding_desc(EncodingDesc::new())
            .with_revision_desc(RevisionDesc::new());

        assert!(header.profile_desc().is_some());
        assert!(header.encoding_desc().is_some());
        assert!(header.revision_desc().is_some());
    }
}
