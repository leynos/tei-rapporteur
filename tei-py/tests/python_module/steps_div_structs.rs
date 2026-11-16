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

    let nested_div = decoded_child(&div, ChildKind::NestedDiv, &types)?;
    assert_decoded_nested_div(&nested_div)?;

    let list_block = decoded_child(&nested_div, ChildKind::ListBlock, &types)?;
    assert_decoded_list_item(&list_block, &types)
}

/// Expectations for one decoded block: its type and the messages that name it.
struct BlockExpectation<'a, 'py> {
    /// Failure context when the parent exposes no `content`.
    content_missing: &'a str,
    /// Which child of `content` to take.
    index: usize,
    /// The struct type the block must be an instance of.
    expected_type: &'a Bound<'py, PyAny>,
    /// Failure context when the block is missing.
    missing: &'a str,
    /// Failure message when the block has the wrong type.
    wrong_type: &'a str,
}

impl<'py> BlockExpectation<'_, 'py> {
    /// Takes the expected child of `parent`'s `content`, requiring its type.
    fn child_of(&self, parent: &Bound<'py, PyAny>) -> Result<Bound<'py, PyAny>> {
        let content = parent
            .getattr("content")
            .context(self.content_missing.to_owned())?;
        let found = content
            .get_item(self.index)
            .context(self.missing.to_owned())?;
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
    let div = blocks.get_item(0).context("division block should exist")?;
    ensure_instance(
        &div,
        &types.div_block,
        "top-level block should be a DivBlock",
    )?;
    Ok(div)
}

/// The decoded children the steps look for under a division.
#[derive(Clone, Copy)]
enum ChildKind {
    /// The nested `DivBlock`, second child of the root division.
    NestedDiv,
    /// The `ListBlock`, first child of the nested division.
    ListBlock,
}

/// Returns the `kind` child of `parent`, which must be an instance of its struct type.
fn decoded_child<'py>(
    parent: &Bound<'py, PyAny>,
    kind: ChildKind,
    types: &DivStructTypes<'py>,
) -> Result<Bound<'py, PyAny>> {
    let expectation = match kind {
        ChildKind::NestedDiv => BlockExpectation {
            content_missing: "DivBlock should expose content",
            index: 1,
            expected_type: &types.div_block,
            missing: "nested div should exist",
            wrong_type: "nested block should be a DivBlock",
        },
        ChildKind::ListBlock => BlockExpectation {
            content_missing: "nested DivBlock should expose content",
            index: 0,
            expected_type: &types.list_block,
            missing: "list block should exist",
            wrong_type: "nested content should be a ListBlock",
        },
    };
    expectation.child_of(parent)
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
