//! Div-focused `msgspec.Struct` BDD steps for the Python module.
//!
//! These steps bind the div round-trip scenarios in
//! `tests/features/python_module.feature` to the shared Python module fixture.
//! They complement the Rust unit tests by asserting that BDD payloads preserve
//! div content through the public Python `Episode` struct path.

use super::state::{PythonModuleState, python_state};
use super::steps_structs::decode_episode;
use anyhow::{Context, Result, ensure};
use pyo3::{Bound, prelude::*};
use rstest_bdd_macros::{scenario, then};
use tei_py::test_support::{bootstrap_msgspec_attached, with_python};

fn first_inline_text(any: &Bound<'_, PyAny>) -> Result<String> {
    any.getattr("content")
        .context("content should exist")?
        .get_item(0)
        .context("first content item should exist")?
        .getattr("value")
        .context("inline text should expose value")?
        .extract()
        .context("inline value should be a string")
}

fn ensure_instance(
    any: &Bound<'_, PyAny>,
    expected_type: &Bound<'_, PyAny>,
    message: &str,
) -> Result<()> {
    ensure!(
        any.is_instance(expected_type)
            .context("msgspec.Struct type check should not raise")?,
        "{message}"
    );
    Ok(())
}

struct DivStructTypes<'py> {
    div_block: Bound<'py, PyAny>,
    list_block: Bound<'py, PyAny>,
    item: Bound<'py, PyAny>,
    label: Bound<'py, PyAny>,
}

fn div_struct_types<'py>(module: &Bound<'py, PyAny>) -> Result<DivStructTypes<'py>> {
    let structs = module
        .getattr("structs")
        .context("structs submodule should exist")?;
    Ok(DivStructTypes {
        div_block: structs
            .getattr("DivBlock")
            .context("DivBlock type should exist")?,
        list_block: structs
            .getattr("ListBlock")
            .context("ListBlock type should exist")?,
        item: structs.getattr("Item").context("Item type should exist")?,
        label: structs
            .getattr("Label")
            .context("Label type should exist")?,
    })
}

fn assert_decoded_root_div(div: &Bound<'_, PyAny>) -> Result<()> {
    let div_type: String = div
        .getattr("div_type")
        .context("DivBlock should expose div_type")?
        .extract()
        .context("div_type should be a string")?;
    ensure!(
        div_type == "show-notes",
        "expected show-notes div, found {div_type:?}"
    );

    let subtype: String = div
        .getattr("subtype")
        .context("DivBlock should expose subtype")?
        .extract()
        .context("subtype should be a string")?;
    ensure!(
        subtype == "chapter-markers",
        "expected chapter-markers subtype, found {subtype:?}"
    );

    let head = div.getattr("head").context("DivBlock should expose head")?;
    ensure!(
        first_inline_text(&head)? == "Chapter markers",
        "expected root division head to survive"
    );
    Ok(())
}

fn assert_decoded_nested_div(nested_div: &Bound<'_, PyAny>) -> Result<()> {
    let nested_head = nested_div
        .getattr("head")
        .context("nested DivBlock should expose head")?;
    ensure!(
        first_inline_text(&nested_head)? == "Guest bios",
        "expected nested division head to survive"
    );
    Ok(())
}

fn assert_decoded_list_item(
    list_block: &Bound<'_, PyAny>,
    types: &DivStructTypes<'_>,
) -> Result<()> {
    let item = list_block
        .getattr("items")
        .context("ListBlock should expose items")?
        .get_item(0)
        .context("item should exist")?;
    ensure_instance(&item, &types.item, "list content should be an Item")?;
    let label = item.getattr("label").context("Item should expose label")?;
    ensure_instance(&label, &types.label, "item label should be a Label")?;
    ensure!(
        first_inline_text(&label)? == "1.",
        "expected item label to survive"
    );
    ensure!(
        first_inline_text(&item)? == "Transcript",
        "expected item content to survive"
    );
    Ok(())
}

fn assert_decoded_div_blocks(module: &Bound<'_, PyAny>, episode: &Bound<'_, PyAny>) -> Result<()> {
    let types = div_struct_types(module)?;
    let blocks = episode
        .getattr("text")
        .context("Episode should expose text")?
        .getattr("body")
        .context("TeiText should expose body")?
        .getattr("blocks")
        .context("TeiBody should expose blocks")?;
    let div = blocks.get_item(0).context("division block should exist")?;
    ensure_instance(
        &div,
        &types.div_block,
        "top-level block should be a DivBlock",
    )?;
    assert_decoded_root_div(&div)?;

    let content = div
        .getattr("content")
        .context("DivBlock should expose content")?;
    let nested_div = content.get_item(1).context("nested div should exist")?;
    ensure_instance(
        &nested_div,
        &types.div_block,
        "nested block should be a DivBlock",
    )?;
    assert_decoded_nested_div(&nested_div)?;

    let list_block = nested_div
        .getattr("content")
        .context("nested DivBlock should expose content")?
        .get_item(0)
        .context("list block should exist")?;
    ensure_instance(
        &list_block,
        &types.list_block,
        "nested content should be a ListBlock",
    )?;
    assert_decoded_list_item(&list_block, &types)
}

#[then("the DivBlock, nested DivBlock, ListBlock, Item, and Label text are preserved")]
pub(super) fn the_div_blocks_are_preserved(
    #[from(python_state)] state: &PythonModuleState,
) -> Result<()> {
    let payload = state.msgpack_payload()?;

    with_python(|py| {
        ensure!(
            bootstrap_msgspec_attached(py),
            "msgspec bootstrap should succeed for Episode div round-trip tests"
        );
        state.with_module(py, |module| {
            let episode = decode_episode(py, &module, &payload)
                .context("MessagePack payload should decode to an Episode")?;
            assert_decoded_div_blocks(&module, &episode)
        })
    })
}

/// Scenario: Round-trip a div-containing Document through the Python Episode struct.
#[scenario(
    path = "tests/features/python_module.feature",
    name = "Round-trip MessagePack via Episode struct with div blocks"
)]
pub fn round_trips_div_blocks_via_episode_struct(
    #[from(python_state)] _python_state: PythonModuleState,
) {
}
