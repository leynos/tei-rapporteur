//! Regression tests for streaming parser handling of unsupported empty
//! elements.

#![cfg(feature = "streaming")]

use tei_xml::streaming::TeiPullParser;

const EMPTY_ITEM_FIXTURE: &str = concat!(
    "<TEI>",
    "<teiHeader>",
    "<fileDesc>",
    "<title>Test</title>",
    "</fileDesc>",
    "</teiHeader>",
    "<text>",
    "<body>",
    "<div type=\"segment\">",
    "<list>",
    "<item/>",
    "</list>",
    "</div>",
    "</body>",
    "</text>",
    "</TEI>",
);

#[test]
fn empty_item_in_list_is_reported_as_streaming_error() {
    let parser = TeiPullParser::from_str(EMPTY_ITEM_FIXTURE);

    let error = parser
        .into_iter()
        .find_map(Result::err)
        .expect("empty <item/> should fail the streaming parse");

    assert_eq!(
        error.to_string(),
        "XML processing error: unexpected empty element <item/> while parsing state InList"
    );
}
