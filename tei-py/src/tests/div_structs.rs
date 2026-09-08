//! Tests covering Python `msgspec.Struct` support for division body blocks.

use super::*;
use crate::test_support::{bootstrap_msgspec_attached, with_python};
use anyhow::Result;
use pyo3::{
    Bound, Python,
    types::{PyAny, PyAnyMethods, PyDict, PyModule},
};
use tei_core::{BodyBlock, Div, Head, Item, Label, List, TeiBody, TeiDocument, TeiHeader, TeiText};
use tei_serde::msgpack::to_vec_named;
use tei_xml::streaming::TeiEvent;

fn division_fixture() -> Result<TeiDocument> {
    let header = TeiHeader::new(tei_core::FileDesc::from_title_str("Bridgewater")?);
    let mut div = Div::new("show-notes")?;
    div.set_id("div1")?;
    div.set_subtype("chapter-markers")?;
    div.set_head(Head::from_text("Chapter markers")?);
    div.push_paragraph(tei_core::P::from_text_segments(["Further reading"])?);

    let mut item = Item::from_text_segments(["Transcript"])?;
    item.set_label(Label::from_text("1.")?);
    let list = List::new([item])?;
    let mut child = Div::new("segment")?;
    child.set_subtype("guest-bio")?;
    child.set_head(Head::from_text("Guest bios")?);
    child.push_list(list);
    div.push_div(child);

    let text = TeiText::new(TeiBody::new([BodyBlock::Div(div)]));
    Ok(TeiDocument::new(header, text))
}

fn text_value(any: &Bound<'_, PyAny>, attr: &str) -> Result<String> {
    Ok(any.getattr(attr)?.extract()?)
}

fn first_text_content(any: &Bound<'_, PyAny>) -> Result<String> {
    Ok(any
        .getattr("content")?
        .get_item(0)?
        .getattr("value")?
        .extract()?)
}

/// Decodes a `MessagePack` payload into the exported `Episode` struct.
fn decode_episode<'py>(
    py: Python<'py>,
    structs: &Bound<'py, PyAny>,
    payload: &[u8],
) -> Result<Bound<'py, PyAny>> {
    let episode_type = structs.getattr("Episode")?;
    let msgpack = py.import("msgspec.msgpack")?;
    let decode_kwargs = PyDict::new(py);
    decode_kwargs.set_item("type", episode_type)?;
    Ok(msgpack
        .getattr("decode")?
        .call((payload,), Some(&decode_kwargs))?)
}

/// Returns the first body block of a decoded `Episode`.
fn first_body_block<'py>(episode: &Bound<'py, PyAny>) -> Result<Bound<'py, PyAny>> {
    Ok(episode
        .getattr("text")?
        .getattr("body")?
        .getattr("blocks")?
        .get_item(0)?)
}

/// Returns the first list item's label of a nested division block.
fn nested_list_item_label<'py>(nested_div: &Bound<'py, PyAny>) -> Result<Bound<'py, PyAny>> {
    Ok(nested_div
        .getattr("content")?
        .get_item(0)?
        .getattr("items")?
        .get_item(0)?
        .getattr("label")?)
}

#[test]
fn episode_struct_decodes_div_blocks() {
    with_python(|py| {
        assert!(
            bootstrap_msgspec_attached(py),
            "msgspec bootstrap should succeed for div struct decoding"
        );
        let module = PyModule::new(py, "tei_rapporteur").expect("module allocation");
        tei_rapporteur(py, &module).expect("module registration");

        let document = division_fixture().expect("division fixture should build");
        let payload = to_vec_named(&crate::projection::PyTeiDocument::from(&document))
            .expect("MessagePack encoding should succeed");

        let structs = module.getattr("structs").expect("structs module");
        let episode = decode_episode(py, &structs, &payload).expect("Episode should decode");
        let first = first_body_block(&episode).expect("division block should exist");

        let div_block_type = structs.getattr("DivBlock").expect("DivBlock class");
        let is_div_block = first
            .is_instance(&div_block_type)
            .expect("DivBlock isinstance check should succeed");
        assert!(
            is_div_block,
            "first block should be a structs.DivBlock instance"
        );

        let div_type = text_value(&first, "div_type").expect("DivBlock should expose div_type");
        assert_eq!(div_type, "show-notes");
        let subtype = text_value(&first, "subtype").expect("DivBlock should expose subtype");
        assert_eq!(subtype, "chapter-markers");

        let head = first.getattr("head").expect("DivBlock should expose head");
        let head_text = first_text_content(&head).expect("head should expose text content");
        assert_eq!(head_text, "Chapter markers");

        let content = first
            .getattr("content")
            .expect("DivBlock should expose content");
        let nested_div = content.get_item(1).expect("nested div should exist");
        let nested_head = nested_div
            .getattr("head")
            .expect("nested DivBlock should expose head");
        let nested_head_text =
            first_text_content(&nested_head).expect("nested head should expose text content");
        assert_eq!(nested_head_text, "Guest bios");

        let label =
            nested_list_item_label(&nested_div).expect("nested list item should have label");
        let label_text = first_text_content(&label).expect("label should expose text content");
        assert_eq!(label_text, "1.");
    });
}

#[test]
fn streaming_div_events_decode_into_python_union() {
    with_python(|py| {
        assert!(
            bootstrap_msgspec_attached(py),
            "msgspec bootstrap should succeed for div event decoding"
        );
        let module = PyModule::new(py, "tei_rapporteur").expect("module allocation");
        tei_rapporteur(py, &module).expect("module registration");
        let structs = module.getattr("structs").expect("structs module");
        let event_type = structs.getattr("Event").expect("Event union");
        let converter = py
            .import("msgspec")
            .expect("msgspec import")
            .getattr("convert")
            .expect("msgspec.convert available");

        let document = division_fixture().expect("division fixture should build");
        let block = document
            .text()
            .body()
            .blocks()
            .first()
            .expect("fixture body block")
            .clone();
        let div_event = crate::projection::py_event_from_core(TeiEvent::BodyBlock(block));
        let py_event =
            pyo3_serde::to_pyobject(py, &div_event).expect("event projection should serialize");
        let decoded_event = converter
            .call((py_event, event_type), None)
            .expect("msgspec conversion should succeed");

        let div_type =
            text_value(&decoded_event, "div_type").expect("DivEvent should expose div_type");
        assert_eq!(div_type, "show-notes");
        let subtype =
            text_value(&decoded_event, "subtype").expect("DivEvent should expose subtype");
        assert_eq!(subtype, "chapter-markers");
        let head = decoded_event
            .getattr("head")
            .expect("DivEvent should expose head");
        let head_text = first_text_content(&head).expect("head should expose text content");
        assert_eq!(head_text, "Chapter markers");
    });
}
