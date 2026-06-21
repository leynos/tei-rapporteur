//! Unit tests covering projection tagging and conversions.

use crate::{
    projection::{,
    ProjectionError, PyInline, document_to_value, py_event_from_core, value_to_document,
    },
    test_support::ensure_msgspec_installed_for_tests,
};
use pyo3::{Python, types::PyAnyMethods, types::PyModule};
use pyo3_serde::to_pyobject;
use tei_core::{
    BodyBlock, CiteData, CiteStructure, Div, EncodingDesc, Head, Inline, P, PointerList, RefsDecl,
    Span, SpanGroup, StandOff, TeiDocument, Utterance,
};
use tei_serde::{json::Value, serde_json::json};
use tei_xml::streaming::TeiEvent;

const TAG_FIELD: &str = "type";
const TAG_DOCUMENT_START: &str = "document_start";
const TAG_HEADER: &str = "header";
const TAG_PARAGRAPH: &str = "paragraph";
const TAG_UTTERANCE: &str = "utterance";
const TAG_DOCUMENT_END: &str = "document_end";

fn example_document() -> TeiDocument {
    let emphasised =
        tei_core::Hi::try_new([Inline::Text("hi".into())]).expect("inline content should validate");
    let mut paragraph = P::from_inline([Inline::Hi(emphasised)]).expect("valid paragraph");
    paragraph.set_id("p1").expect("id should validate");
    let mut body = tei_core::TeiBody::default();
    body.push_paragraph(paragraph);
    let text = tei_core::TeiText::new(body);
    let mut refs_decl = RefsDecl::new();
    let mut cite_structure = CiteStructure::new("//u[@xml:id]");
    cite_structure.add_cite_data(CiteData::new("speaker"));
    refs_decl.add_cite_structure(cite_structure);
    let header = tei_core::TeiHeader::new(
        tei_core::FileDesc::from_title_str("Bridgewater").expect("title should validate"),
    )
    .with_encoding_desc(EncodingDesc::new().with_refs_decl(refs_decl));

    let mut span = Span::new();
    span.set_target(PointerList::new(["#p1"]).expect("pointer list should validate"));
    let mut span_group =
        SpanGroup::new("citation").unwrap_or_else(|error| panic!("group kind: {error}"));
    span_group.add_span(span);
    let mut stand_off = StandOff::new();
    stand_off.add_span_group(span_group);

    TeiDocument::new(header, text).with_stand_off(stand_off)
}

fn assert_round_tripped_paragraph(original: &TeiDocument, round_tripped: &TeiDocument) {
    let original_blocks: Vec<&BodyBlock> = original.text().body().blocks().iter().collect();
    let round_trip_blocks: Vec<&BodyBlock> = round_tripped.text().body().blocks().iter().collect();
    assert_eq!(
        original_blocks.len(),
        round_trip_blocks.len(),
        "body block counts should match"
    );
    assert!(
        matches!(round_trip_blocks.first(), Some(BodyBlock::Paragraph(_))),
        "first block should remain a paragraph"
    );
    if let (Some(BodyBlock::Paragraph(orig_p)), Some(BodyBlock::Paragraph(rt_p))) =
        (original_blocks.first(), round_trip_blocks.first())
    {
        assert_eq!(
            orig_p.id(),
            rt_p.id(),
            "paragraph xml:id should survive projection"
        );
        assert_eq!(
            orig_p.content().len(),
            rt_p.content().len(),
            "inline content count should match"
        );
    }
}

#[test]
fn inline_projection_uses_type_discriminator() {
    let inline = Inline::Pause(tei_core::Pause::new());
    let value: Value =
        tei_serde::json::to_value(&PyInline::from(inline)).expect("projection should serialise");

    assert_eq!(value.get("type"), Some(&json!("pause")));
    assert!(value.get("kind").is_none(), "kind should default to None");
}

#[test]
fn document_projection_tags_inline_content() {
    let document = example_document();
    let value = document_to_value(&document).expect("projection serialises to JSON");
    let blocks = value
        .get("text")
        .and_then(|text| text.get("body"))
        .and_then(|body| body.get("blocks"))
        .and_then(Value::as_array)
        .expect("body blocks should be present");
    let paragraph = blocks.first().expect("one paragraph expected");
    assert_eq!(paragraph.get("type"), Some(&json!("paragraph")));
    let content = paragraph
        .get("content")
        .and_then(Value::as_array)
        .expect("inline content should be an array");
    let first_type = content
        .first()
        .and_then(|item| item.get("type"))
        .and_then(Value::as_str)
        .expect("inline discriminator");
    assert_eq!(first_type, "hi");
}

