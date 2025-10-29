//! Core data structures for TEI-Rapporteur.
//!
//! The crate concentrates on the canonical Rust data model for the profiled TEI
//! Episodic subset. Later phases will extend the structures, but the current
//! focus is the document shell (`TeiDocument`, `TeiHeader`, and `TeiText`) and
//! the header metadata types referenced throughout the design document. The
//! text module models the TEI body using paragraphs and utterances so tests can
//! exercise real script fragments.

mod header;
mod text;
mod title;

pub use header::{
    AnnotationSystem, AnnotationSystemId, EncodingDesc, FileDesc, HeaderValidationError,
    LanguageTag, ProfileDesc, ResponsibleParty, RevisionChange, RevisionDesc, SpeakerName,
    TeiHeader,
};
pub use text::{
    BodyBlock, BodyContentError, Hi, IdentifierValidationError, Inline, P, Pause, Speaker,
    SpeakerValidationError, TeiBody, TeiText, Utterance, XmlId,
};
pub use title::{DocumentTitle, DocumentTitleError};

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors raised by TEI core data model operations.
#[derive(Clone, Debug, Error, Eq, PartialEq)]
#[non_exhaustive]
pub enum TeiError {
    /// Wrapper around [`DocumentTitleError`] values.
    #[error(transparent)]
    DocumentTitle(#[from] DocumentTitleError),
    /// Wrapper around [`HeaderValidationError`] values.
    #[error(transparent)]
    Header(#[from] HeaderValidationError),
    /// Wrapper around [`BodyContentError`] values.
    #[error(transparent)]
    Body(#[from] BodyContentError),
    /// Wrapper around [`IdentifierValidationError`] values.
    #[error(transparent)]
    Identifier(#[from] IdentifierValidationError),
    /// Wrapper around [`SpeakerValidationError`] values.
    #[error(transparent)]
    Speaker(#[from] SpeakerValidationError),
}

/// Root TEI document combining metadata and textual content.
///
/// # Examples
///
/// ```
/// use tei_core::{TeiDocument, TeiError};
///
/// let document = TeiDocument::from_title_str("Night Vale Episode")?;
/// assert_eq!(document.title().as_str(), "Night Vale Episode");
/// # Ok::<(), TeiError>(())
/// ```
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename = "TEI")]
pub struct TeiDocument {
    #[serde(rename = "teiHeader")]
    header: TeiHeader,
    #[serde(rename = "text")]
    text: TeiText,
}

impl TeiDocument {
    /// Builds a document from fully formed components.
    #[must_use]
    pub const fn new(header: TeiHeader, text: TeiText) -> Self {
        Self { header, text }
    }

    /// Validates an input title and constructs a skeletal document.
    ///
    /// # Errors
    ///
    /// Returns [`TeiError::DocumentTitle`] when the supplied title trims to an
    /// empty string.
    pub fn from_title_str(value: &str) -> Result<Self, TeiError> {
        let file_desc = FileDesc::from_title_str(value)?;
        let header = TeiHeader::new(file_desc);
        Ok(Self::new(header, TeiText::empty()))
    }

    /// Returns the TEI header.
    #[must_use]
    pub const fn header(&self) -> &TeiHeader {
        &self.header
    }

    /// Returns the textual component.
    #[must_use]
    pub const fn text(&self) -> &TeiText {
        &self.text
    }

    /// Returns the validated title.
    #[must_use]
    pub const fn title(&self) -> &DocumentTitle {
        self.header.file_desc().title()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructs_document_from_title() {
        let document = TeiDocument::from_title_str("King Falls AM").expect("valid document");
        assert_eq!(document.title().as_str(), "King Falls AM");
    }

    #[test]
    fn converts_document_title_error_into_tei_error() {
        let error: TeiError = DocumentTitleError::Empty.into();
        assert!(matches!(
            error,
            TeiError::DocumentTitle(DocumentTitleError::Empty)
        ));
    }

    #[test]
    fn converts_body_content_error_into_tei_error() {
        let error: TeiError = BodyContentError::EmptySpeaker.into();
        assert!(matches!(
            error,
            TeiError::Body(BodyContentError::EmptySpeaker)
        ));
    }

    #[test]
    fn converts_header_validation_error_into_tei_error() {
        let error: TeiError = HeaderValidationError::EmptyField { field: "header" }.into();

        assert!(matches!(
            error,
            TeiError::Header(HeaderValidationError::EmptyField { field: "header" })
        ));
    }

    #[test]
    fn converts_identifier_validation_error_into_tei_error() {
        let error: TeiError = IdentifierValidationError::Empty.into();

        assert!(matches!(
            error,
            TeiError::Identifier(IdentifierValidationError::Empty)
        ));
    }

    #[test]
    fn converts_speaker_validation_error_into_tei_error() {
        let error: TeiError = SpeakerValidationError::Empty.into();

        assert!(matches!(
            error,
            TeiError::Speaker(SpeakerValidationError::Empty)
        ));
    }
}
