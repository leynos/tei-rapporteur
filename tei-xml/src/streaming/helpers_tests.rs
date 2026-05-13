//! Unit tests for streaming parser attribute-extraction helpers.
//!
//! These tests verify that `extract_utterance_attrs`, `extract_div_attrs`,
//! `extract_item_attrs`, and `extract_pause_attrs` correctly decode and
//! normalise XML attribute values and propagate errors from malformed markup.

use quick_xml::events::BytesStart;
use rstest::rstest;

use super::helpers::{extract_div_attrs, extract_item_attrs, extract_pause_attrs,
    extract_utterance_attrs};
use super::state::RawUtteranceAttrs;

/// Constructs a `BytesStart` from raw element content for test use.
///
/// `name_len` must equal the byte length of the element name.
fn start(content: &str, name_len: usize) -> BytesStart<'_> {
    BytesStart::from_content(content, name_len)
}

// ── extract_utterance_attrs ───────────────────────────────────────────────────

#[test]
fn utterance_attrs_returns_all_present_attributes() {
    let element = start(
        r##"u xml:id="u1" n="42" who="#sp1" source="#src" resp="#ed" cert="high" corresp="#u2" ana="#cat""##,
        1,
    );

    let attrs = extract_utterance_attrs(&element)
        .unwrap_or_else(|e| panic!("unexpected error: {e}"));

    assert_eq!(
        attrs,
        RawUtteranceAttrs {
            id: Some("u1".to_owned()),
            n: Some("42".to_owned()),
            who: Some("#sp1".to_owned()),
            source: Some("#src".to_owned()),
            resp: Some("#ed".to_owned()),
            cert: Some("high".to_owned()),
            corresp: Some("#u2".to_owned()),
            ana: Some("#cat".to_owned()),
        }
    );
}

#[test]
fn utterance_attrs_returns_none_for_absent_optional_attributes() {
    let element = start("u", 1);

    let attrs = extract_utterance_attrs(&element)
        .unwrap_or_else(|e| panic!("unexpected error: {e}"));

    assert_eq!(attrs, RawUtteranceAttrs::default());
}

