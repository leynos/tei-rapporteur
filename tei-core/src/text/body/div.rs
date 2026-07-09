//! Division model for grouping related body content.
//!
//! Defines TEI `<div>` elements that organize paragraphs, utterances, lists,
//! nested divisions, and optional headings into thematic sections identified by
//! `@type`, optional `@subtype`, and optional `@xml:id`.

use crate::text::types::{DivSubtype, DivType, XmlId};

use super::{BodyContentError, Head, List, P, Utterance, set_optional_identifier};
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
    div_type: DivType,
    #[serde(rename = "@subtype", skip_serializing_if = "Option::is_none", default)]
    subtype: Option<DivSubtype>,
    #[serde(
        rename = "@xml:id",
        alias = "@id",
        skip_serializing_if = "Option::is_none",
        default
    )]
    id: Option<XmlId>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    head: Option<Head>,
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
        let validated = DivType::new(div_type)
            .map_err(|_| BodyContentError::EmptySegment { container: "div" })?;

        Ok(Self {
            div_type: validated,
            subtype: None,
            id: None,
            head: None,
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

    /// Sets the optional division subtype, overwriting any existing value.
    ///
    /// # Errors
    ///
    /// Returns [`BodyContentError::EmptySegment`] when the value lacks visible
    /// characters after trimming.
    pub fn set_subtype(&mut self, subtype: impl Into<String>) -> Result<(), BodyContentError> {
        self.subtype = Some(
            DivSubtype::new(subtype)
                .map_err(|_| BodyContentError::EmptySegment { container: "div" })?,
        );
        Ok(())
    }

    /// Sets the optional heading for the division, overwriting any existing
    /// value.
    pub fn set_head(&mut self, head: Head) {
        self.head = Some(head);
    }

    /// Clears any associated `xml:id`.
    pub fn clear_id(&mut self) {
        self.id = None;
    }

    /// Clears any associated subtype.
    pub fn clear_subtype(&mut self) {
        self.subtype = None;
    }

    /// Clears any associated heading.
    pub fn clear_head(&mut self) {
        self.head = None;
    }

    /// Returns the division identifier when present.
    #[must_use]
    pub const fn id(&self) -> Option<&XmlId> {
        self.id.as_ref()
    }

    /// Returns the division type.
    #[must_use]
    pub const fn div_type(&self) -> &str {
        self.div_type.as_str()
    }

    /// Returns the division subtype when present.
    #[must_use]
    pub fn subtype(&self) -> Option<&str> {
        self.subtype.as_ref().map(DivSubtype::as_str)
    }

    /// Returns the heading when present.
    #[must_use]
    pub const fn head(&self) -> Option<&Head> {
        self.head.as_ref()
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

    /// Appends a nested division to the division.
    pub fn push_div(&mut self, div: Self) {
        self.content.push(DivContent::Div(div));
    }

    /// Reports whether the division contains any content.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.head.is_none() && self.content.is_empty()
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
    /// Nested division block.
    #[serde(rename = "div")]
    Div(Div),
}

#[cfg(test)]
mod tests {
    //! Unit tests for division construction and manipulation.

    use super::*;
    use crate::text::body::Item;

    fn sample_list() -> Result<List, BodyContentError> {
        let item = Item::from_text_segments(["List item"])?;
        List::new([item])
    }

    fn sample_child_div() -> Result<Div, BodyContentError> {
        let mut child_div = Div::new("guest-bios")?;
        child_div.set_head(Head::from_text("Guests")?);
        Ok(child_div)
    }

    fn div_content_kinds(div: &Div) -> Vec<&'static str> {
        div.content()
            .iter()
            .map(|content| match content {
                DivContent::Paragraph(_) => "p",
                DivContent::Utterance(_) => "u",
                DivContent::List(_) => "list",
                DivContent::Div(_) => "div",
            })
            .collect()
    }

    #[test]
    fn div_new_rejects_empty_type() {
        let result = Div::new("");
        assert!(matches!(
            result,
            Err(BodyContentError::EmptySegment { container }) if container == "div"
        ));
    }

    #[test]
    fn div_new_rejects_whitespace_only_type() {
        let result = Div::new("   ");
        assert!(matches!(
            result,
            Err(BodyContentError::EmptySegment { container }) if container == "div"
        ));
    }

    #[test]
    fn div_new_trims_type() {
        let div = Div::new("  chapter  ").unwrap_or_else(|error| panic!("valid div: {error}"));
        assert_eq!(div.div_type(), "chapter");
    }

    #[test]
    fn div_new_accepts_valid_type() {
        let div = Div::new("show-notes").unwrap_or_else(|error| panic!("valid div: {error}"));
        assert_eq!(div.div_type(), "show-notes");
        assert!(div.is_empty());
    }

    #[test]
    fn div_deserialization_rejects_empty_type() {
        let json = r#"{"@type":"","$value":[]}"#;
        let result: Result<Div, _> = tei_serde::serde_json::from_str(json);
        assert!(
            result.is_err(),
            "empty @type must be rejected at deserialization"
        );
    }

    #[test]
    fn div_deserialization_rejects_whitespace_type() {
        let json = r#"{"@type":"   ","$value":[]}"#;
        let result: Result<Div, _> = tei_serde::serde_json::from_str(json);
        assert!(
            result.is_err(),
            "whitespace-only @type must be rejected at deserialization"
        );
    }

    #[test]
    fn div_set_id_round_trips() {
        let mut div = Div::new("chapter").unwrap_or_else(|error| panic!("valid div: {error}"));
        div.set_id("div1")
            .unwrap_or_else(|error| panic!("valid id: {error}"));
        assert_eq!(div.id().map(XmlId::as_str), Some("div1"));
        div.clear_id();
        assert!(div.id().is_none());
    }

    #[test]
    fn div_subtype_round_trips() {
        let mut div = Div::new("segment").unwrap_or_else(|error| panic!("valid div: {error}"));
        div.set_subtype("chapter-marker")
            .unwrap_or_else(|error| panic!("valid subtype: {error}"));
        assert_eq!(div.subtype(), Some("chapter-marker"));
        div.clear_subtype();
        assert!(div.subtype().is_none());
    }

    #[test]
    fn div_head_round_trips() {
        let mut div = Div::new("segment").unwrap_or_else(|error| panic!("valid div: {error}"));
        let head = Head::from_text("Chapter markers")
            .unwrap_or_else(|error| panic!("valid head: {error}"));
        div.set_head(head.clone());
        assert_eq!(div.head(), Some(&head));
        div.clear_head();
        assert!(div.head().is_none());
    }

    #[test]
    fn div_push_content() {
        let mut div = Div::new("show-notes").unwrap_or_else(|error| panic!("valid div: {error}"));

        let paragraph = P::from_text_segments(["Intro text"])
            .unwrap_or_else(|error| panic!("valid paragraph: {error}"));
        div.push_paragraph(paragraph);

        let utterance = Utterance::from_text_segments(Some("host"), ["Hello!"])
            .unwrap_or_else(|error| panic!("valid utterance: {error}"));
        div.push_utterance(utterance);

        div.push_list(sample_list().expect("valid list"));
        div.push_div(sample_child_div().expect("valid child div"));
        assert_eq!(div_content_kinds(&div), vec!["p", "u", "list", "div"]);
        assert!(!div.is_empty());
    }

    #[test]
    fn div_serde_round_trips_head_subtype_and_nested_div() {
        let mut parent = Div::new("segment").unwrap_or_else(|error| panic!("valid div: {error}"));
        parent
            .set_subtype("chapter-markers")
            .unwrap_or_else(|error| panic!("valid subtype: {error}"));
        parent.set_head(
            Head::from_text("Chapter markers")
                .unwrap_or_else(|error| panic!("valid head: {error}")),
        );

        let mut child =
            Div::new("segment").unwrap_or_else(|error| panic!("valid child div: {error}"));
        child
            .set_subtype("chapter-marker")
            .unwrap_or_else(|error| panic!("valid subtype: {error}"));
        child.set_head(
            Head::from_text("Cold open")
                .unwrap_or_else(|error| panic!("valid child head: {error}")),
        );
        child.push_paragraph(
            P::from_text_segments(["Welcome back"])
                .unwrap_or_else(|error| panic!("valid paragraph: {error}")),
        );
        parent.push_div(child);

        let json = tei_serde::serde_json::to_string(&parent)
            .unwrap_or_else(|error| panic!("json: {error}"));
        let parsed: Div = tei_serde::serde_json::from_str(&json)
            .unwrap_or_else(|error| panic!("round trip should parse: {error}"));

        assert_eq!(parsed, parent);
    }
}
