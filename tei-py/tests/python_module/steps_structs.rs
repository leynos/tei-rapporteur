//! Steps covering the `msgspec.Struct` projections exposed to Python.

use anyhow::{Context, Result, ensure};
use pyo3::{Bound, prelude::*, types::PyDict};
use rstest_bdd_macros::{scenario, then, when};
use serde::Deserialize;
use super::state::{PythonModuleState, python_state};
use tei_core::{BodyBlock, DivContent, FileDesc, Inline, TeiDocument, TeiHeader};
use tei_py::projection::PyTeiDocument;
use tei_py::test_support::try_ensure_msgspec_installed;
use tei_serde::json::Value;
use tei_serde::msgpack::{from_slice, to_vec_named};

const _: fn() -> PythonModuleState = python_state;

fn retitle_document(document: &TeiDocument, title: &str) -> Result<TeiDocument> {
    let old_header = document.header().clone();
    let old_file_desc = old_header.file_desc().clone();

    let mut new_file_desc =
        FileDesc::from_title_str(title).context("fallback title must be valid")?;

    if let Some(series) = old_file_desc.series() {
        new_file_desc = new_file_desc.with_series(series);
    }

    if let Some(synopsis) = old_file_desc.synopsis() {
        new_file_desc = new_file_desc.with_synopsis(synopsis);
    }

    let mut header = TeiHeader::new(new_file_desc);

    if let Some(profile) = old_header.profile_desc().cloned() {
        header = header.with_profile_desc(profile);
    }

    if let Some(encoding) = old_header.encoding_desc().cloned() {
        header = header.with_encoding_desc(encoding);
    }

    if let Some(revision) = old_header.revision_desc().cloned() {
        header = header.with_revision_desc(revision);
    }

    Ok(TeiDocument::new(header, document.text().clone()))
}

fn decode_episode<'py>(
    py: Python<'py>,
    module: &Bound<'py, PyAny>,
    payload: &[u8],
) -> Result<Bound<'py, PyAny>> {
    let structs = module
        .getattr("structs")
        .context("structs submodule should exist")?;
    let episode_type = structs
        .getattr("Episode")
        .context("Episode class should be exported")?;
    let msgpack = py
        .import("msgspec.msgpack")
        .context("msgspec.msgpack import should succeed")?;
    let kwargs = PyDict::new(py);
    kwargs.set_item("type", episode_type)?;

    Ok(msgpack
        .getattr("decode")
        .context("decode function should exist")?
        .call((payload,), Some(&kwargs))?)
}

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

struct ContentQuery {
    index: usize,
    missing_msg: &'static str,
    mismatch_msg: &'static str,
}
fn extract_div_content<'a, T>(
    div: &'a tei_core::Div,
    query: ContentQuery,
    extract: impl Fn(&'a DivContent) -> Option<T>,
) -> Result<T> {
    let item = div.content().get(query.index).context(query.missing_msg)?;
    extract(item).ok_or_else(|| anyhow::anyhow!("{}", query.mismatch_msg))
}
fn nested_div(div: &tei_core::Div) -> Result<&tei_core::Div> {
    extract_div_content(
        div,
        ContentQuery {
            index: 1,
            missing_msg: "document should contain a nested division",
            mismatch_msg: "second division content item should be a nested division",
        },
        |c| {
            if let DivContent::Div(d) = c {
                Some(d)
            } else {
                None
            }
        },
    )
}

