//! Unit tests covering projection tagging and conversions.

use crate::projection::{PyInline, document_to_value, py_event_from_core, value_to_document};
use tei_core::{BodyBlock, Inline, P, TeiDocument};
use tei_serde::{json::Value, serde_json::json};
use tei_xml::streaming::TeiEvent;

fn example_document() -> TeiDocument {
    let emphasised =
        tei_core::Hi::try_new([Inline::Text("hi".into())]).expect("inline content should validate");
    let mut paragraph = P::from_inline([Inline::Hi(emphasised)]).expect("valid paragraph");
    paragraph.set_id("p1").expect("id should validate");
    let mut body = tei_core::TeiBody::default();
    body.push_paragraph(paragraph);
    let text = tei_core::TeiText::new(body);
    TeiDocument::new(
        tei_core::TeiHeader::new(
            tei_core::FileDesc::from_title_str("Bridgewater").expect("title should validate"),
        ),
        text,
    )
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

    assert_eq!(value.get("type"), Some(&json!("paragraph")));
    assert!(value.get("content").is_some());
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
        panic!("invalid inline union should fail conversion");
    };
    let message = error.to_string();
    assert!(
        message.contains("invalid TEI"),
        "error should include projection prefix, got {message}"
    );
}
