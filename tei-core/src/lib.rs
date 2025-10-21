//! Core data structures for TEI-Rapporteur.
//!
//! The crate concentrates on the canonical Rust data model for the profiled TEI
//! Episodic subset. Later phases will extend the structures, but the current
//! focus is the document shell (`TeiDocument`, `TeiHeader`, and `TeiText`) and
//! the header metadata types referenced throughout the design document.

mod header;
mod text;
mod title;

pub use header::{
    AnnotationSystem, AnnotationSystemId, EncodingDesc, FileDesc, HeaderValidationError,
    ProfileDesc, RevisionChange, RevisionDesc, TeiHeader,
};
pub use text::TeiText;
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
    pub fn new(header: TeiHeader, text: TeiText) -> Self {
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
        Ok(Self::new(header, TeiText::default()))
    }

    /// Returns the TEI header.
    #[must_use]
    pub fn header(&self) -> &TeiHeader {
        &self.header
    }

    /// Returns the textual component.
    #[must_use]
    pub fn text(&self) -> &TeiText {
        &self.text
    }

    /// Returns the validated title.
    #[must_use]
    pub fn title(&self) -> &DocumentTitle {
        self.header.file_desc().title()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructs_document_from_title() {
        let document = match TeiDocument::from_title_str("King Falls AM") {
            Ok(document) => document,
            Err(error) => panic!("valid document: {error}"),
        };
        assert_eq!(document.title().as_str(), "King Falls AM");
    }
}
