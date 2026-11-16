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

/// Reads a string attribute from a `DivBlock`, naming the attribute in any failure.
fn div_string_attr(div: &Bound<'_, PyAny>, name: &str) -> Result<String> {
    div.getattr(name)
        .with_context(|| format!("DivBlock should expose {name}"))?
        .extract()
        .with_context(|| format!("{name} should be a string"))
}

fn assert_decoded_root_div(div: &Bound<'_, PyAny>) -> Result<()> {
    let div_type = div_string_attr(div, "div_type")?;
    ensure!(
        div_type == "show-notes",
        "expected show-notes div, found {div_type:?}"
    );

    let subtype = div_string_attr(div, "subtype")?;
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
    let div = decoded_root_div(episode, &types)?;
    assert_decoded_root_div(&div)?;

    let nested_div = decoded_nested_div(&div, &types)?;
    assert_decoded_nested_div(&nested_div)?;

    let list_block = decoded_list_block(&nested_div, &types)?;
    assert_decoded_list_item(&list_block, &types)
}

/// Expectations for one decoded block: its type and the messages that name it.
struct BlockExpectation<'a, 'py> {
    /// The struct type the block must be an instance of.
    expected_type: &'a Bound<'py, PyAny>,
    /// Failure context when the block is missing.
    missing: &'a str,
    /// Failure message when the block has the wrong type.
    wrong_type: &'a str,
}

impl<'py> BlockExpectation<'_, 'py> {
    /// Takes `block` if it is present, requiring it to be of the expected type.
    fn require(&self, block: PyResult<Bound<'py, PyAny>>) -> Result<Bound<'py, PyAny>> {
        let found = block.context(self.missing.to_owned())?;
        ensure_instance(&found, self.expected_type, self.wrong_type)?;
        Ok(found)
    }
}

/// Returns the episode's first body block, which must be a `DivBlock`.
fn decoded_root_div<'py>(
    episode: &Bound<'py, PyAny>,
    types: &DivStructTypes<'py>,
) -> Result<Bound<'py, PyAny>> {
    let blocks = episode
        .getattr("text")
        .context("Episode should expose text")?
        .getattr("body")
        .context("TeiText should expose body")?
        .getattr("blocks")
        .context("TeiBody should expose blocks")?;
    BlockExpectation {
        expected_type: &types.div_block,
        missing: "division block should exist",
        wrong_type: "top-level block should be a DivBlock",
    }
    .require(blocks.get_item(0))
}

/// Returns the second child of `div`, which must be a nested `DivBlock`.
fn decoded_nested_div<'py>(
    div: &Bound<'py, PyAny>,
    types: &DivStructTypes<'py>,
) -> Result<Bound<'py, PyAny>> {
    let content = div
        .getattr("content")
        .context("DivBlock should expose content")?;
    BlockExpectation {
        expected_type: &types.div_block,
        missing: "nested div should exist",
        wrong_type: "nested block should be a DivBlock",
    }
    .require(content.get_item(1))
}

/// Returns the first child of `nested_div`, which must be a `ListBlock`.
fn decoded_list_block<'py>(
    nested_div: &Bound<'py, PyAny>,
    types: &DivStructTypes<'py>,
) -> Result<Bound<'py, PyAny>> {
    let content = nested_div
        .getattr("content")
        .context("nested DivBlock should expose content")?;
    BlockExpectation {
        expected_type: &types.list_block,
        missing: "list block should exist",
        wrong_type: "nested content should be a ListBlock",
    }
    .require(content.get_item(0))
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
