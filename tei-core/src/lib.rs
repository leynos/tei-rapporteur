//! Core data structures for TEI-Rapporteur.
//!
//! The crate concentrates on the canonical Rust data model for the profiled TEI
//! Episodic subset. Later phases will extend the structures, but the current
//! focus is the document shell (`TeiDocument`, `TeiHeader`, and `TeiText`) and
//! the header metadata types referenced throughout the design document.

use std::fmt;

use thiserror::Error;

/// Error raised when a [`DocumentTitle`] fails validation.
#[derive(Debug, Error, Clone, Eq, PartialEq)]
pub enum DocumentTitleError {
    /// The provided title was empty after trimming whitespace.
    #[error("document title may not be empty")]
    Empty,
}

/// Error raised when TEI header metadata fails validation.
#[derive(Debug, Error, Clone, Eq, PartialEq)]
pub enum HeaderValidationError {
    /// A textual field was empty once normalised.
    #[error("{field} may not be empty")]
    EmptyField { field: &'static str },
}

/// Title metadata carried by a [`TeiDocument`].
///
/// Titles are trimmed and must not be empty, ensuring downstream consumers can
/// always serialise a non-empty `<title>` element.
///
/// # Examples
///
/// ```
/// use tei_core::{DocumentTitle, DocumentTitleError};
///
/// let title = DocumentTitle::new("Voynich Manuscript")?;
/// assert_eq!(title.as_str(), "Voynich Manuscript");
/// # Ok::<(), DocumentTitleError>(())
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentTitle(String);

impl DocumentTitle {
    /// Creates a validated document title.
    ///
    /// The input is trimmed; passing only whitespace returns
    /// [`DocumentTitleError::Empty`].
    ///
    /// # Errors
    ///
    /// Returns [`DocumentTitleError::Empty`] when the trimmed input is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use tei_core::{DocumentTitle, DocumentTitleError};
    ///
    /// let title = DocumentTitle::new("  Vox Machina ")?;
    /// assert_eq!(title.as_str(), "Vox Machina");
    /// # Ok::<(), DocumentTitleError>(())
    /// ```
    pub fn new<S>(value: S) -> Result<Self, DocumentTitleError>
    where
        S: Into<String>,
    {
        let raw = value.into();
        let trimmed = raw.trim();

        if trimmed.is_empty() {
            return Err(DocumentTitleError::Empty);
        }

        Ok(Self(trimmed.to_owned()))
    }

    /// Returns the title as a string slice.
    ///
    /// # Examples
    ///
    /// ```
    /// use tei_core::{DocumentTitle, DocumentTitleError};
    ///
    /// let title = DocumentTitle::new("Podmix")?;
    /// assert_eq!(title.as_str(), "Podmix");
    /// # Ok::<(), DocumentTitleError>(())
    /// ```
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Display for DocumentTitle {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl TryFrom<&str> for DocumentTitle {
    type Error = DocumentTitleError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<String> for DocumentTitle {
    type Error = DocumentTitleError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
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

/// Placeholder for the body of a TEI document.
///
/// The text model will be expanded in future steps. For now the structure
/// records linear text segments so fixtures can stash script fragments.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TeiText {
    segments: Vec<String>,
}

impl TeiText {
    /// Builds a `TeiText` from the provided segments.
    #[must_use]
    pub fn new<S>(segments: impl IntoIterator<Item = S>) -> Self
    where
        S: Into<String>,
    {
        Self {
            segments: segments.into_iter().map(Into::into).collect(),
        }
    }

    /// Returns an empty text node.
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    /// Appends a new text segment.
    pub fn push_segment<S>(&mut self, segment: S)
    where
        S: Into<String>,
    {
        self.segments.push(segment.into());
    }

    /// Returns the stored text segments.
    #[must_use]
    pub fn segments(&self) -> &[String] {
        self.segments.as_slice()
    }

    /// Reports whether any text has been recorded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }
}

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

/// Bibliographic metadata describing the TEI file.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileDesc {
    title: DocumentTitle,
    series: Option<String>,
    synopsis: Option<String>,
}

impl FileDesc {
    /// Builds a file description from a validated title.
    #[must_use]
    pub fn new(title: DocumentTitle) -> Self {
        Self {
            title,
            series: None,
            synopsis: None,
        }
    }

    /// Validates a raw title before creating the file description.
    ///
    /// # Errors
    ///
    /// Returns [`DocumentTitleError::Empty`] when the supplied title trims to an
    /// empty string.
    pub fn from_title_str(value: &str) -> Result<Self, DocumentTitleError> {
        DocumentTitle::new(value).map(Self::new)
    }

    /// Assigns an optional series label.
    #[must_use]
    pub fn with_series(mut self, series: impl Into<String>) -> Self {
        self.series = normalise_optional_text(series);
        self
    }

    /// Assigns an optional synopsis.
    #[must_use]
    pub fn with_synopsis(mut self, synopsis: impl Into<String>) -> Self {
        self.synopsis = normalise_optional_text(synopsis);
        self
    }

    /// Returns the document title.
    #[must_use]
    pub fn title(&self) -> &DocumentTitle {
        &self.title
    }

    /// Returns the series label when present.
    #[must_use]
    pub fn series(&self) -> Option<&str> {
        self.series.as_deref()
    }

    /// Returns the synopsis when present.
    #[must_use]
    pub fn synopsis(&self) -> Option<&str> {
        self.synopsis.as_deref()
    }
}

/// Audience and linguistic profile metadata.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ProfileDesc {
    synopsis: Option<String>,
    speakers: Vec<String>,
    languages: Vec<String>,
}

impl ProfileDesc {
    /// Creates an empty profile description.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Assigns an optional synopsis.
    #[must_use]
    pub fn with_synopsis(mut self, synopsis: impl Into<String>) -> Self {
        self.synopsis = normalise_optional_text(synopsis);
        self
    }

