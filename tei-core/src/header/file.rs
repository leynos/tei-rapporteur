//! Bibliographic file description (`<fileDesc>`) for TEI headers.
//! Validates the title and normalises optional series and synopsis text.
use crate::title::{DocumentTitle, DocumentTitleError};

use super::normalise_optional_text;

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
    pub const fn new(title: DocumentTitle) -> Self {
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
    pub const fn title(&self) -> &DocumentTitle {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_desc_carries_optional_metadata() {
        let file_desc = FileDesc::from_title_str("Wolf 359")
            .expect("valid title")
            .with_series("Kakos Industries")
            .with_synopsis("Drama podcast");

        assert_eq!(file_desc.series(), Some("Kakos Industries"));
        assert_eq!(file_desc.synopsis(), Some("Drama podcast"));
    }
}
