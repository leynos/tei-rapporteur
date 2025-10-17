//! Core data structures for TEI-Rapporteur.
//!
//! The crate currently focuses on foundational types that exercise the
//! workspace scaffolding. Future phases will flesh out the full TEI Episodic
//! Profile.

use std::fmt;

use thiserror::Error;

/// Error raised when a [`DocumentTitle`] fails validation.
#[derive(Debug, Error, Clone, Eq, PartialEq)]
pub enum DocumentTitleError {
    /// The provided title was empty after trimming whitespace.
    #[error("document title may not be empty")]
    Empty,
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

/// Minimal TEI document placeholder.
///
/// The struct captures the document title so other crates can demonstrate
/// dependencies during the scaffolding stage.
///
/// # Examples
///
/// ```
/// use tei_core::{DocumentTitle, DocumentTitleError, TeiDocument};
///
/// let document = TeiDocument::from_title_str("Night Vale Episode")?;
/// assert_eq!(document.title().as_str(), "Night Vale Episode");
/// # Ok::<(), DocumentTitleError>(())
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TeiDocument {
    title: DocumentTitle,
}

impl TeiDocument {
    /// Builds a document from an already validated title.
    #[must_use]
    pub fn new(title: DocumentTitle) -> Self {
        Self { title }
    }

    /// Validates an input title and constructs the document.
    ///
    /// # Examples
    ///
    /// ```
    /// use tei_core::{DocumentTitleError, TeiDocument};
    ///
    /// let document = TeiDocument::from_title_str("Within the Wires")?;
    /// assert_eq!(document.title().as_str(), "Within the Wires");
    /// # Ok::<(), DocumentTitleError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`DocumentTitleError::Empty`] when the supplied title trims to an
    /// empty string.
    pub fn from_title_str(value: &str) -> Result<Self, DocumentTitleError> {
        DocumentTitle::new(value).map(Self::new)
    }

    /// Returns the validated title.
    ///
    /// # Examples
    ///
    /// ```
    /// use tei_core::{DocumentTitle, DocumentTitleError, TeiDocument};
    ///
    /// let title = DocumentTitle::new("Epitaph")?;
    /// let document = TeiDocument::new(title.clone());
    /// assert_eq!(document.title(), &title);
    /// # Ok::<(), DocumentTitleError>(())
    /// ```
    #[must_use]
    pub fn title(&self) -> &DocumentTitle {
        &self.title
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
        let title = expect_ok(DocumentTitle::new("King Falls AM"), "valid title");
        let document = TeiDocument::new(title.clone());
        assert_eq!(document.title(), &title);
    }
}
