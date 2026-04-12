//! Division heading model for structural body sections.
//!
//! Defines a profile-constrained TEI `<head>` wrapper used at the start of a
//! `<div>` to carry inline heading content.

use crate::text::Inline;

use super::{BodyContentError, ensure_container_content, push_validated_text_segment};
use serde::{Deserialize, Serialize};

/// Heading prefix for a structural division, containing inline content.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
#[serde(rename = "head")]
pub struct Head {
    #[serde(rename = "$value", default)]
    content: Vec<Inline>,
}

impl Head {
    /// Builds a heading from pre-constructed inline content.
    ///
    /// # Errors
    ///
    /// Returns [`BodyContentError::EmptySegment`] with `container` set to
    /// `"head"` when the content lacks visible inline information.
    pub fn new(content: impl IntoIterator<Item = Inline>) -> Result<Self, BodyContentError> {
        let collected: Vec<Inline> = content.into_iter().collect();
        ensure_container_content(&collected, "head")?;

        Ok(Self { content: collected })
    }

    /// Builds a heading from plain text.
    ///
    /// # Errors
    ///
    /// Returns [`BodyContentError::EmptySegment`] with `container` set to
    /// `"head"` when the text lacks visible characters.
    pub fn from_text(text: impl Into<String>) -> Result<Self, BodyContentError> {
        let mut content = Vec::new();
        push_validated_text_segment(&mut content, text, "head")?;
        ensure_container_content(&content, "head")?;

        Ok(Self { content })
    }

    /// Returns the stored inline content.
    #[must_use]
    pub const fn content(&self) -> &[Inline] {
        self.content.as_slice()
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for division heading construction.

    use super::*;

    #[test]
    fn head_rejects_empty_text() {
        let result = Head::from_text("");
        assert!(matches!(
            result,
            Err(BodyContentError::EmptySegment { container }) if container == "head"
        ));
    }

    #[test]
    fn head_accepts_visible_text() {
        let head = Head::from_text("Chapter markers")
            .unwrap_or_else(|error| panic!("valid head: {error}"));
        assert_eq!(head.content(), &[Inline::Text("Chapter markers".into())]);
    }
}
