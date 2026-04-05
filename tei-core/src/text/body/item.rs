//! List item and label models for structured enumeration.
//!
//! Defines TEI `<item>` and `<label>` elements with inline content, identifiers,
//! and cross-reference attributes.

use crate::text::{
    Inline,
    types::{PointerList, XmlId},
};

use super::{
    BodyContentError, ensure_container_content, push_validated_inline, push_validated_text_segment,
    set_optional_identifier,
};
use serde::{Deserialize, Serialize};

/// Label prefix for a list item, containing inline content.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
#[serde(rename = "label")]
pub struct Label {
    #[serde(rename = "$value", default)]
    content: Vec<Inline>,
}

impl Label {
    /// Builds a label from pre-constructed inline content.
    ///
    /// # Errors
    ///
    /// Returns [`BodyContentError::EmptyContent`] when the content lacks
    /// visible inline information.
    pub fn new(content: impl IntoIterator<Item = Inline>) -> Result<Self, BodyContentError> {
        let collected: Vec<Inline> = content.into_iter().collect();
        ensure_container_content(&collected, "label")?;

        Ok(Self { content: collected })
    }

    /// Builds a label from plain text.
    ///
    /// # Errors
    ///
    /// Returns [`BodyContentError::EmptyContent`] when the text lacks visible
    /// characters.
    pub fn from_text(text: impl Into<String>) -> Result<Self, BodyContentError> {
        let mut content = Vec::new();
        push_validated_text_segment(&mut content, text, "label")?;
        ensure_container_content(&content, "label")?;

        Ok(Self { content })
    }

    /// Returns the stored inline content.
    #[must_use]
    pub const fn content(&self) -> &[Inline] {
        self.content.as_slice()
    }
}

/// Single list item, optionally prefixed by a label.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
#[serde(rename = "item", deny_unknown_fields)]
pub struct Item {
    #[serde(
        rename = "@xml:id",
        alias = "@id",
        skip_serializing_if = "Option::is_none",
        default
    )]
    id: Option<XmlId>,
    #[serde(rename = "@n", skip_serializing_if = "Option::is_none", default)]
    n: Option<String>,
    #[serde(rename = "@corresp", skip_serializing_if = "Option::is_none", default)]
    #[cfg_attr(feature = "json-schema", schemars(with = "Option<String>"))]
    corresp: Option<PointerList>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    label: Option<Label>,
    #[serde(rename = "$value", default)]
    content: Vec<Inline>,
}

impl Item {
    /// Builds an item from pre-constructed inline content.
    ///
    /// # Errors
    ///
    /// Returns [`BodyContentError::EmptyContent`] when the content lacks
    /// visible inline information.
    pub fn new(content: impl IntoIterator<Item = Inline>) -> Result<Self, BodyContentError> {
        let collected: Vec<Inline> = content.into_iter().collect();
        ensure_container_content(&collected, "item")?;

        Ok(Self {
            id: None,
            n: None,
            corresp: None,
            label: None,
            content: collected,
        })
    }

    /// Builds an item from text segments, validating inline content.
    ///
    /// # Errors
    ///
    /// Returns [`BodyContentError::EmptyContent`] when no segments contain
    /// visible characters.
    pub fn from_text_segments<S>(
        segments: impl IntoIterator<Item = S>,
    ) -> Result<Self, BodyContentError>
    where
        S: Into<String>,
    {
        let mut content = Vec::new();
        for segment in segments {
            push_validated_text_segment(&mut content, segment, "item")?;
        }
        ensure_container_content(&content, "item")?;

        Ok(Self {
            id: None,
            n: None,
            corresp: None,
            label: None,
            content,
        })
    }

    /// Sets an `xml:id` attribute on the item.
    ///
    /// # Errors
    ///
    /// Returns [`BodyContentError::EmptyIdentifier`] when the identifier lacks
    /// visible characters. Returns
    /// [`BodyContentError::InvalidIdentifier`] when the identifier contains
    /// internal whitespace.
    pub fn set_id(&mut self, id: impl Into<String>) -> Result<(), BodyContentError> {
        set_optional_identifier(&mut self.id, id, "item")
    }

    /// Clears any associated `xml:id`.
    pub fn clear_id(&mut self) {
        self.id = None;
    }