fn nested_list(div: &tei_core::Div) -> Result<&tei_core::List> {
    extract_div_content(
        div,
        ContentQuery {
            index: 0,
            missing_msg: "nested division should contain a list",
            mismatch_msg: "nested division content should be a list",
        },
        |c| {
            if let DivContent::List(l) = c {
                Some(l)
            } else {
                None
            }
        },
    )
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
pub(super) fn i_convert_payload_to_episode_and_retitle(
    #[from(python_state)] state: &PythonModuleState,
    title: String,
) -> Result<()> {
    let payload = state.msgpack_payload()?;

    if !try_ensure_msgspec_installed() {
        let projection: PyTeiDocument =
            from_slice(&payload).context("fallback decoding MessagePack document")?;
        let document: TeiDocument = TeiDocument::try_from(projection)
            .context("projection should convert to TeiDocument")?;
        let retitled = retitle_document(&document, title.as_str())?;
        let projection_updated = PyTeiDocument::from(&retitled);
        let updated_payload =
            to_vec_named(&projection_updated).context("fallback encoding updated document")?;
        state.store_msgpack_payload(updated_payload);
        return Ok(());
    }

    Python::attach(|py| {
        state.with_module(py, |module| {
            let episode = match decode_episode(py, &module, &payload) {
                Ok(value) => value,
                Err(error) => {
                    state.store_error(error.to_string());
                    return Ok::<(), anyhow::Error>(());
                }
            };
            let header = episode.getattr("header")?;
            let file_desc = header.getattr("file_desc")?;
            file_desc.setattr("title", title)?;

            let msgpack = py.import("msgspec.msgpack")?;
            let updated_payload: Vec<u8> = msgpack
                .getattr("encode")
                .context("encode function should exist")?
                .call1((episode,))
                .and_then(|value| value.extract())
                .context("encoding Episode should succeed")?;

            state.store_msgpack_payload(updated_payload);
            Ok::<(), anyhow::Error>(())
        })
    })?;
    Ok(())
}

#[when("I decode the MessagePack payload to an Episode struct")]
pub(super) fn i_decode_the_payload_to_an_episode(
    #[from(python_state)] state: &PythonModuleState,
) -> Result<()> {
    let payload = state.msgpack_payload()?;

    if !try_ensure_msgspec_installed() {
        #[expect(
            dead_code,
            reason = "EpisodeCarrier is only used to trigger a missing-field decode when msgspec is unavailable."
        )]
        #[derive(Debug, Deserialize)]
        struct EpisodeCarrier {
            header: Value,
            text: Value,
        }

        if let Err(error) = from_slice::<EpisodeCarrier>(&payload) {
            state.store_error(error.to_string());
        }

        return Ok(());
    }

    Python::attach(|py| {
        state.with_module(py, |module| match decode_episode(py, &module, &payload) {
            Ok(_) => Ok::<(), anyhow::Error>(()),
            Err(error) => {
                state.store_error(error.to_string());
                Ok::<(), anyhow::Error>(())
            }
        })
    })?;
    Ok(())
}

#[then("the DivBlock, nested DivBlock, ListBlock, and Item are preserved")]
pub(super) fn the_div_blocks_are_preserved(
    #[from(python_state)] state: &PythonModuleState,
) -> Result<()> {
    let payload = state.msgpack_payload()?;
    if !msgspec_available() {
        let projection: PyTeiDocument =
            from_slice(&payload).context("fallback decoding MessagePack document")?;
        let document: TeiDocument = TeiDocument::try_from(projection)
            .context("projection should convert to TeiDocument")?;
        return assert_core_div_blocks(&document);
    }

    Python::attach(|py| {
        state.with_module(py, |module| {
            let episode = decode_episode(py, &module, &payload)
                .context("MessagePack payload should decode to an Episode")?;
            assert_decoded_div_blocks(&episode)
        })
    })
}
pub fn round_trips_via_episode_struct(python_state: PythonModuleState) {
    let _ = python_state;
}

/// Scenario: Round-trip a div-containing Document through the Python Episode struct.
#[scenario(path = "tests/features/python_module.feature", index = 19)]
pub fn round_trips_div_blocks_via_episode_struct(python_state: PythonModuleState) {
    let _ = python_state;
}
pub fn episode_decoding_reports_errors(python_state: PythonModuleState) {
    let _ = python_state;
}
