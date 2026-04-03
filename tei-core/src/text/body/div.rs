//! Division model for grouping related body content.
//!
//! Defines TEI `<div>` elements that organize paragraphs, utterances, and lists
//! into thematic sections identified by `@type` and optional `@xml:id`.

use crate::text::types::XmlId;

use super::{BodyContentError, List, P, Utterance, set_optional_identifier};
use serde::{Deserialize, Serialize};

/// Thematic or structural division of body content.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
#[serde(rename = "div")]
#[expect(
    clippy::struct_field_names,
    reason = "TEI @type attribute maps naturally to div_type"
)]
pub struct Div {
    #[serde(rename = "@type")]
    div_type: String,
    #[serde(
        rename = "@xml:id",
        alias = "@id",
        skip_serializing_if = "Option::is_none",
        default
    )]
    id: Option<XmlId>,
    #[serde(rename = "$value", default)]
    content: Vec<DivContent>,
}

impl Div {
    /// Builds a division with the specified type.
    ///
    /// # Errors
    ///
    /// Returns [`BodyContentError::EmptySegment`] when the type lacks visible
    /// characters after trimming.
    ///
    /// # Examples
    ///
    /// ```
    /// use tei_core::Div;
    ///
    /// let div = Div::new("show-notes")
    ///     .unwrap_or_else(|error| panic!("valid div: {error}"));
    /// assert_eq!(div.div_type(), "show-notes");
    /// ```
    pub fn new(div_type: impl Into<String>) -> Result<Self, BodyContentError> {
        let value = div_type.into();
        if value.trim().is_empty() {
            return Err(BodyContentError::EmptySegment {
                container: "div",
            });
        }

        Ok(Self {
            div_type: value,
            id: None,
            content: Vec::new(),
        })
    }

    /// Sets an `xml:id` attribute on the division.
    ///
    /// # Errors
    ///
    /// Returns [`BodyContentError::EmptyIdentifier`] when the identifier lacks
    /// visible characters. Returns
    /// [`BodyContentError::InvalidIdentifier`] when the identifier contains
    /// internal whitespace.
    pub fn set_id(&mut self, id: impl Into<String>) -> Result<(), BodyContentError> {
        set_optional_identifier(&mut self.id, id, "div")
    }

    /// Clears any associated `xml:id`.
    pub fn clear_id(&mut self) {
        self.id = None;
    }

    /// Returns the division identifier when present.
    #[must_use]
    #[expect(
        clippy::missing_const_for_fn,
        reason = "Option::as_ref is not const-stable on current MSRV."
    )]
    pub fn id(&self) -> Option<&XmlId> {
        self.id.as_ref()
    }

    /// Returns the division type.
    #[must_use]
    #[expect(
        clippy::missing_const_for_fn,
        reason = "String::as_str is not const-stable on current MSRV."
    )]
    pub fn div_type(&self) -> &str {
        self.div_type.as_str()
    }

    /// Returns the stored content.
    #[must_use]
    pub const fn content(&self) -> &[DivContent] {
        self.content.as_slice()
    }

    /// Appends a paragraph to the division.
    pub fn push_paragraph(&mut self, paragraph: P) {
        self.content.push(DivContent::Paragraph(paragraph));
    }

    /// Appends an utterance to the division.
    pub fn push_utterance(&mut self, utterance: Utterance) {
        self.content.push(DivContent::Utterance(utterance));
    }

    /// Appends a list to the division.
    pub fn push_list(&mut self, list: List) {
        self.content.push(DivContent::List(list));
    }

    /// Reports whether the division contains any content.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.content.is_empty()
    }
}

/// Content permitted inside a `<div>` element.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
pub enum DivContent {
    /// Paragraph block.
    #[serde(rename = "p")]
    Paragraph(P),
    /// Utterance block.
    #[serde(rename = "u")]
    Utterance(Utterance),
    /// List block.
    #[serde(rename = "list")]
    List(List),
}

#[cfg(test)]
mod tests {
    //! Unit tests for division construction and manipulation.

    use super::*;
    use crate::text::body::Item;

    #[test]
    fn div_new_rejects_empty_type() {
        let result = Div::new("");
        assert!(matches!(
            result,
            Err(BodyContentError::EmptySegment { container }) if container == "div"
        ));
    }

    #[test]
    fn div_new_accepts_valid_type() {
        let div = Div::new("show-notes")
            .unwrap_or_else(|error| panic!("valid div: {error}"));
        assert_eq!(div.div_type(), "show-notes");
        assert!(div.is_empty());
    }

    #[test]
    fn div_set_id_round_trips() {
        let mut div = Div::new("chapter")
            .unwrap_or_else(|error| panic!("valid div: {error}"));
        div.set_id("div1")
            .unwrap_or_else(|error| panic!("valid id: {error}"));
        assert_eq!(div.id().map(XmlId::as_str), Some("div1"));
        div.clear_id();
        assert!(div.id().is_none());
    }

    #[test]
    fn div_push_content() {
        let mut div = Div::new("show-notes")
            .unwrap_or_else(|error| panic!("valid div: {error}"));

        let paragraph = P::from_text_segments(["Intro text"])
            .unwrap_or_else(|error| panic!("valid paragraph: {error}"));
        div.push_paragraph(paragraph.clone());

        let utterance = Utterance::from_text_segments(Some("host"), ["Hello!"])
            .unwrap_or_else(|error| panic!("valid utterance: {error}"));
        div.push_utterance(utterance.clone());

        let item = Item::from_text_segments(["List item"])
            .unwrap_or_else(|error| panic!("valid item: {error}"));
        let list = super::List::new([item]);
        div.push_list(list.clone());

        assert_eq!(div.content().len(), 3);
        assert!(!div.is_empty());
        assert!(matches!(div.content()[0], DivContent::Paragraph(_)));
        assert!(matches!(div.content()[1], DivContent::Utterance(_)));
        assert!(matches!(div.content()[2], DivContent::List(_)));
    }
}
