//! Tests for div, list, and item XML emission.

use tei_core::{DivContent, Inline};
use tei_xml::{emit_xml, parse_xml};

const DIV_WITH_PARAGRAPH: &str = concat!(
    "<TEI>",
    "<teiHeader>",
    "<fileDesc><title>Test</title></fileDesc>",
    "</teiHeader>",
    "<text>",
    "<body>",
    "<div type=\"section\">",
    "<p>Hello, world!</p>",
    "</div>",
    "</body>",
    "</text>",
    "</TEI>",
);

const DIV_WITH_LIST: &str = concat!(
    "<TEI>",
    "<teiHeader>",
    "<fileDesc><title>Test</title></fileDesc>",
    "</teiHeader>",
    "<text>",
    "<body>",
    "<div type=\"list-section\">",
    "<list>",
    "<item>First item</item>",
    "<item>Second item</item>",
    "</list>",
    "</div>",
    "</body>",
    "</text>",
    "</TEI>",
);

const ITEM_WITH_LABEL: &str = concat!(
    "<TEI>",
    "<teiHeader>",
    "<fileDesc><title>Test</title></fileDesc>",
    "</teiHeader>",
    "<text>",
    "<body>",
    "<div type=\"labeled-list\">",
    "<list>",
    "<item><label>Label:</label>Item content</item>",
    "</list>",
    "</div>",
    "</body>",
    "</text>",
    "</TEI>",
);

#[test]
fn emits_div_with_paragraph() {
    let document = parse_xml(DIV_WITH_PARAGRAPH).expect("test fixture should parse");
    let xml = emit_xml(&document).expect("document should emit");

    // Check that the XML contains the div and paragraph
    assert!(xml.contains("<div type=\"section\">"));
    assert!(xml.contains("<p>Hello, world!</p>"));
    assert!(xml.contains("</div>"));
}

#[test]
fn emits_div_with_list_and_items() {
    let document = parse_xml(DIV_WITH_LIST).expect("test fixture should parse");
    let xml = emit_xml(&document).expect("document should emit");

    // Check that the XML contains the div, list, and items
    assert!(xml.contains("<div type=\"list-section\">"));
    assert!(xml.contains("<list>"));
    assert!(xml.contains("<item>First item</item>"));
    assert!(xml.contains("<item>Second item</item>"));
    assert!(xml.contains("</list>"));
    assert!(xml.contains("</div>"));
}

#[test]
fn emits_item_with_label() {
    let document = parse_xml(ITEM_WITH_LABEL).expect("test fixture should parse");
    let xml = emit_xml(&document).expect("document should emit");

    // Check that the XML contains the label
    assert!(xml.contains("<label>Label:</label>"));
    assert!(xml.contains("<item>"));
    assert!(xml.contains("Item content"));
}

#[test]
fn round_trips_div_with_complex_content() {
    const COMPLEX_DIV: &str = concat!(
        "<TEI>",
        "<teiHeader>",
        "<fileDesc><title>Test</title></fileDesc>",
        "</teiHeader>",
        "<text>",
        "<body>",
        "<div type=\"complex\">",
        "<p>Intro paragraph</p>",
        "<list>",
        "<item n=\"1\"><label>1.</label>First</item>",
        "<item>Second</item>",
        "</list>",
        "</div>",
        "</body>",
        "</text>",
        "</TEI>",
    );

    let document = parse_xml(COMPLEX_DIV).expect("test fixture should parse");
    let xml = emit_xml(&document).expect("document should emit");

    // Parse it back
    let parsed = parse_xml(&xml).expect("emitted XML should parse");

    // Verify structure
    assert_eq!(parsed.text().body().divs().count(), 1);
    let parsed_div = parsed.text().body().divs().next().unwrap();
    assert_eq!(parsed_div.div_type(), "complex");

    // Verify content
    let content = parsed_div.content();
    assert_eq!(content.len(), 2); // Paragraph + List

    match &content[0] {
        DivContent::Paragraph(p) => {
            assert_eq!(p.content(), &[Inline::Text("Intro paragraph".into())]);
        }
        _ => panic!("expected paragraph"),
    }

    match &content[1] {
        DivContent::List(list) => {
            assert_eq!(list.items().len(), 2);
            assert_eq!(list.items()[0].n(), Some("1"));
            assert!(list.items()[0].label().is_some());
        }
        _ => panic!("expected list"),
    }
}
