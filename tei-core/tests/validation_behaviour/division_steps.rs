//! Division-related `when` steps for the validation behaviour feature.
//!
//! Split from the parent module to keep it within the module size ceiling;
//! these steps reuse the parent's shared `ValidationState` fixture.

use anyhow::{Context, Result};
use rstest_bdd_macros::when;
use tei_core::{Div, Item, List, TeiDocument};

use super::ValidationState;

#[when("I add a division \"{div_type}\" containing an item with id \"{identifier}\"")]
fn i_add_a_division_containing_an_item_with_id(
    #[from(validated_state)] state: &ValidationState,
    div_type: String,
    identifier: String,
) -> Result<()> {
    state.update_document(|document| {
        let mut item =
            Item::from_text_segments(["Linked resource"]).context("item should be valid")?;
        item.set_id(identifier.as_str())
            .context("identifier should validate")?;
        let list = List::new([item]).context("list should be valid")?;
        let mut div = Div::new(div_type.as_str()).context("division should be valid")?;
        div.push_list(list);
        let mut text = document.text().clone();
        text.body_mut().push_div(div);
        Ok(TeiDocument::new(document.header().clone(), text))
    })
}

#[when("I add a division \"{div_type}\" containing an item with corresp \"{corresp}\"")]
fn i_add_a_division_containing_an_item_with_corresp(
    #[from(validated_state)] state: &ValidationState,
    div_type: String,
    corresp: String,
) -> Result<()> {
    state.update_document(|document| {
        let mut item =
            Item::from_text_segments(["External reference"]).context("item should be valid")?;
        item.set_corresp(tei_core::PointerList::new([corresp.as_str()])?);
        let list = List::new([item]).context("list should be valid")?;
        let mut div = Div::new(div_type.as_str()).context("division should be valid")?;
        div.push_list(list);
        let mut text = document.text().clone();
        text.body_mut().push_div(div);
        Ok(TeiDocument::new(document.header().clone(), text))
    })
}

#[when("I add a nested division \"{div_type}\" containing a child item with id \"{identifier}\"")]
fn i_add_a_nested_division_containing_an_item_with_id(
    #[from(validated_state)] state: &ValidationState,
    div_type: String,
    identifier: String,
) -> Result<()> {
    state.update_document(|document| {
        let mut item =
            Item::from_text_segments(["Nested resource"]).context("item should be valid")?;
        item.set_id(identifier.as_str())
            .context("identifier should validate")?;
        let list = List::new([item]).context("list should be valid")?;
        let mut child = Div::new(div_type.as_str()).context("division should be valid")?;
        child.push_list(list);
        let mut parent = Div::new("parent").context("parent division should be valid")?;
        parent.push_div(child);
        let mut text = document.text().clone();
        text.body_mut().push_div(parent);
        Ok(TeiDocument::new(document.header().clone(), text))
    })
}

#[when("I add a nested division \"{div_type}\" containing a child item with corresp \"{corresp}\"")]
fn i_add_a_nested_division_containing_an_item_with_corresp(
    #[from(validated_state)] state: &ValidationState,
    div_type: String,
    corresp: String,
) -> Result<()> {
    state.update_document(|document| {
        let mut item =
            Item::from_text_segments(["Nested reference"]).context("item should be valid")?;
        item.set_corresp(tei_core::PointerList::new([corresp.as_str()])?);
        let list = List::new([item]).context("list should be valid")?;
        let mut child = Div::new(div_type.as_str()).context("division should be valid")?;
        child.push_list(list);
        let mut parent = Div::new("parent").context("parent division should be valid")?;
        parent.push_div(child);
        let mut text = document.text().clone();
        text.body_mut().push_div(parent);
        Ok(TeiDocument::new(document.header().clone(), text))
    })
}
