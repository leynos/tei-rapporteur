//! List model for ordered and unordered sequences of items.
//!
//! Defines the TEI `<list>` element containing `<item>` children with optional
//! identifiers.

use crate::text::types::XmlId;

use super::{BodyContentError, Item, set_optional_identifier};
use serde::{Deserialize, Serialize};

/// Ordered or unordered list of items.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
#[serde(rename = "list")]
pub struct List {
    #[serde(
        rename = "@xml:id",
        alias = "@id",
        skip_serializing_if = "Option::is_none",
        default
    )]
    id: Option<XmlId>,
    #[serde(rename = "$value", default)]
    items: Vec<Item>,
}

impl List {
    /// Builds a list from the provided items.
    ///
    /// # Errors
    ///
    /// Returns [`BodyContentError::EmptyContent`] when the list has no items.
    ///
    /// # Examples
    ///
    /// ```
    /// use tei_core::{Item, List};
    ///
    /// let item1 = Item::from_text_segments(["First item"])
    ///     .unwrap_or_else(|error| panic!("valid item: {error}"));
    /// let item2 = Item::from_text_segments(["Second item"])
    ///     .unwrap_or_else(|error| panic!("valid item: {error}"));
    ///
    /// let list = List::new([item1, item2])
    ///     .unwrap_or_else(|error| panic!("valid list: {error}"));
    /// assert_eq!(list.items().len(), 2);
    /// ```
    pub fn new(items: impl IntoIterator<Item = Item>) -> Result<Self, BodyContentError> {
        let collected: Vec<Item> = items.into_iter().collect();
        if collected.is_empty() {
            return Err(BodyContentError::EmptyContent { container: "list" });
        }

        Ok(Self {
            id: None,
            items: collected,
        })
    }

    /// Sets an `xml:id` attribute on the list.
    ///
    /// # Errors
    ///
    /// Returns [`BodyContentError::EmptyIdentifier`] when the identifier lacks
    /// visible characters. Returns
    /// [`BodyContentError::InvalidIdentifier`] when the identifier contains
    /// internal whitespace.
    pub fn set_id(&mut self, id: impl Into<String>) -> Result<(), BodyContentError> {
        set_optional_identifier(&mut self.id, id, "list")
    }

    /// Clears any associated `xml:id`.
    pub fn clear_id(&mut self) {
        self.id = None;
    }

    /// Returns the list identifier when present.
    #[must_use]
    #[expect(
        clippy::missing_const_for_fn,
        reason = "Option::as_ref is not const-stable on current MSRV."
    )]
    pub fn id(&self) -> Option<&XmlId> {
        self.id.as_ref()
    }

    /// Returns the stored items.
    #[must_use]
    pub const fn items(&self) -> &[Item] {
        self.items.as_slice()
    }

    /// Appends a new item to the list.
    pub fn push_item(&mut self, item: Item) {
        self.items.push(item);
    }

    /// Reports whether the list contains any items.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for list construction and manipulation.

    use super::*;

    #[test]
    fn list_new_constructs_from_items() {
        let item1 = Item::from_text_segments(["First"])
            .unwrap_or_else(|error| panic!("valid item: {error}"));
        let item2 = Item::from_text_segments(["Second"])
            .unwrap_or_else(|error| panic!("valid item: {error}"));

        let list = List::new([item1.clone(), item2.clone()])
            .unwrap_or_else(|error| panic!("valid list: {error}"));
        assert_eq!(list.items(), [item1, item2]);
        assert!(!list.is_empty());
    }

    #[test]
    fn list_new_rejects_empty_items() {
        let result = List::new(Vec::<Item>::new());
        assert!(matches!(
            result,
            Err(BodyContentError::EmptyContent { container }) if container == "list"
        ));
    }

    #[test]
    fn list_set_id_round_trips() {
        let item = Item::from_text_segments(["Only"])
            .unwrap_or_else(|error| panic!("valid item: {error}"));
        let list_instance = List::new([item]).unwrap_or_else(|error| panic!("valid list: {error}"));
        let mut list = list_instance;
        list.set_id("list1")
            .unwrap_or_else(|error| panic!("valid id: {error}"));
        assert_eq!(list.id().map(XmlId::as_str), Some("list1"));
        list.clear_id();
        assert!(list.id().is_none());
    }

    #[test]
    fn list_push_item() {
        let first = Item::from_text_segments(["First item"])
            .unwrap_or_else(|error| panic!("valid item: {error}"));
        let mut list =
            List::new([first.clone()]).unwrap_or_else(|error| panic!("valid list: {error}"));
        let item = Item::from_text_segments(["New item"])
            .unwrap_or_else(|error| panic!("valid item: {error}"));
        list.push_item(item.clone());
        assert_eq!(list.items(), [first, item]);
    }
}