    /// Returns the item identifier when present.
    #[must_use]
    #[expect(
        clippy::missing_const_for_fn,
        reason = "Option::as_ref is not const-stable on current MSRV."
    )]
    pub fn id(&self) -> Option<&XmlId> {
        self.id.as_ref()
    }

    /// Sets the `@n` attribute (numbering/labelling).
    ///
    /// # Errors
    ///
    /// Returns [`BodyContentError::EmptySegment`] when the value lacks visible
    /// characters.
    pub fn set_n(&mut self, n: impl Into<String>) -> Result<(), BodyContentError> {
        let value = n.into();
        if value.trim().is_empty() {
            return Err(BodyContentError::EmptySegment { container: "item" });
        }
        self.n = Some(value);
        Ok(())
    }

    /// Returns the `@n` attribute when present.
    #[must_use]
    pub const fn n(&self) -> Option<&str> {
        match &self.n {
            Some(value) => Some(value.as_str()),
            None => None,
        }
    }

    /// Sets the `@corresp` pointer list attribute.
    pub fn set_corresp(&mut self, corresp: PointerList) {
        self.corresp = Some(corresp);
    }

    /// Returns the `@corresp` pointer list when present.
    #[must_use]
    #[expect(
        clippy::missing_const_for_fn,
        reason = "Option::as_ref is not const-stable on current MSRV."
    )]
    pub fn corresp(&self) -> Option<&PointerList> {
        self.corresp.as_ref()
    }

    /// Sets the label prefix for this item.
    pub fn set_label(&mut self, label: Label) {
        self.label = Some(label);
    }

    /// Clears any associated label.
    pub fn clear_label(&mut self) {
        self.label = None;
    }

    /// Returns the label when present.
    #[must_use]
    #[expect(
        clippy::missing_const_for_fn,
        reason = "Option::as_ref is not const-stable on current MSRV."
    )]
    pub fn label(&self) -> Option<&Label> {
        self.label.as_ref()
    }

    /// Returns the stored inline content.
    #[must_use]
    pub const fn content(&self) -> &[Inline] {
        self.content.as_slice()
    }

    /// Appends a new segment.
    ///
    /// # Errors
    ///
    /// Returns [`BodyContentError::EmptySegment`] when the segment lacks
    /// visible characters.
    pub fn push_segment<S>(&mut self, segment: S) -> Result<(), BodyContentError>
    where
        S: Into<String>,
    {
        push_validated_text_segment(&mut self.content, segment, "item")
    }

    /// Appends a new inline node.
    ///
    /// # Errors
    ///
    /// Returns [`BodyContentError::EmptySegment`] when the inline text lacks
    /// visible characters. Returns [`BodyContentError::EmptyContent`] when the
    /// inline element has no meaningful children.
    pub fn push_inline(&mut self, inline: Inline) -> Result<(), BodyContentError> {
        push_validated_inline(&mut self.content, inline, "item")
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for item and label construction and validation.

    use super::*;
    use crate::text::types::PointerList;

    #[test]
    fn label_rejects_empty_content() {
        let result = Label::new(Vec::<Inline>::new());
        assert!(matches!(
            result,
            Err(BodyContentError::EmptyContent { container }) if container == "label"
        ));
    }

    #[test]
    fn label_from_text_constructs_inline() {
        let label =
            Label::from_text("Link:").unwrap_or_else(|error| panic!("valid label: {error}"));
        assert_eq!(label.content(), [Inline::text("Link:")]);
    }

    #[test]
    fn item_rejects_empty_content() {
        let result = Item::from_text_segments(Vec::<String>::new());
        assert!(matches!(
            result,
            Err(BodyContentError::EmptyContent { container }) if container == "item"
        ));
    }

    #[test]
    fn item_set_id_round_trips() {
        let mut item = Item::from_text_segments(["Visit our website"])
            .unwrap_or_else(|error| panic!("valid item: {error}"));
        item.set_id("item1")
            .unwrap_or_else(|error| panic!("valid id: {error}"));
        assert_eq!(item.id().map(XmlId::as_str), Some("item1"));
        item.clear_id();
        assert!(item.id().is_none());
    }

    #[test]
    fn item_set_n_attribute() {
        let mut item = Item::from_text_segments(["First item"])
            .unwrap_or_else(|error| panic!("valid item: {error}"));
        item.set_n("1")
            .unwrap_or_else(|error| panic!("valid n: {error}"));
        assert_eq!(item.n(), Some("1"));
    }

    #[test]
    fn item_set_corresp_attribute() {
        let mut item = Item::from_text_segments(["Bio summary"])
            .unwrap_or_else(|error| panic!("valid item: {error}"));
        let corresp = PointerList::new(["#guest1", "#guest2"])
            .unwrap_or_else(|error| panic!("valid pointer list: {error}"));
        item.set_corresp(corresp.clone());
        assert_eq!(item.corresp(), Some(&corresp));
    }

    #[test]
    fn item_set_label() {
        let mut item = Item::from_text_segments(["Visit our website"])
            .unwrap_or_else(|error| panic!("valid item: {error}"));
        let label =
            Label::from_text("Link:").unwrap_or_else(|error| panic!("valid label: {error}"));
        item.set_label(label.clone());
        assert_eq!(item.label(), Some(&label));
        item.clear_label();
        assert!(item.label().is_none());
    }
}
