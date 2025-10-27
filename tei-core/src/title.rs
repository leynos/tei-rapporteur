//! Provides the validated document title type used by `FileDesc` and
//! `TeiHeader`, guaranteeing non-empty trimmed text for serialisation.

use std::fmt;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Error raised when a [`DocumentTitle`] fails validation.
#[derive(Clone, Debug, Deserialize, Error, Eq, PartialEq, Serialize)]
pub enum DocumentTitleError {
    /// The provided title was empty after trimming whitespace.
    #[error("document title may not be empty")]
    Empty,
}

/// Title metadata carried by a [`crate::TeiDocument`].
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
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
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
    pub const fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Display for DocumentTitle {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl AsRef<str> for DocumentTitle {
    fn as_ref(&self) -> &str {
        self.as_str()
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

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;
    use std::fmt::Display;

    fn expect_ok<T, E>(result: Result<T, E>, message: &str) -> T
    where
        E: Display,
    {
        match result {
            Ok(value) => value,
            Err(error) => panic!("{message}: {error}"),
        }
    }

    fn expect_err<T, E>(result: Result<T, E>, message: &str) -> E
    where
        E: Display,
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
}
