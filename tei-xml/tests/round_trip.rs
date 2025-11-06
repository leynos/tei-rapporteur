//! Integration tests covering parse/emit round trips.

use serde::Deserialize;
use tei_core::BodyBlock;
use tei_xml::{emit_xml, parse_xml};

const PRETTY_MINIMAL_TEI: &str = concat!(
    "<TEI>\n",
    "  <teiHeader>\n",
    "    <fileDesc>\n",
    "      <title>  Wolf 359  </title>\n",
    "    </fileDesc>\n",
    "  </teiHeader>\n",
    "  <text>\n",
    "    <body/>\n",
    "  </text>\n",
    "</TEI>\n",
);

const CANONICAL_MINIMAL_TEI: &str = concat!(
    "<TEI>",
    "<teiHeader>",
    "<fileDesc>",
    "<title>Wolf 359</title>",
    "</fileDesc>",
    "</teiHeader>",
    "<text>",
    "<body/>",
    "</text>",
    "</TEI>",
);

const NAMESPACED_SOURCE: &str = concat!(
    "<TEI>",
    "<teiHeader>",
    "<fileDesc>",
    "<title>Wolf 359</title>",
    "</fileDesc>",
    "</teiHeader>",
    "<text>",
    "<body>",
    "<u xml:id=\"u1\" who=\"host\">Hello</u>",
    "</body>",
    "</text>",
    "</TEI>",
);

const NAMESPACED_EXPECTED: &str = concat!(
    "<TEI>",
    "<teiHeader>",
    "<fileDesc>",
    "<title>Wolf 359</title>",
    "</fileDesc>",
    "</teiHeader>",
    "<text>",
    "<body>",
    "<u xml:id=\"u1\" who=\"host\">Hello</u>",
    "</body>",
    "</text>",
    "</TEI>",
);

#[test]
fn normalises_insignificant_whitespace_during_round_trip() {
    let document = parse_xml(PRETTY_MINIMAL_TEI).expect("pretty XML should parse");
    let emitted = emit_xml(&document).expect("parsed document should emit");

    assert_eq!(emitted, CANONICAL_MINIMAL_TEI);
}

#[test]
fn preserves_xml_id_namespace_attributes() {
    let document = parse_xml(NAMESPACED_SOURCE).expect("namespaced TEI should parse");
    match document
        .text()
        .body()
        .blocks()
        .first()
        .expect("document should have at least one block")
    {
        BodyBlock::Utterance(utterance) => {
            assert!(
                utterance.id().is_some(),
                "parsed utterance should retain xml:id",
            );
        }
        BodyBlock::Paragraph(_) => panic!("expected utterance block, found paragraph"),
    }
    let emitted = emit_xml(&document).expect("namespaced TEI should emit");

    assert_eq!(emitted, NAMESPACED_EXPECTED);
}

#[derive(Debug, Deserialize)]
struct XmlAttrCarrier {
    #[serde(rename = "@xml:id", alias = "@id")]
    identifier: String,
}

#[test]
fn quick_xml_deserializes_xml_id_attributes() {
    let carrier: XmlAttrCarrier = quick_xml::de::from_str("<node xml:id=\"n1\"/>")
        .expect("xml:id attribute should deserialize");
    assert_eq!(carrier.identifier, "n1");
}
