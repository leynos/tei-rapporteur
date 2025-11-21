//! XML-specific unit tests covering the `PyO3` parse/emit bindings.

use super::*;
use tei_xml::emit_xml as emit_document_xml;

#[test]
fn parse_xml_builds_documents() {
    let source =
        TeiDocument::from_title_str("Wolf 359").expect("valid title should construct document");
    let xml = emit_document_xml(&source).expect("emitting XML fixture should work");
    let document = parse_xml(xml.as_str()).expect("XML payload should parse");
    assert_eq!(document.title(), "Wolf 359");
}

#[test]
fn parse_xml_rejects_invalid_payloads() {
    let Err(error) = parse_xml("<TEI><text><body/></text></TEI>") else {
        panic!("missing header should fail");
    };
    assert!(error.to_string().contains("teiHeader"));
}

#[test]
fn parse_xml_rejects_empty_payloads() {
    let Err(_) = parse_xml("") else {
        panic!("empty XML payloads must fail");
    };
}

#[test]
fn parse_xml_rejects_whitespace_payloads() {
    let Err(_) = parse_xml("   \n\t  ") else {
        panic!("whitespace-only XML payloads must fail");
    };
}

#[test]
fn parse_xml_rejects_malformed_xml() {
    let Err(_) = parse_xml("<TEI><title>Missing tags</tei>") else {
        panic!("malformed XML must fail parsing");
    };
}

#[test]
fn parse_xml_rejects_unexpected_structure() {
    let Err(_) = parse_xml("<root><nottei>Oops</nottei></root>") else {
        panic!("unexpected XML structure must fail parsing");
    };
}

#[test]
fn emit_xml_serialises_documents() {
    let document = Document::try_from_title("Wolf 359").expect("valid title should build");
    let xml = emit_xml(&document).expect("serialising TEI should succeed");
    assert!(xml.contains("<title>Wolf 359</title>"));
}

#[test]
fn emit_xml_rejects_control_characters() {
    let document = Document::try_from_title("\u{0}").expect("control chars survive validation");
    let Err(error) = emit_xml(&document) else {
        panic!("forbidden XML characters must fail emission");
    };
    assert!(error.to_string().contains("U+0000"));
}