#[test]
fn streaming_event_projection_is_tagged() {
    let event = TeiEvent::BodyBlock(BodyBlock::Paragraph(
        P::from_text_segments(["Hello"]).expect("valid paragraph"),
    ));
    let projected = py_event_from_core(event);
    let value = tei_serde::json::to_value(&projected).expect("event should serialise");

    assert_eq!(value.get(TAG_FIELD), Some(&json!(TAG_PARAGRAPH)));
    assert!(value.get("content").is_some());
}

#[test]
fn streaming_event_discriminators_remain_aligned() {
    let start = tei_serde::json::to_value(&py_event_from_core(TeiEvent::DocumentStart))
        .expect("document_start serialises");
    assert_eq!(start.get(TAG_FIELD), Some(&json!(TAG_DOCUMENT_START)));

    let end = tei_serde::json::to_value(&py_event_from_core(TeiEvent::DocumentEnd))
        .expect("document_end serialises");
    assert_eq!(end.get(TAG_FIELD), Some(&json!(TAG_DOCUMENT_END)));

    let header_event = tei_serde::json::to_value(&py_event_from_core(TeiEvent::Header(
        tei_core::TeiHeader::new(
            tei_core::FileDesc::from_title_str("Bridgewater").expect("title should validate"),
        ),
    )))
    .expect("header serialises");
    assert_eq!(header_event.get(TAG_FIELD), Some(&json!(TAG_HEADER)));

    let mut utterance =
        Utterance::from_text_segments(Some("speaker"), ["hi"]).expect("valid utterance fixture");
    utterance.set_number("1");
    utterance.set_source(PointerList::new(["#src1"]).expect("pointer list should validate"));
    let utterance_event = tei_serde::json::to_value(&py_event_from_core(TeiEvent::BodyBlock(
        BodyBlock::Utterance(utterance),
    )))
    .expect("utterance serialises");
    assert_eq!(utterance_event.get(TAG_FIELD), Some(&json!(TAG_UTTERANCE)));
    assert_eq!(utterance_event.get("n"), Some(&json!("1")));
    assert_eq!(utterance_event.get("source"), Some(&json!(["#src1"])));
}

#[test]
fn streaming_events_decode_into_python_event_union() {
    let _import_state_lock = python_import_state_lock();
    Python::attach(|py| {
        if ensure_msgspec_installed_for_tests(py).is_err() {
            return;
        }
        let module = PyModule::new(py, "tei_rapporteur").expect("module allocation should succeed");
        crate::bindings::py_exports::tei_rapporteur(py, &module)
            .expect("module registration should succeed");
        let structs = module
            .getattr("structs")
            .expect("structs module must exist");
        let event_type = structs
            .getattr("Event")
            .expect("Event union must be exported");
        let converter = py
            .import("msgspec")
            .expect("msgspec import")
            .getattr("convert")
            .expect("msgspec.convert available");

        let events = [
            py_event_from_core(TeiEvent::DocumentStart),
            py_event_from_core(TeiEvent::Header(tei_core::TeiHeader::new(
                tei_core::FileDesc::from_title_str("Bridgewater")
                    .expect("header title should validate"),
            ))),
            py_event_from_core(TeiEvent::BodyBlock(BodyBlock::Paragraph(
                P::from_text_segments(["hello"]).expect("paragraph fixture should validate"),
            ))),
            py_event_from_core(TeiEvent::BodyBlock(BodyBlock::Utterance(
                Utterance::from_text_segments(Some("host"), ["hi"])
                    .expect("utterance fixture should validate"),
            ))),
            py_event_from_core(TeiEvent::DocumentEnd),
        ];

        for event in events {
            let py_event = to_pyobject(py, &event).expect("event projection should serialise");
            converter
                .call((py_event, event_type.clone()), None)
                .expect("msgspec conversion should succeed for all PyEvent variants");
        }
    });
}

