use thiserror::Error;

/// Error raised when TEI body content fails validation.
#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum BodyContentError {
    /// A container (paragraph or utterance) was left empty after trimming.
    #[error("{container} content must include at least one non-empty segment")]
    EmptyContent {
        /// Name of the container that failed validation.
        container: &'static str,
    },

    /// A text segment lacked visible characters.
    #[error("{container} segments may not be empty")]
    EmptySegment {
        /// Name of the container that received the invalid segment.
        container: &'static str,
    },

    /// A speaker reference was provided but contained no visible characters.
    #[error("speaker references must not be empty")]
    EmptySpeaker,

    /// An `xml:id` attribute was provided but contained no visible characters.
    #[error("{container} identifiers must not be empty")]
    EmptyIdentifier {
        /// Name of the container that received the invalid identifier.
        container: &'static str,
    },

    /// An `xml:id` attribute contained internal whitespace, which is disallowed.
    #[error("{container} identifiers must not contain whitespace")]
    InvalidIdentifier {
        /// Name of the container that received the invalid identifier.
        container: &'static str,
    },
}