    /// Adds a speaker to the cast list.
    ///
    /// # Errors
    ///
    /// Returns [`HeaderValidationError::EmptyField`] when the speaker name is
    /// empty after trimming.
    pub fn add_speaker(&mut self, speaker: impl Into<String>) -> Result<(), HeaderValidationError> {
        let Some(speaker) = normalise_optional_text(speaker) else {
            return Err(HeaderValidationError::EmptyField { field: "speaker" });
        };

        self.speakers.push(speaker);
        Ok(())
    }

    /// Adds a language identifier to the profile.
    ///
    /// # Errors
    ///
    /// Returns [`HeaderValidationError::EmptyField`] when the language tag is
    /// empty after trimming.
    pub fn add_language(
        &mut self,
        language: impl Into<String>,
    ) -> Result<(), HeaderValidationError> {
        let Some(language) = normalise_optional_text(language) else {
            return Err(HeaderValidationError::EmptyField { field: "language" });
        };

        self.languages.push(language);
        Ok(())
    }

    /// Returns the synopsis when present.
    #[must_use]
    pub fn synopsis(&self) -> Option<&str> {
        self.synopsis.as_deref()
    }

    /// Returns the registered speakers.
    #[must_use]
    pub fn speakers(&self) -> &[String] {
        self.speakers.as_slice()
    }

    /// Returns the recorded languages.
    #[must_use]
    pub fn languages(&self) -> &[String] {
        self.languages.as_slice()
    }

    /// Reports whether any metadata has been recorded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.synopsis.is_none() && self.speakers.is_empty() && self.languages.is_empty()
    }
}

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
    use rstest::rstest;

    fn expect_ok<T, E>(result: Result<T, E>, message: &str) -> T
    where
        E: std::fmt::Display,
    {
        match result {
            Ok(value) => value,
            Err(error) => panic!("{message}: {error}"),
        }
    }

    fn expect_err<T, E>(result: Result<T, E>, message: &str) -> E
    where
        E: std::fmt::Display,
    {
        match result {
            Ok(_) => panic!("{message}"),
            Err(error) => error,
        }
    }

    #[rstest]
    #[case("Voynich Manuscript", "Voynich Manuscript")]
    #[case("  The Magnus Archives  ", "The Magnus Archives")]
    fn trims_and_validates_titles(#[case] input: &str, #[case] expected: &str) {
        let title = expect_ok(DocumentTitle::new(input), "valid title");
        assert_eq!(title.as_str(), expected);
    }

    #[rstest]
    #[case("")]
    #[case("    ")]
    fn rejects_empty_titles(#[case] input: &str) {
        let error = expect_err(DocumentTitle::new(input), "empty titles are invalid");
        assert_eq!(error, DocumentTitleError::Empty);
    }

    #[test]
    fn constructs_document_from_title() {
        let document = expect_ok(TeiDocument::from_title_str("King Falls AM"), "valid doc");
        assert_eq!(document.title().as_str(), "King Falls AM");
    }

    #[test]
    fn file_desc_carries_optional_metadata() {
        let file_desc = expect_ok(FileDesc::from_title_str("Wolf 359"), "valid title")
            .with_series("Kakos Industries")
            .with_synopsis("Drama podcast");

        assert_eq!(file_desc.series(), Some("Kakos Industries"));
        assert_eq!(file_desc.synopsis(), Some("Drama podcast"));
    }

    #[test]
    fn profile_desc_tracks_speakers_and_languages() {
        let mut profile = ProfileDesc::new();
        expect_ok(profile.add_speaker("Keisha"), "speaker recorded");
        expect_ok(profile.add_language("en-GB"), "language recorded");

        assert_eq!(profile.speakers(), ["Keisha"]);
        assert_eq!(profile.languages(), ["en-GB"]);
    }

    #[test]
    fn annotation_system_requires_identifier() {
        let error = expect_err(
            AnnotationSystem::new("   ", "cliché detection"),
            "identifier rejected",
        );

        assert_eq!(
            error,
            HeaderValidationError::EmptyField {
                field: "annotation system",
            }
        );
    }

    #[test]
    fn revision_change_requires_description() {
        let error = expect_err(RevisionChange::new("   ", ""), "revision rejected");

        assert_eq!(
            error,
            HeaderValidationError::EmptyField {
                field: "revision note",
            }
        );
    }
}