#[test]
fn round_trip_document_to_value_and_back_preserves_core_structure() {
    let original = example_document();
    let value = document_to_value(&original).expect("projection should serialise to JSON");
    let round_tripped =
        value_to_document(&value).expect("projection JSON should round-trip into TeiDocument");

    assert_eq!(
        original.header().file_desc().title(),
        round_tripped.header().file_desc().title(),
        "header title should be preserved by document_to_value/value_to_document round-trip"
    );
    assert!(
        round_tripped.stand_off().is_some(),
        "standOff should survive projection"
    );
    let stand_off = round_tripped
        .stand_off()
        .unwrap_or_else(|| panic!("standOff should survive projection"));
    let first_group = stand_off
        .span_groups()
        .first()
        .unwrap_or_else(|| panic!("standOff should contain a span group"));
    let first_span = first_group
        .spans()
        .first()
        .unwrap_or_else(|| panic!("span group should contain a span"));
    assert_eq!(
        first_span.target().map(tei_core::PointerList::to_strings),
        Some(vec![String::from("#p1")]),
        "standOff span targets should survive projection"
    );

    let refs_decl = round_tripped
        .header()
        .encoding_desc()
        .and_then(|encoding| encoding.refs_decl())
        .unwrap_or_else(|| panic!("refsDecl should survive projection"));
    let cite_structure = refs_decl
        .cite_structures()
        .first()
        .unwrap_or_else(|| panic!("refsDecl should contain a citeStructure"));
    assert_eq!(
        cite_structure.match_expr(),
        "//u[@xml:id]",
        "citeStructure @match should survive projection"
    );
    assert_eq!(
        cite_structure
            .cite_data()
            .first()
            .map(tei_core::CiteData::property),
        Some("speaker"),
        "citeData entries should survive projection"
    );
    assert_round_tripped_paragraph(&original, &round_tripped);
}

#[test]
fn nested_division_projection_round_trips_head_and_subtype() {
    let mut child = Div::new("segment").expect("child div should validate");
    child
        .set_subtype("guest-bio")
        .expect("child subtype should validate");
    child.set_head(Head::from_text("Guest bios").expect("head should validate"));
    child.push_paragraph(P::from_text_segments(["Profile"]).expect("paragraph should validate"));

    let mut parent = Div::new("show-notes").expect("div should validate");
    parent
        .set_subtype("chapter-markers")
        .expect("subtype should validate");
    parent.set_head(Head::from_text("Chapter markers").expect("head should validate"));
    parent.push_div(child);

    let header = tei_core::TeiHeader::new(
        tei_core::FileDesc::from_title_str("Bridgewater").expect("title should validate"),
    );
    let body = tei_core::TeiBody::new([BodyBlock::Div(parent)]);
    let document = TeiDocument::new(header, tei_core::TeiText::new(body));

    let value = document_to_value(&document).expect("projection should serialise to JSON");
    assert_eq!(
        value.pointer("/text/body/blocks/0/subtype"),
        Some(&json!("chapter-markers"))
    );
    assert_eq!(
        value.pointer("/text/body/blocks/0/head/content/0/value"),
        Some(&json!("Chapter markers"))
    );
    assert_eq!(
        value.pointer("/text/body/blocks/0/content/0/subtype"),
        Some(&json!("guest-bio"))
    );

    let round_tripped =
        value_to_document(&value).expect("projection JSON should round-trip into TeiDocument");
    let Some(BodyBlock::Div(div)) = round_tripped.text().body().blocks().first() else {
        panic!("expected div body block after round-trip");
    };
    assert_eq!(div.subtype(), Some("chapter-markers"));
    assert!(div.head().is_some());
    assert!(matches!(
        div.content().first(),
        Some(tei_core::DivContent::Div(_))
    ));
}

#[test]
fn value_to_document_reports_inline_union_errors() {
    let invalid_json = json!({
        "header": {
            "file_desc": { "title": "Broken" }
        },
        "text": {
            "body": {
                "blocks": [
                    {
                        "type": "paragraph",
                        "content": [
                            "just-a-string"
                        ]
                    }
                ]
            }
        }
    });

    let result = value_to_document(&invalid_json);
    let Err(error) = result else {
        panic!("invalid inline union should fail JSON projection decoding");
    };
    assert!(
        matches!(error, ProjectionError::Serde(_)),
        "projection errors should remain distinguishable from TEI validation failures"
    );
    assert!(
        error.to_string().contains("invalid TEI projection"),
        "errors must carry the projection prefix for debugging clarity"
    );
}
