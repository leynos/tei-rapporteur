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
use tei_core::{BodyBlock, DivContent, Inline, TeiDocument};
use tei_py::projection::PyTeiDocument;
use tei_py::test_support::{ensure_msgspec_available, with_python};
use tei_serde::msgpack::from_slice;

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

fn assert_decoded_div_blocks(episode: &Bound<'_, PyAny>) -> Result<()> {
    let blocks = episode
        .getattr("text")
        .context("Episode should expose text")?
        .getattr("body")
        .context("TeiText should expose body")?
        .getattr("blocks")
        .context("TeiBody should expose blocks")?;
    let div = blocks.get_item(0).context("division block should exist")?;

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

    let content = div
        .getattr("content")
        .context("DivBlock should expose content")?;
    let nested_div = content.get_item(1).context("nested div should exist")?;
    let nested_head = nested_div
        .getattr("head")
        .context("nested DivBlock should expose head")?;
    ensure!(
        first_inline_text(&nested_head)? == "Guest bios",
        "expected nested division head to survive"
    );

    let list_block = nested_div
        .getattr("content")
        .context("nested DivBlock should expose content")?
        .get_item(0)
        .context("list block should exist")?;
    let item = list_block
        .getattr("items")
        .context("ListBlock should expose items")?
        .get_item(0)
        .context("item should exist")?;
    let label = item.getattr("label").context("Item should expose label")?;
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

fn core_inline_text(content: &[Inline]) -> Result<&str> {
    match content
        .first()
        .context("content should include inline text")?
    {
        Inline::Text(value) => Ok(value.as_str()),
        _ => anyhow::bail!("expected first inline node to be text"),
    }
}

fn root_div(document: &TeiDocument) -> Result<&tei_core::Div> {
    let block = document
        .text()
        .body()
        .blocks()
        .first()
        .context("document should contain a body block")?;

    match block {
        BodyBlock::Div(div) => Ok(div),
        _ => anyhow::bail!("top-level body block should be a division"),
    }
}

fn nested_div(div: &tei_core::Div) -> Result<&tei_core::Div> {
    match div
        .content()
        .get(1)
        .context("document should contain a nested division")?
    {
        DivContent::Div(d) => Ok(d),
        _ => anyhow::bail!("second division content item should be a nested division"),
    }
}

fn nested_list(div: &tei_core::Div) -> Result<&tei_core::List> {
    match div
        .content()
        .first()
        .context("nested division should contain a list")?
    {
        DivContent::List(l) => Ok(l),
        _ => anyhow::bail!("nested division content should be a list"),
    }
}

fn assert_core_root_div(div: &tei_core::Div) -> Result<()> {
    ensure!(
        div.div_type() == "show-notes",
        "division type should survive"
    );
    ensure!(
        div.subtype() == Some("chapter-markers"),
        "division subtype should survive"
    );
    ensure!(
        div.head()
            .map(|head| core_inline_text(head.content()))
            .transpose()?
            == Some("Chapter markers"),
        "division head should survive"
    );
    Ok(())
}

fn assert_core_nested_div(nested_div: &tei_core::Div) -> Result<()> {
    ensure!(
        nested_div
            .head()
            .map(|head| core_inline_text(head.content()))
            .transpose()?
            == Some("Guest bios"),
        "nested division head should survive"
    );
    Ok(())
}

fn assert_core_list(list: &tei_core::List) -> Result<()> {
    let item = list
        .items()
        .first()
        .context("list should contain an item")?;
    ensure!(
        item.label()
            .map(|label| core_inline_text(label.content()))
            .transpose()?
            == Some("1."),
        "list item label should survive"
    );
    ensure!(
        core_inline_text(item.content())? == "Transcript",
        "list item content should survive"
    );

    Ok(())
}

fn assert_core_div_blocks(document: &TeiDocument) -> Result<()> {
    let div = root_div(document)?;
    assert_core_root_div(div)?;

    let child_div = nested_div(div)?;
    assert_core_nested_div(child_div)?;
    assert_core_list(nested_list(child_div)?)
}

#[then("the DivBlock, nested DivBlock, ListBlock, Item, and Label text are preserved")]
pub(super) fn the_div_blocks_are_preserved(
    #[from(python_state)] state: &PythonModuleState,
) -> Result<()> {
    let payload = state.msgpack_payload()?;
    if !ensure_msgspec_available() {
        let projection: PyTeiDocument =
            from_slice(&payload).context("fallback decoding MessagePack document")?;
        let document: TeiDocument = TeiDocument::try_from(projection)
            .context("projection should convert to TeiDocument")?;
        return assert_core_div_blocks(&document);
    }

    with_python(|py| {
        state.with_module(py, |module| {
            let episode = decode_episode(py, &module, &payload)
                .context("MessagePack payload should decode to an Episode")?;
            assert_decoded_div_blocks(&episode)
        })
    })
}

/// Scenario: Round-trip a div-containing Document through the Python Episode struct.
#[scenario(
    path = "tests/features/python_module.feature",
    name = "Round-trip MessagePack via Episode struct with div blocks"
)]
#[expect(
    unused_variables,
    reason = "rstest-bdd injects the state fixture into generated step calls"
)]
pub fn round_trips_div_blocks_via_episode_struct(python_state: PythonModuleState) {}
