//! Tests for div, list, and item XML emission.

use tei_core::{DivContent, Inline, PointerList};
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
    let parsed = parse_xml(&xml).expect("emitted XML should parse");

    let div = parsed
        .text()
        .body()
        .divs()
        .next()
        .expect("should have a div");
    let Some(DivContent::List(list)) = div.content().first() else {
        panic!("expected list as first div child");
    };
    let item = list.items().first().expect("list should have an item");
    let label = item.label().expect("item should have a label");
    assert_eq!(label.content(), &[Inline::Text("Label:".into())]);
    assert_eq!(item.content(), &[Inline::Text("Item content".into())]);
}

const COMPLEX_DIV: &str = concat!(
    "<TEI>",
    "<teiHeader>",
    "<fileDesc><title>Test</title></fileDesc>",
    "</teiHeader>",
    "<text>",
    "<body>",
    "<div type=\"complex\" xml:id=\"div1\">",
    "<p>Intro paragraph</p>",
    "<list xml:id=\"list1\">",
    "<item xml:id=\"item1\" n=\"1\" corresp=\"#ref1 #ref2\"><label>1.</label>First</item>",
    "<item>Second</item>",
    "</list>",
    "</div>",
    "</body>",
    "</text>",
    "</TEI>",
);

#[test]
fn round_trips_div_structure() {
    let document = parse_xml(COMPLEX_DIV).expect("test fixture should parse");
    let xml = emit_xml(&document).expect("document should emit");
    let parsed = parse_xml(&xml).expect("emitted XML should parse");

    assert_eq!(parsed.text().body().divs().count(), 1);
    let parsed_div = parsed
        .text()
        .body()
        .divs()
        .next()
        .expect("should have a div");
    assert_eq!(parsed_div.div_type(), "complex");
    assert_eq!(parsed_div.id().map(tei_core::XmlId::as_str), Some("div1"));
    assert_eq!(parsed_div.content().len(), 2);
}

#[test]
fn round_trips_div_paragraph_content() {
    let document = parse_xml(COMPLEX_DIV).expect("test fixture should parse");
    let xml = emit_xml(&document).expect("document should emit");
    let parsed = parse_xml(&xml).expect("emitted XML should parse");

    let parsed_div = parsed
        .text()
        .body()
        .divs()
        .next()
        .expect("should have a div");
    let Some(DivContent::Paragraph(p)) = parsed_div.content().first() else {
        panic!("expected paragraph as first div child");
    };
    assert_eq!(p.content(), &[Inline::Text("Intro paragraph".into())]);
}

#[test]
#[expect(
    clippy::cognitive_complexity,
    reason = "integration test validates complete round-trip behavior"
)]
fn round_trips_div_list_content() {
    let document = parse_xml(COMPLEX_DIV).expect("test fixture should parse");
    let xml = emit_xml(&document).expect("document should emit");
    let parsed = parse_xml(&xml).expect("emitted XML should parse");

    let parsed_div = parsed
        .text()
        .body()
        .divs()
        .next()
        .expect("should have a div");
    let Some(DivContent::List(list)) = parsed_div.content().get(1) else {
        panic!("expected list as second div child");
    };
    assert_eq!(list.id().map(tei_core::XmlId::as_str), Some("list1"));
    assert_eq!(list.items().len(), 2);
    let first_item = list.items().first().expect("list should have items");
    assert_eq!(first_item.id().map(tei_core::XmlId::as_str), Some("item1"));
    assert_eq!(first_item.n(), Some("1"));
    let expected_corresp =
        PointerList::parse_attribute("#ref1 #ref2").expect("test fixture should be valid");
    assert_eq!(first_item.corresp(), Some(&expected_corresp));
    let label = first_item.label().expect("first item should have a label");
    assert_eq!(label.content(), &[Inline::Text("1.".into())]);
    assert_eq!(first_item.content(), &[Inline::Text("First".into())]);
    let second_item = list.items().get(1).expect("list should have a second item");
    assert_eq!(second_item.id(), None);
    assert_eq!(second_item.n(), None);
    assert_eq!(second_item.corresp(), None);
    assert!(second_item.label().is_none());
    assert_eq!(second_item.content(), &[Inline::Text("Second".into())]);
}