#[test]
fn utterance_attrs_normalises_xml_entity_in_who() {
    let element = start(r#"u who="&amp;speaker""#, 1);

    let attrs = extract_utterance_attrs(&element)
        .unwrap_or_else(|e| panic!("unexpected error: {e}"));

    assert_eq!(attrs.who, Some("&speaker".to_owned()));
}

#[test]
fn utterance_attrs_normalises_whitespace_in_n() {
    let element = start("u n='alpha\tbeta'", 1);

    let attrs = extract_utterance_attrs(&element)
        .unwrap_or_else(|e| panic!("unexpected error: {e}"));

    assert_eq!(attrs.n, Some("alpha beta".to_owned()));
}

#[test]
fn utterance_attrs_returns_error_for_unknown_entity() {
    let element = start(r#"u who="&unknown;""#, 1);

    extract_utterance_attrs(&element)
        .err()
        .unwrap_or_else(|| panic!("expected error for unknown entity"));
}

// ── extract_div_attrs ─────────────────────────────────────────────────────────

#[test]
fn div_attrs_returns_all_present_attributes() {
    let element = start(r#"div type="session" subtype="morning" xml:id="d1""#, 3);

    let attrs = extract_div_attrs(&element, None)
        .unwrap_or_else(|e| panic!("unexpected error: {e}"));

    assert_eq!(attrs.div_type, "session");
    assert_eq!(attrs.subtype, Some("morning".to_owned()));
    assert_eq!(attrs.id, Some("d1".to_owned()));
    assert!(attrs.head.is_none());
}

#[test]
fn div_attrs_returns_error_when_type_is_absent() {
    let element = start(r#"div xml:id="d1""#, 3);

    let error = extract_div_attrs(&element, None)
        .err()
        .unwrap_or_else(|| panic!("expected error for missing @type"));

    assert!(
        error.to_string().contains("@type"),
        "error should mention @type; got: {error}"
    );
}

#[rstest]
#[case("div type=\"body\"")]
#[case("div type=\"front\"")]
fn div_attrs_subtype_is_optional(#[case] content: &str) {
    let element = start(content, 3);

    let attrs = extract_div_attrs(&element, None)
        .unwrap_or_else(|e| panic!("unexpected error: {e}"));

    assert_eq!(attrs.subtype, None);
    assert_eq!(attrs.id, None);
}

#[test]
fn div_attrs_normalises_entity_in_type() {
    let element = start(r#"div type="a&amp;b""#, 3);

    let attrs = extract_div_attrs(&element, None)
        .unwrap_or_else(|e| panic!("unexpected error: {e}"));

    assert_eq!(attrs.div_type, "a&b");
}

// ── extract_item_attrs ────────────────────────────────────────────────────────

#[test]
fn item_attrs_returns_all_present_attributes() {
    let element = start(r##"item xml:id="i1" n="3" corresp="#i0""##, 4);

    let attrs = extract_item_attrs(&element, None)
        .unwrap_or_else(|e| panic!("unexpected error: {e}"));

    assert_eq!(attrs.id, Some("i1".to_owned()));
    assert_eq!(attrs.n, Some("3".to_owned()));
    assert_eq!(attrs.corresp, Some("#i0".to_owned()));
    assert!(attrs.label.is_none());
}

#[test]
fn item_attrs_returns_none_for_absent_attributes() {
    let element = start("item", 4);

    let attrs = extract_item_attrs(&element, None)
        .unwrap_or_else(|e| panic!("unexpected error: {e}"));

    assert_eq!(attrs.id, None);
    assert_eq!(attrs.n, None);
    assert_eq!(attrs.corresp, None);
    assert!(attrs.label.is_none());
}

#[test]
fn item_attrs_normalises_whitespace_in_n() {
    let element = start("item n='a\tb'", 4);

    let attrs = extract_item_attrs(&element, None)
        .unwrap_or_else(|e| panic!("unexpected error: {e}"));

    assert_eq!(attrs.n, Some("a b".to_owned()));
}

// ── extract_pause_attrs ───────────────────────────────────────────────────────

#[test]
fn pause_attrs_returns_both_attributes_when_present() {
    let element = start(r#"pause dur="PT2S" type="short""#, 5);

    let (dur, pause_type) = extract_pause_attrs(&element)
        .unwrap_or_else(|e| panic!("unexpected error: {e}"));

    assert_eq!(dur, Some("PT2S".to_owned()));
    assert_eq!(pause_type, Some("short".to_owned()));
}

#[test]
fn pause_attrs_returns_none_when_attributes_absent() {
    let element = start("pause", 5);

    let (dur, pause_type) = extract_pause_attrs(&element)
        .unwrap_or_else(|e| panic!("unexpected error: {e}"));

    assert_eq!(dur, None);
    assert_eq!(pause_type, None);
}

#[test]
fn pause_attrs_returns_only_dur_when_type_absent() {
    let element = start(r#"pause dur="PT1S""#, 5);

    let (dur, pause_type) = extract_pause_attrs(&element)
        .unwrap_or_else(|e| panic!("unexpected error: {e}"));

    assert_eq!(dur, Some("PT1S".to_owned()));
    assert_eq!(pause_type, None);
}

#[rstest]
#[case(r#"pause dur="PT1S""#, Some("PT1S"), None)]
#[case(r#"pause type="long""#, None, Some("long"))]
#[case("pause", None, None)]
fn pause_attrs_parametrised(
    #[case] content: &str,
    #[case] expected_dur: Option<&str>,
    #[case] expected_type: Option<&str>,
) {
    let element = start(content, 5);

    let (dur, pause_type) = extract_pause_attrs(&element)
        .unwrap_or_else(|e| panic!("unexpected error: {e}"));

    assert_eq!(dur.as_deref(), expected_dur);
    assert_eq!(pause_type.as_deref(), expected_type);
}