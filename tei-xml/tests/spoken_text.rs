//! Behaviour tests for ADR-006 spoken-text extraction from TEI XML.

use rstest::rstest;
use tei_core::SpokenTextSegment;
use tei_xml::spoken_text_segments;

fn document_with_body(body: &str) -> String {
    format!(
        concat!(
            "<TEI>",
            "<teiHeader><fileDesc><title>Spoken Fixture</title></fileDesc></teiHeader>",
            "<text><body>{}</body></text>",
            "</TEI>"
        ),
        body
    )
}

fn texts(segments: &[SpokenTextSegment]) -> Vec<&str> {
    segments.iter().map(SpokenTextSegment::text).collect()
}

#[rstest]
#[case(
    "<p>Hello <seg>there</seg>.</p>",
    vec!["Hello there."]
)]
#[case(
    "<sp><speaker>Host</speaker><p>First line.</p><p>Second line.</p></sp>",
    vec!["First line.", "Second line."]
)]
#[case(
    "<p>Hello<note>editorial aside</note>there.</p>",
    vec!["Hello there."]
)]
#[case(
    "<p>Hello <hi rend=\"italic\">very</hi> there<pause/>friend.</p>",
    vec!["Hello very there friend."]
)]
#[case(
    "<div type=\"notes\"><p>Show note.</p></div><p>Spoken.</p>",
    vec!["Spoken."]
)]
#[case(
    "<u xml:id=\"u1\">Direct utterance.</u>",
    vec!["Direct utterance."]
)]
#[case(
    "<p>こんにちは 世界</p>",
    vec!["こんにちは 世界"]
)]
fn extracts_spoken_segments_in_document_order(#[case] body: &str, #[case] expected: Vec<&str>) {
    let xml = document_with_body(body);

    let segments = spoken_text_segments(&xml).expect("spoken extraction should succeed");

    assert_eq!(texts(&segments), expected);
}

#[test]
fn utterance_with_child_spoken_blocks_emits_only_child_segments() {
    let xml = document_with_body(
        "<u xml:id=\"u1\"><p xml:id=\"p1\">Line 1.</p><p xml:id=\"p2\">Line 2.</p></u>",
    );

    let segments = spoken_text_segments(&xml).expect("spoken extraction should succeed");

    assert_eq!(texts(&segments), vec!["Line 1.", "Line 2."]);
    assert!(
        segments
            .iter()
            .all(|segment| segment.provenance().xml_id() != Some("u1"))
    );
}

#[test]
fn reports_segment_provenance() {
    let xml = document_with_body(
        "<sp xml:id=\"turn-1\"><speaker>Host</speaker><p xml:id=\"line-2\">Line.</p></sp>",
    );

    let segments = spoken_text_segments(&xml).expect("spoken extraction should succeed");

    let [segment] = segments.as_slice() else {
        panic!("expected exactly one spoken segment");
    };
    assert_eq!(segment.text(), "Line.");
    assert_eq!(segment.provenance().xml_id(), Some("line-2"));
    assert_eq!(segment.provenance().locator(), "/TEI/text/body/sp[1]/p[1]");
}

#[test]
fn rejects_malformed_xml_without_partial_estimates() {
    let xml = concat!(
        "<TEI>",
        "<teiHeader><fileDesc><title>Broken</title></fileDesc></teiHeader>",
        "<text><body><p>Unclosed",
        "</body></text></TEI>",
    );

    let error = spoken_text_segments(xml).expect_err("malformed XML should be rejected");

    assert!(error.to_string().contains("XML"));
}

#[test]
fn rejects_missing_tei_header() {
    let xml = concat!("<TEI>", "<text><body><p>Hi</p></body></text>", "</TEI>");

    let error = spoken_text_segments(xml).expect_err("missing teiHeader should be rejected");

    assert!(error.to_string().contains("teiHeader"));
}

#[test]
fn rejects_invalid_tei_header() {
    let xml = concat!(
        "<TEI>",
        "<teiHeader/>",
        "<text><body><p>Hi</p></body></text>",
        "</TEI>"
    );

    let error = spoken_text_segments(xml).expect_err("invalid teiHeader should be rejected");

    assert!(error.to_string().contains("teiHeader"));
}

#[rstest]
#[case(concat!(
    "<TEI>",
    "<teiHeader><fileDesc><title>Spoken Fixture</title></fileDesc></teiHeader>",
    "</TEI>"
))]
#[case(concat!(
    "<TEI>",
    "<teiHeader><fileDesc><title>Spoken Fixture</title></fileDesc></teiHeader>",
    "<text></text>",
    "</TEI>"
))]
fn rejects_missing_body(#[case] xml: &str) {
    let error = spoken_text_segments(xml).expect_err("missing body should be rejected");

    assert!(error.to_string().contains("body"));
}

#[test]
fn rejects_unsupported_body_element() {
    let xml = document_with_body("<unknown/>");

    let error =
        spoken_text_segments(&xml).expect_err("unsupported body elements should be rejected");

    assert!(error.to_string().contains("unsupported TEI body element"));
}

#[test]
fn resolves_entities_inside_spoken_segments() {
    let xml = document_with_body("<p>&amp; hello &#x20; &lt;there&gt;</p>");

    let segments = spoken_text_segments(&xml).expect("spoken extraction should succeed");

    assert_eq!(texts(&segments), vec!["& hello <there>"]);
}
