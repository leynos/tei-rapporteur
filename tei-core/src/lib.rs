//! Core data structures for TEI-Rapporteur.
//!
//! The crate concentrates on the canonical Rust data model for the profiled TEI
//! Episodic subset. Later phases will extend the structures, but the current
//! focus is the document shell (`TeiDocument`, `TeiHeader`, and `TeiText`) and
//! the header metadata types referenced throughout the design document. The
//! text module models the TEI body using paragraphs and utterances so tests can
//! exercise real script fragments.

mod annotation;
mod header;
mod text;
mod title;
mod validation;

pub use annotation::{AnnotationValidationError, Span, SpanGroup, StandOff};
pub use header::{
    AnnotationSystem, AnnotationSystemId, CiteData, CiteStructure, EncodingDesc, FileDesc,
    HeaderValidationError, LanguageTag, ProfileDesc, RefsDecl, ResponsibleParty, RevisionChange,
    RevisionDesc, SpeakerName, TeiHeader,
};
pub use text::{
    BodyBlock, BodyContentError, Certainty, CertaintyValidationError, Hi,
    IdentifierValidationError, Inline, P, Pause, Pointer, PointerList, PointerListValidationError,
    PointerValidationError, Speaker, SpeakerValidationError, TeiBody, TeiText, Utterance, XmlId,
};
pub use title::{DocumentTitle, DocumentTitleError};
pub use validation::ValidationError;

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
    /// Wrapper around [`PointerValidationError`] values.
    #[error(transparent)]
    Pointer(#[from] PointerValidationError),
    /// Wrapper around [`PointerListValidationError`] values.
    #[error(transparent)]
    PointerList(#[from] PointerListValidationError),
    /// Wrapper around [`AnnotationValidationError`] values.
    #[error(transparent)]
    Annotation(#[from] AnnotationValidationError),
    /// Wrapper around [`CertaintyValidationError`] values.
    #[error(transparent)]
    Certainty(#[from] CertaintyValidationError),
    /// Wrapper around [`SpeakerValidationError`] values.
    #[error(transparent)]
    Speaker(#[from] SpeakerValidationError),
    /// Wrapper around [`ValidationError`] values.
    #[error(transparent)]
    Validation(#[from] ValidationError),
    /// XML parsing or serialization failed.
    #[error("XML processing error: {message}")]
    Xml {
        /// Message describing the failure emitted by the XML layer.
        message: String,
    },
    /// Wrapper around I/O operations errors.
    #[error("I/O error: {message}")]
    Io {
        /// Message describing the I/O failure.
        message: String,
    },
}

impl TeiError {
    /// Builds an XML processing error with the provided message.
    #[must_use]
    pub fn xml(message: impl Into<String>) -> Self {
        Self::Xml {
            message: message.into(),
        }
    }

    /// Builds an I/O error with the provided message.
    #[must_use]
    pub fn io(message: impl Into<String>) -> Self {
        Self::Io {
            message: message.into(),
        }
    }
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
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
#[serde(rename = "TEI")]
pub struct TeiDocument {
    #[serde(rename = "teiHeader")]
    header: TeiHeader,
    #[serde(rename = "standOff", skip_serializing_if = "Option::is_none", default)]
    stand_off: Option<StandOff>,
    #[serde(rename = "text")]
    text: TeiText,
}

impl TeiDocument {
    /// Builds a document from fully formed components.
    #[must_use]
    pub const fn new(header: TeiHeader, text: TeiText) -> Self {
        Self {
            header,
            stand_off: None,
            text,
        }
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

    /// Returns the stand-off annotation layer when present.
    #[must_use]
    pub const fn stand_off(&self) -> Option<&StandOff> {
        self.stand_off.as_ref()
    }

    /// Returns the mutable stand-off annotation layer when present.
    #[expect(
        clippy::missing_const_for_fn,
        reason = "review requested a non-const mutable accessor to avoid a misleading API surface"
    )]
    pub fn stand_off_mut(&mut self) -> Option<&mut StandOff> {
        self.stand_off.as_mut()
    }

    /// Returns the textual component.
    #[must_use]
    pub const fn text(&self) -> &TeiText {
        &self.text
    }

    /// Attaches a stand-off annotation layer.
    #[must_use]
    pub fn with_stand_off(mut self, stand_off: StandOff) -> Self {
        self.stand_off = Some(stand_off);
        self
    }

    /// Returns the validated title.
    #[must_use]
    pub const fn title(&self) -> &DocumentTitle {
        self.header.file_desc().title()
    }

    /// Validates document-wide invariants.
    ///
    /// # Errors
    ///
    /// Returns [`TeiError::Validation`] when duplicated identifiers, unresolved
    /// internal pointers, invalid citation declarations, or unknown speaker
    /// references are detected.
    pub fn validate(&self) -> Result<(), TeiError> {
        Ok(validation::validate_document(self)?)
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for core document constructors and error conversions.

    use super::*;

    #[test]
    fn constructs_document_from_title() {
        let document = TeiDocument::from_title_str("King Falls AM")
            .unwrap_or_else(|error| panic!("valid document: {error}"));
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
    fn converts_pointer_list_validation_error_into_tei_error() {
        let error: TeiError = PointerListValidationError::Empty.into();

        assert!(matches!(
            error,
            TeiError::PointerList(PointerListValidationError::Empty)
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

    #[test]
    fn converts_validation_error_into_tei_error() {
        let error: TeiError = ValidationError::DuplicateXmlId {
            id: String::from("dup"),
        }
        .into();

        assert!(matches!(
            error,
            TeiError::Validation(ValidationError::DuplicateXmlId { id }) if id == "dup"
        ));
    }

    #[test]
    fn constructs_xml_error_from_message() {
        let error = TeiError::xml("missing header");
        let TeiError::Xml { message } = error else {
            panic!("expected XML error variant");
        };

        assert_eq!(message, "missing header");
    }

    #[test]
    fn constructs_io_error_from_message() {
        let error = TeiError::io("disk full");
        let TeiError::Io { message } = error else {
            panic!("expected I/O error variant");
        };

        assert_eq!(message, "disk full");
    }
}
