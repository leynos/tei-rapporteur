//! Tests covering Python `msgspec.Struct` support for division body blocks.

use super::*;
use crate::test_support::ensure_msgspec_installed;
use pyo3::{
    Python,
    types::{PyAnyMethods, PyDict, PyModule},
};
use tei_core::{BodyBlock, Div, Head, Item, Label, List, TeiBody, TeiDocument, TeiHeader, TeiText};
use tei_serde::msgpack::to_vec_named;
use tei_xml::streaming::TeiEvent;

fn division_fixture() -> TeiDocument {
    let header = TeiHeader::new(
        tei_core::FileDesc::from_title_str("Bridgewater").expect("title should validate"),
    );
    let mut div = Div::new("show-notes").expect("div type should validate");
    div.set_id("div1").expect("id should validate");
    div.set_subtype("chapter-markers")
        .expect("subtype should validate");
    div.set_head(Head::from_text("Chapter markers").expect("head should validate"));
    div.push_paragraph(
        tei_core::P::from_text_segments(["Further reading"]).expect("paragraph should validate"),
    );

    let mut item = Item::from_text_segments(["Transcript"]).expect("item should validate");
    item.set_label(Label::from_text("1.").expect("label should validate"));
    let list = List::new([item]).expect("list should validate");
    let mut child = Div::new("segment").expect("child div type should validate");
    child
        .set_subtype("guest-bio")
        .expect("child subtype should validate");
    child.set_head(Head::from_text("Guest bios").expect("child head should validate"));
    child.push_list(list);
    div.push_div(child);

    let text = TeiText::new(TeiBody::new([BodyBlock::Div(div)]));
    TeiDocument::new(header, text)
}

fn text_value(any: &pyo3::Bound<'_, pyo3::PyAny>, attr: &str) -> String {
    any.getattr(attr)
        .unwrap_or_else(|error| panic!("{attr} should exist: {error}"))
        .extract()
        .unwrap_or_else(|error| panic!("{attr} should be a string: {error}"))
}

fn first_text_content(any: &pyo3::Bound<'_, pyo3::PyAny>) -> String {
    any.getattr("content")
        .expect("content should exist")
        .get_item(0)
        .expect("first content item should exist")
        .getattr("value")
        .expect("text inline should expose value")
        .extract()
        .expect("content value should be a string")
}

#[test]
fn episode_struct_decodes_div_blocks() {
    Python::with_gil(|py| {
        if ensure_msgspec_installed(py).is_err() {
            return;
        }

        let module = PyModule::new(py, "tei_rapporteur").expect("module allocation");
        tei_rapporteur(py, &module).expect("module registration");

        let payload = to_vec_named(&crate::projection::PyTeiDocument::from(&division_fixture()))
            .expect("MessagePack encoding should succeed");

        let structs = module.getattr("structs").expect("structs module");
        let episode_type = structs.getattr("Episode").expect("Episode class");
        let msgpack = py
            .import("msgspec.msgpack")
            .expect("msgspec.msgpack import");
        let decode_kwargs = PyDict::new(py);
        decode_kwargs
            .set_item("type", episode_type)
            .expect("kwargs population");

        let episode = msgpack
            .getattr("decode")
            .expect("decode function")
            .call((payload,), Some(&decode_kwargs))
            .expect("Episode should decode");
        let blocks = episode
            .getattr("text")
            .expect("Episode should expose text")
            .getattr("body")
            .expect("TeiText should expose body")
            .getattr("blocks")
            .expect("TeiBody should expose blocks");
        let first = blocks.get_item(0).expect("division block should exist");
        let div_block_type = structs.getattr("DivBlock").expect("DivBlock class");
        let is_div_block = first
            .is_instance(&div_block_type)
            .expect("DivBlock isinstance check should succeed");
        assert!(
            is_div_block,
            "first block should be a structs.DivBlock instance"
        );

        assert_eq!(text_value(&first, "div_type"), "show-notes");
        assert_eq!(text_value(&first, "subtype"), "chapter-markers");
        let head = first.getattr("head").expect("DivBlock should expose head");
        assert_eq!(first_text_content(&head), "Chapter markers");
        let content = first
            .getattr("content")
            .expect("DivBlock should expose content");
        let nested_div = content.get_item(1).expect("nested div should exist");
        let nested_head = nested_div
            .getattr("head")
            .expect("nested DivBlock should expose head");
        assert_eq!(first_text_content(&nested_head), "Guest bios");
        let list_block = nested_div
            .getattr("content")
            .expect("nested content")
            .get_item(0)
            .expect("list block should exist");
        let items = list_block
            .getattr("items")
            .expect("ListBlock should expose items");
        let item = items.get_item(0).expect("item should exist");
        let label = item.getattr("label").expect("Item should expose label");
        assert_eq!(first_text_content(&label), "1.");
    });
}

#[test]
fn streaming_div_events_decode_into_python_union() {
    Python::with_gil(|py| {
        if ensure_msgspec_installed(py).is_err() {
            return;
        }

        let module = PyModule::new(py, "tei_rapporteur").expect("module allocation");
        tei_rapporteur(py, &module).expect("module registration");
        let structs = module.getattr("structs").expect("structs module");
        let event_type = structs.getattr("Event").expect("Event union");
        let converter = py
            .import("msgspec")
            .expect("msgspec import")
            .getattr("convert")
            .expect("msgspec.convert available");

        let div_event = crate::projection::py_event_from_core(TeiEvent::BodyBlock(
            division_fixture()
                .text()
                .body()
                .blocks()
                .first()
                .expect("fixture body block")
                .clone(),
        ));
        let py_event =
            pyo3_serde::to_pyobject(py, &div_event).expect("event projection should serialise");
        let decoded_event = converter
            .call((py_event, event_type), None)
            .expect("msgspec conversion should succeed");

        let div_type: String = decoded_event
            .getattr("div_type")
            .expect("DivEvent should expose div_type")
            .extract()
            .expect("div_type should be a string");
        assert_eq!(div_type, "show-notes");
        let subtype: String = decoded_event
            .getattr("subtype")
            .expect("DivEvent should expose subtype")
            .extract()
            .expect("subtype should be a string");
        assert_eq!(subtype, "chapter-markers");
    });
}
