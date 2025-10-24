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
pub use text::{BodyBlock, BodyContentError, P, TeiBody, TeiText, Utterance};
pub use title::{DocumentTitle, DocumentTitleError};

/// Root TEI document combining metadata and textual content.
///
/// # Examples
///
/// ```
/// use tei_core::{DocumentTitleError, TeiDocument};
///
/// let document = TeiDocument::from_title_str("Night Vale Episode")?;
/// assert_eq!(document.title().as_str(), "Night Vale Episode");
/// # Ok::<(), DocumentTitleError>(())
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TeiDocument {
    header: TeiHeader,
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
    /// Returns [`DocumentTitleError::Empty`] when the supplied title trims to an
    /// empty string.
    pub fn from_title_str(value: &str) -> Result<Self, DocumentTitleError> {
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
}
